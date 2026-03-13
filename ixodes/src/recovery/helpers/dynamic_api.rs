use std::ffi::c_void;
use crate::recovery::helpers::pe::{IMAGE_DOS_HEADER, IMAGE_NT_HEADERS64};

#[repr(C)]
pub struct UNICODE_STRING {
    pub length: u16,
    pub maximum_length: u16,
    pub buffer: *mut u16,
}

#[repr(C)]
pub struct LIST_ENTRY {
    pub flink: *mut LIST_ENTRY,
    pub blink: *mut LIST_ENTRY,
}

#[repr(C)]
pub struct PEB_LDR_DATA {
    pub length: u32,
    pub initialized: u8,
    pub ss_handle: *mut c_void,
    pub in_load_order_module_list: LIST_ENTRY,
    pub in_memory_order_module_list: LIST_ENTRY,
    pub in_initialization_order_module_list: LIST_ENTRY,
}

#[repr(C)]
pub struct LDR_DATA_TABLE_ENTRY {
    pub in_load_order_links: LIST_ENTRY,
    pub in_memory_order_links: LIST_ENTRY,
    pub in_initialization_order_links: LIST_ENTRY,
    pub dll_base: *mut c_void,
    pub entry_point: *mut c_void,
    pub size_of_image: u32,
    pub full_dll_name: UNICODE_STRING,
    pub base_dll_name: UNICODE_STRING,
}

#[repr(C)]
pub struct PEB {
    pub reserved1: [u8; 2],
    pub being_debugged: u8,
    pub reserved2: [u8; 1],
    pub reserved3: [*mut c_void; 2],
    pub ldr: *mut PEB_LDR_DATA,
}

pub const fn djb2_hash(s: &str) -> u32 {
    let mut hash: u32 = 5381;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        // For function names, we might want case-sensitive, but let's stay consistent
        hash = (hash << 5).wrapping_add(hash).wrapping_add(c as u32);
        i += 1;
    }
    hash
}

pub unsafe fn djb2_hash_wide(ptr: *const u16, len: usize) -> u32 {
    unsafe {
        let mut hash: u32 = 5381;
        for i in 0..len {
            let mut c = *ptr.add(i);
            // Case-insensitive for module names
            if c >= b'A' as u16 && c <= b'Z' as u16 {
                c += b'a' as u16 - b'A' as u16 ;
            }
            hash = (hash << 5).wrapping_add(hash).wrapping_add(c as u32);
        }
        hash
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn get_peb() -> *mut PEB {
    unsafe {
        let peb: *mut PEB;
        std::arch::asm!(
            "mov {0}, gs:[0x60]",
            out(reg) peb
        );
        peb
    }
}

pub unsafe fn get_module_base(module_hash: u32) -> *mut c_void {
    unsafe {
        let peb = get_peb();
        let ldr = (*peb).ldr;
        let mut current_entry = (*ldr).in_load_order_module_list.flink;
        let list_head = &(*ldr).in_load_order_module_list as *const LIST_ENTRY;

        while current_entry != list_head as *mut LIST_ENTRY {
            let table_entry = current_entry as *mut LDR_DATA_TABLE_ENTRY;
            let base_name = &(*table_entry).base_dll_name;
            
            let hash = djb2_hash_wide(base_name.buffer, (base_name.length / 2) as usize);
            if hash == module_hash {
                return (*table_entry).dll_base;
            }

            current_entry = (*current_entry).flink;
        }

        std::ptr::null_mut()
    }
}

#[repr(C)]
struct IMAGE_EXPORT_DIRECTORY {
    pub characteristics: u32,
    pub time_date_stamp: u32,
    pub major_version: u16,
    pub minor_version: u16,
    pub name: u32,
    pub base: u32,
    pub number_of_functions: u32,
    pub number_of_names: u32,
    pub address_of_functions: u32,
    pub address_of_names: u32,
    pub address_of_name_ordinals: u32,
}

pub unsafe fn get_proc_address(module_base: *mut c_void, func_hash: u32) -> *mut c_void {
    unsafe {
        if module_base.is_null() {
            return std::ptr::null_mut();
        }

        let dos_header = &*(module_base as *const IMAGE_DOS_HEADER);
        let nt_headers = &*(module_base.add(dos_header.e_lfanew as usize) as *const IMAGE_NT_HEADERS64);
        
        let export_dir_rva = nt_headers.optional_header.data_directory[0].virtual_address;
        if export_dir_rva == 0 {
            return std::ptr::null_mut();
        }

        let export_dir = &*(module_base.add(export_dir_rva as usize) as *const IMAGE_EXPORT_DIRECTORY);
        
        let names = module_base.add(export_dir.address_of_names as usize) as *const u32;
        let ordinals = module_base.add(export_dir.address_of_name_ordinals as usize) as *const u16;
        let functions = module_base.add(export_dir.address_of_functions as usize) as *const u32;

        for i in 0..export_dir.number_of_names {
            let name_rva = *names.add(i as usize);
            let name_ptr = module_base.add(name_rva as usize) as *const i8;
            
            // Calculate hash of the name
            let mut hash: u32 = 5381;
            let mut j = 0;
            while *name_ptr.add(j) != 0 {
                hash = (hash << 5).wrapping_add(hash).wrapping_add(*name_ptr.add(j) as u8 as u32);
                j += 1;
            }

            if hash == func_hash {
                let ordinal = *ordinals.add(i as usize);
                let func_rva = *functions.add(ordinal as usize);
                return module_base.add(func_rva as usize);
            }
        }

        std::ptr::null_mut()
    }
}

// Common hashes for ntdll.dll and kernel32.dll
pub const NTDLL_HASH: u32 = djb2_hash("ntdll.dll");
pub const KERNEL32_HASH: u32 = djb2_hash("kernel32.dll");
#[allow(dead_code)]
pub const ADVAPI32_HASH: u32 = djb2_hash("advapi32.dll");
#[allow(dead_code)]
pub const USER32_HASH: u32 = djb2_hash("user32.dll");
#[allow(dead_code)]
pub const SHELL32_HASH: u32 = djb2_hash("shell32.dll");
pub const WINHTTP_HASH: u32 = djb2_hash("winhttp.dll");
pub const WINSQLITE3_HASH: u32 = djb2_hash("winsqlite3.dll");
#[allow(dead_code)]
pub const CRYPT32_HASH: u32 = djb2_hash("crypt32.dll");

#[macro_export]
macro_rules! dynamic_invoke {
    ($module_hash:expr, $func_hash:expr, $type:ty, $($arg:expr),*) => {
        {
            let module_base = $crate::recovery::helpers::dynamic_api::get_module_base($module_hash);
            let func_ptr = $crate::recovery::helpers::dynamic_api::get_proc_address(module_base, $func_hash);
            if !func_ptr.is_null() {
                let func: $type = std::mem::transmute(func_ptr);
                Some(func($($arg),*))
            } else {
                None
            }
        }
    };
}

pub unsafe fn load_library(module_name: &str) -> *mut c_void {
    unsafe {
        let kernel32_base = get_module_base(KERNEL32_HASH);
        if kernel32_base.is_null() {
            return std::ptr::null_mut();
        }

        let load_library_w_ptr = get_proc_address(kernel32_base, djb2_hash("LoadLibraryW"));
        if load_library_w_ptr.is_null() {
            return std::ptr::null_mut();
        }

        type FnLoadLibraryW = unsafe extern "system" fn(lp_lib_file_name: *const u16) -> *mut c_void;
        let load_library_w: FnLoadLibraryW = std::mem::transmute(load_library_w_ptr);

        let module_name_w: Vec<u16> = module_name.encode_utf16().chain(std::iter::once(0)).collect();
        load_library_w(module_name_w.as_ptr())
    }
}
