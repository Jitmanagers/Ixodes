use ntapi::ntexapi::{
    NtQuerySystemInformation, SYSTEM_HANDLE_INFORMATION, SystemHandleInformation,
};
use ntapi::ntmmapi::{NtMapViewOfSection, NtUnmapViewOfSection, ViewShare};
use ntapi::ntobapi::{
    NtDuplicateObject, NtQueryObject, OBJECT_TYPE_INFORMATION, ObjectTypeInformation,
};
use ntapi::winapi::ctypes::c_void;
use std::fs::File;
use std::io::Write;
use std::mem::{size_of, zeroed};
use std::path::Path;
use windows_sys::Win32::Foundation::{
    CloseHandle, INVALID_HANDLE_VALUE, MAX_PATH, STATUS_INFO_LENGTH_MISMATCH,
    STATUS_SUCCESS,
};
use windows_sys::Win32::Storage::FileSystem::{GetLogicalDriveStringsW, QueryDosDeviceW};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Memory::{PAGE_READONLY, SECTION_MAP_READ};
use windows_sys::Win32::System::ProcessStatus::GetMappedFileNameW;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, PROCESS_DUP_HANDLE, PROCESS_QUERY_LIMITED_INFORMATION,
};

fn u16_ptr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe {
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        String::from_utf16_lossy(slice)
    }
}

fn u16_arr_to_string(arr: &[u16]) -> String {
    let len = arr.iter().position(|&x| x == 0).unwrap_or(arr.len());
    String::from_utf16_lossy(&arr[..len])
}

pub mod proc {
    use super::*;

    pub fn find_by_name(name: &str) -> u32 {
        unsafe {
            let h_snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if h_snap == INVALID_HANDLE_VALUE {
                return 0;
            }

            let mut pe: PROCESSENTRY32W = zeroed();
            pe.dwSize = size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(h_snap, &mut pe) != 0 {
                loop {
                    let exe_file = u16_arr_to_string(&pe.szExeFile);
                    if exe_file.eq_ignore_ascii_case(name) {
                        let _ = CloseHandle(h_snap);
                        return pe.th32ProcessID;
                    }

                    if Process32NextW(h_snap, &mut pe) == 0 {
                        break;
                    }
                }
            }
            let _ = CloseHandle(h_snap);
            0
        }
    }
}

mod obj {
    use super::*;

    pub fn get_type(handle: *mut c_void) -> String {
        unsafe {
            let mut len: u32 = 4096;
            let mut buf: Vec<u8> = vec![0; len as usize];

            let status = NtQueryObject(
                handle,
                ObjectTypeInformation,
                buf.as_mut_ptr() as _,
                len,
                &mut len,
            );

            if status != STATUS_SUCCESS as i32 {
                return String::new();
            }

            let type_info = buf.as_ptr() as *const OBJECT_TYPE_INFORMATION;
            let type_name = (*type_info).TypeName;

            if type_name.Buffer.is_null() || type_name.Length == 0 {
                return String::new();
            }

            let slice =
                std::slice::from_raw_parts(type_name.Buffer, (type_name.Length / 2) as usize);
            String::from_utf16_lossy(slice)
        }
    }
}

mod section {
    use super::*;

    pub fn get_file_name(section_handle: *mut c_void) -> String {
        unsafe {
            let mut base: *mut c_void = null_mut();
            let mut view_size: usize = 0;

            let status = NtMapViewOfSection(
                section_handle as *mut _,
                GetCurrentProcess() as *mut _,
                &mut base as *mut _ as _,
                0,
                0,
                null_mut(),
                &mut view_size,
                ViewShare,
                0,
                PAGE_READONLY as _,
            );

            if status != STATUS_SUCCESS as i32 {
                return String::new();
            }

            let mut device_path_buf = [0u16; (MAX_PATH * 2) as usize];
            let result = GetMappedFileNameW(GetCurrentProcess() as _, base as _, device_path_buf.as_mut_ptr(), device_path_buf.len() as u32);

            NtUnmapViewOfSection(GetCurrentProcess() as *mut _, base as *mut _);

            if result == 0 {
                return String::new();
            }

            let device_path = u16_arr_to_string(&device_path_buf);
            resolve_dos_path(device_path)
        }
    }

    fn resolve_dos_path(device_path: String) -> String {
        unsafe {
            let mut drives_buf = [0u16; 512];
            let len = GetLogicalDriveStringsW(drives_buf.len() as u32, drives_buf.as_mut_ptr());
            if len > 0 {
                let mut ptr = drives_buf.as_ptr();
                while *ptr != 0 {
                    let drive_root = u16_ptr_to_string(ptr);
                    let drive_letter = drive_root.trim_end_matches('\\');

                    let mut dev_name_buf = [0u16; MAX_PATH as usize];
                    let drive_letter_u16: Vec<u16> =
                        drive_letter.encode_utf16().chain(Some(0)).collect();

                    if QueryDosDeviceW(drive_letter_u16.as_ptr(), dev_name_buf.as_mut_ptr(), dev_name_buf.len() as u32)
                        > 0
                    {
                        let dev_name = u16_arr_to_string(&dev_name_buf);

                        if device_path
                            .to_lowercase()
                            .starts_with(&dev_name.to_lowercase())
                        {
                            return format!("{}{}", drive_letter, &device_path[dev_name.len()..]);
                        }
                    }

                    let mut len = 0;
                    while *ptr.add(len) != 0 {
                        len += 1;
                    }
                    ptr = ptr.add(len + 1);
                }
            }
        }
        device_path
    }
}

use std::ptr::null_mut;

pub fn copy_locked_file(proc_name: &str, target_file: &Path, dest_path: &Path) -> bool {
    if std::fs::copy(target_file, dest_path).is_ok() {
        return true;
    }

    let search_term = target_file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    if search_term.is_empty() {
        return false;
    }

    let pid = proc::find_by_name(proc_name);
    if pid == 0 {
        return false;
    }

    unsafe {
        let h_proc = OpenProcess(
            PROCESS_DUP_HANDLE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        );

        if h_proc.is_null() || h_proc == INVALID_HANDLE_VALUE as _ {
            return false;
        }

        let mut buf_size: u32 = 1024 * 1024 * 2;
        let mut buffer: Vec<u8> = vec![0; buf_size as usize];
        let mut return_len: u32 = 0;

        let mut status = NtQuerySystemInformation(
            SystemHandleInformation,
            buffer.as_mut_ptr() as _,
            buf_size,
            &mut return_len,
        );

        if status == STATUS_INFO_LENGTH_MISMATCH as i32 {
            buf_size = return_len + 1024;
            buffer.resize(buf_size as usize, 0);
            status = NtQuerySystemInformation(
                SystemHandleInformation,
                buffer.as_mut_ptr() as _,
                buf_size,
                &mut return_len,
            );
        }

        if status != STATUS_SUCCESS as i32 {
            let _ = CloseHandle(h_proc);
            return false;
        }

        let handle_info = buffer.as_ptr() as *const SYSTEM_HANDLE_INFORMATION;
        let handle_count = (*handle_info).NumberOfHandles as usize;
        let handles_ptr = (*handle_info).Handles.as_ptr();
        let handles = std::slice::from_raw_parts(handles_ptr, handle_count);

        for handle_entry in handles {
            if handle_entry.UniqueProcessId != pid as u16 {
                continue;
            }

            let mut h_dup: *mut c_void = null_mut();
            let dup_status = NtDuplicateObject(
                h_proc as *mut _,
                handle_entry.HandleValue as *mut _,
                GetCurrentProcess() as *mut _,
                &mut h_dup as *mut _ as _,
                SECTION_MAP_READ as u32,
                0,
                0,
            );

            if dup_status != STATUS_SUCCESS as i32 {
                continue;
            }

            let type_name = obj::get_type(h_dup);
            if type_name != "Section" {
                let _ = CloseHandle(h_dup as _);
                continue;
            }

            let full_path = section::get_file_name(h_dup);
            if full_path.is_empty() {
                let _ = CloseHandle(h_dup as _);
                continue;
            }

            let found_file_name = Path::new(&full_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();

            if found_file_name != search_term {
                let _ = CloseHandle(h_dup as _);
                continue;
            }

            let mut base: *mut c_void = null_mut();
            let mut view_size: usize = 0;

            if NtMapViewOfSection(
                h_dup as *mut _,
                GetCurrentProcess() as *mut _,
                &mut base as *mut _ as _,
                0,
                0,
                null_mut(),
                &mut view_size,
                ViewShare,
                0,
                PAGE_READONLY as _,
            ) == STATUS_SUCCESS as i32
            {
                let success = match File::create(dest_path) {
                    Ok(mut file) => {
                        let data = std::slice::from_raw_parts(base as *const u8, view_size);
                        file.write_all(data).is_ok()
                    }
                    Err(_) => false,
                };

                NtUnmapViewOfSection(GetCurrentProcess() as *mut _, base as *mut _);
                let _ = CloseHandle(h_dup as _);
                let _ = CloseHandle(h_proc as _);
                return success;
            }

            let _ = CloseHandle(h_dup as _);
        }

        let _ = CloseHandle(h_proc);
    }
    false
}
