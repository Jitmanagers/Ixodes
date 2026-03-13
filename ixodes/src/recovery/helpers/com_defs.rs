use std::ffi::c_void;
use windows_sys::Win32::Foundation::VARIANT_BOOL;
use windows_sys::Win32::System::Variant::VARIANT;
use windows_sys::core::{GUID, HRESULT};

pub type BSTR = *mut u16;

#[repr(C)]
pub struct IUnknown {
    pub lp_vtbl: *const IUnknownVtbl,
}

#[repr(C)]
pub struct IUnknownVtbl {
    pub query_interface:
        unsafe extern "system" fn(*mut IUnknown, *const GUID, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "system" fn(*mut IUnknown) -> u32,
    pub release: unsafe extern "system" fn(*mut IUnknown) -> u32,
}

#[repr(C)]
#[allow(dead_code)]
pub struct IDispatch {
    pub lp_vtbl: *mut IDispatchVtbl,
}

#[repr(C)]
pub struct IDispatchVtbl {
    pub parent: IUnknownVtbl,
    pub _get_type_info_count: *const c_void,
    pub _get_type_info: *const c_void,
    pub _get_ids_of_names: *const c_void,
    pub _invoke: *const c_void,
}

pub const CLSID_TASKSCHEDULER: GUID = GUID {
    data1: 0x0F87369F,
    data2: 0xA4E5,
    data3: 0x4CFC,
    data4: [0xBD, 0x3E, 0x73, 0xE6, 0x15, 0x45, 0x72, 0xDD],
};
pub const IID_ITASKSERVICE: GUID = GUID {
    data1: 0x2FABA4C7,
    data2: 0x4DA9,
    data3: 0x4013,
    data4: [0x96, 0x97, 0x20, 0xCC, 0x3F, 0xD4, 0x0F, 0x85],
};
pub const IID_ILOGONTRIGGER: GUID = GUID {
    data1: 0x72DADE38,
    data2: 0xFAE4,
    data3: 0x4B3E,
    data4: [0xBA, 0xF4, 0x5D, 0x00, 0x9A, 0xF0, 0x2B, 0x1C],
};
pub const IID_IEXECACTION: GUID = GUID {
    data1: 0x4C3D624D,
    data2: 0xFD6B,
    data3: 0x49A3,
    data4: [0xB9, 0xB7, 0x09, 0xCB, 0x3C, 0xD3, 0xF0, 0x47],
};

pub const CLSID_WBEMLOCATOR: GUID = GUID {
    data1: 0x4590F811,
    data2: 0x1D3A,
    data3: 0x11D0,
    data4: [0x89, 0x1F, 0x00, 0xAA, 0x00, 0x4B, 0x2E, 0x24],
};
pub const IID_IWBEMLOCATOR: GUID = GUID {
    data1: 0xDC12A687,
    data2: 0x737F,
    data3: 0x11CF,
    data4: [0x88, 0x4D, 0x00, 0xAA, 0x00, 0x4B, 0x2E, 0x24],
};

pub const TASK_LOGON_NONE: i32 = 0;
pub const TASK_LOGON_INTERACTIVE_TOKEN: i32 = 3;
pub const TASK_LOGON_SERVICE_ACCOUNT: i32 = 5;
pub const TASK_RUNLEVEL_LUA: i32 = 0;
pub const TASK_RUNLEVEL_HIGHEST: i32 = 1;
pub const TASK_CREATE_OR_UPDATE: i32 = 6;
pub const TASK_ACTION_EXEC: i32 = 0;
pub const TASK_TRIGGER_LOGON: i32 = 9;
pub const TASK_COMPATIBILITY_V2: i32 = 2;
pub const WBEM_FLAG_CREATE_OR_UPDATE: i32 = 0;

#[repr(C)]
pub struct ITaskService {
    pub lp_vtbl: *const ITaskServiceVtbl,
}

#[repr(C)]
pub struct ITaskServiceVtbl {
    pub parent: IDispatchVtbl,
    pub get_folder:
        unsafe extern "system" fn(*mut ITaskService, BSTR, *mut *mut ITaskFolder) -> HRESULT,
    pub _get_running_tasks: *const c_void,
    pub new_task:
        unsafe extern "system" fn(*mut ITaskService, u32, *mut *mut ITaskDefinition) -> HRESULT,
    pub connect:
        unsafe extern "system" fn(*mut ITaskService, VARIANT, VARIANT, VARIANT, VARIANT) -> HRESULT,
    pub _get_connected: *const c_void,
    pub _get_target_server: *const c_void,
    pub _get_connected_user: *const c_void,
    pub _get_connected_domain: *const c_void,
    pub _get_highest_version: *const c_void,
}

#[repr(C)]
pub struct ITaskFolder {
    pub lp_vtbl: *const ITaskFolderVtbl,
}

#[repr(C)]
pub struct ITaskFolderVtbl {
    pub parent: IDispatchVtbl,
    pub _get_name: *const c_void,
    pub _get_path: *const c_void,
    pub _get_folder: *const c_void,
    pub _get_folders: *const c_void,
    pub _create_folder: *const c_void,
    pub _delete_folder: *const c_void,
    pub _get_task: *const c_void,
    pub _get_tasks: *const c_void,
    pub _delete_task: *const c_void,
    pub _register_task: *const c_void,
    pub register_task_definition: unsafe extern "system" fn(
        *mut ITaskFolder,
        BSTR,
        *mut ITaskDefinition,
        i32,
        VARIANT,
        VARIANT,
        i32,
        VARIANT,
        *mut *mut IRegisteredTask,
    ) -> HRESULT,
}

#[repr(C)]
pub struct ITaskDefinition {
    pub lp_vtbl: *const ITaskDefinitionVtbl,
}

#[repr(C)]
pub struct ITaskDefinitionVtbl {
    pub parent: IDispatchVtbl,
    pub get_registration_info:
        unsafe extern "system" fn(*mut ITaskDefinition, *mut *mut IRegistrationInfo) -> HRESULT,
    pub _put_registration_info: *const c_void,
    pub get_triggers:
        unsafe extern "system" fn(*mut ITaskDefinition, *mut *mut ITriggerCollection) -> HRESULT,
    pub _put_triggers: *const c_void,
    pub get_settings:
        unsafe extern "system" fn(*mut ITaskDefinition, *mut *mut ITaskSettings) -> HRESULT,
    pub _put_settings: *const c_void,
    pub _get_data: *const c_void,
    pub _put_data: *const c_void,
    pub get_principal:
        unsafe extern "system" fn(*mut ITaskDefinition, *mut *mut IPrincipal) -> HRESULT,
    pub _put_principal: *const c_void,
    pub get_actions:
        unsafe extern "system" fn(*mut ITaskDefinition, *mut *mut IActionCollection) -> HRESULT,
    pub _put_actions: *const c_void,
}

#[repr(C)]
pub struct IRegistrationInfo {
    pub lp_vtbl: *const IRegistrationInfoVtbl,
}

#[repr(C)]
pub struct IRegistrationInfoVtbl {
    pub parent: IDispatchVtbl,
    pub _get_description: *const c_void,
    pub put_description: unsafe extern "system" fn(*mut IRegistrationInfo, BSTR) -> HRESULT,
    pub _get_author: *const c_void,
    pub put_author: unsafe extern "system" fn(*mut IRegistrationInfo, BSTR) -> HRESULT,
}

#[repr(C)]
pub struct ITaskSettings {
    pub lp_vtbl: *const ITaskSettingsVtbl,
}

#[repr(C)]
pub struct ITaskSettingsVtbl {
    pub parent: IDispatchVtbl,
    pub _get_allow_demand_start: *const c_void,
    pub put_allow_demand_start:
        unsafe extern "system" fn(*mut ITaskSettings, VARIANT_BOOL) -> HRESULT,
    pub _get_auto_maintenance_open: *const c_void,
    pub _put_auto_maintenance_open: *const c_void,
    pub _get_compatibility: *const c_void,
    pub put_compatibility: unsafe extern "system" fn(*mut ITaskSettings, i32) -> HRESULT,
    pub _get_create_deskto: *const c_void,
    pub _put_create_desktop: *const c_void,
    pub _get_delete_expired_task_after: *const c_void,
    pub _put_delete_expired_task_after: *const c_void,
    pub _get_disallow_start_if_on_batteries: *const c_void,
    pub _put_disallow_start_if_on_batteries: *const c_void,
    pub _get_enabled: *const c_void,
    pub put_enabled: unsafe extern "system" fn(*mut ITaskSettings, VARIANT_BOOL) -> HRESULT,
    pub _get_hidden: *const c_void,
    pub put_hidden: unsafe extern "system" fn(*mut ITaskSettings, VARIANT_BOOL) -> HRESULT,
    pub _get_idle_settings: *const c_void,
    pub _put_idle_settings: *const c_void,
    pub _get_network_settings: *const c_void,
    pub _put_network_settings: *const c_void,
    pub _get_priority: *const c_void,
    pub _put_priority: *const c_void,
    pub _get_restart_count: *const c_void,
    pub _put_restart_count: *const c_void,
    pub _get_restart_interval: *const c_void,
    pub _put_restart_interval: *const c_void,
    pub _get_run_only_if_idle: *const c_void,
    pub _put_run_only_if_idle: *const c_void,
    pub _get_run_only_if_network_available: *const c_void,
    pub _put_run_only_if_network_available: *const c_void,
    pub _get_start_when_available: *const c_void,
    pub put_start_when_available:
        unsafe extern "system" fn(*mut ITaskSettings, VARIANT_BOOL) -> HRESULT,
}

#[repr(C)]
pub struct ITriggerCollection {
    pub lp_vtbl: *const ITriggerCollectionVtbl,
}

#[repr(C)]
pub struct ITriggerCollectionVtbl {
    pub parent: IDispatchVtbl,
    pub _get_count: *const c_void,
    pub _get_item: *const c_void,
    pub _get_new_enum: *const c_void,
    pub create:
        unsafe extern "system" fn(*mut ITriggerCollection, i32, *mut *mut ITrigger) -> HRESULT,
    pub _remove: *const c_void,
    pub _clear: *const c_void,
}

#[repr(C)]
pub struct ITrigger {
    pub lp_vtbl: *const ITriggerVtbl,
}

#[repr(C)]
pub struct ITriggerVtbl {
    pub parent: IDispatchVtbl,
    pub _get_type: *const c_void,
    pub _get_id: *const c_void,
    pub _put_id: *const c_void,
    pub _get_repetition: *const c_void,
    pub _put_repetition: *const c_void,
    pub _get_execution_time_limit: *const c_void,
    pub _put_execution_time_limit: *const c_void,
    pub _get_start_boundary: *const c_void,
    pub _put_start_boundary: *const c_void,
    pub _get_end_boundary: *const c_void,
    pub _put_end_boundary: *const c_void,
    pub _get_enabled: *const c_void,
    pub put_enabled: unsafe extern "system" fn(*mut ITrigger, VARIANT_BOOL) -> HRESULT,
}

#[repr(C)]
pub struct ILogonTrigger {
    pub lp_vtbl: *const ILogonTriggerVtbl,
}

#[repr(C)]
pub struct ILogonTriggerVtbl {
    pub parent: ITriggerVtbl,
    pub _get_delay: *const c_void,
    pub _put_delay: *const c_void,
    pub _get_user_id: *const c_void,
    pub _put_user_id: *const c_void,
}

#[repr(C)]
pub struct IActionCollection {
    pub lp_vtbl: *const IActionCollectionVtbl,
}

#[repr(C)]
pub struct IActionCollectionVtbl {
    pub parent: IDispatchVtbl,
    pub _get_count: *const c_void,
    pub _get_item: *const c_void,
    pub _get_new_enum: *const c_void,
    pub create:
        unsafe extern "system" fn(*mut IActionCollection, i32, *mut *mut IAction) -> HRESULT,
    pub _remove: *const c_void,
    pub _clear: *const c_void,
}

#[repr(C)]
pub struct IAction {
    pub lp_vtbl: *const IActionVtbl,
}

#[repr(C)]
pub struct IActionVtbl {
    pub parent: IDispatchVtbl,
    pub _get_id: *const c_void,
    pub _put_id: *const c_void,
    pub _get_type: *const c_void,
}

#[repr(C)]
pub struct IExecAction {
    pub lp_vtbl: *const IExecActionVtbl,
}

#[repr(C)]
pub struct IExecActionVtbl {
    pub parent: IActionVtbl,
    pub _get_path: *const c_void,
    pub put_path: unsafe extern "system" fn(*mut IExecAction, BSTR) -> HRESULT,
    pub _get_arguments: *const c_void,
    pub _put_arguments: *const c_void,
    pub _get_working_directory: *const c_void,
    pub _put_working_directory: *const c_void,
}

#[repr(C)]
pub struct IPrincipal {
    pub lp_vtbl: *const IPrincipalVtbl,
}

#[repr(C)]
pub struct IPrincipalVtbl {
    pub parent: IDispatchVtbl,
    pub _get_id: *const c_void,
    pub _put_id: *const c_void,
    pub _get_display_name: *const c_void,
    pub _put_display_name: *const c_void,
    pub _get_user_id: *const c_void,
    pub put_user_id: unsafe extern "system" fn(*mut IPrincipal, BSTR) -> HRESULT,
    pub _get_logon_type: *const c_void,
    pub put_logon_type: unsafe extern "system" fn(*mut IPrincipal, i32) -> HRESULT,
    pub _get_group_id: *const c_void,
    pub _put_group_id: *const c_void,
    pub _get_run_level: *const c_void,
    pub put_run_level: unsafe extern "system" fn(*mut IPrincipal, i32) -> HRESULT,
}

#[repr(C)]
pub struct IRegisteredTask {
    pub lp_vtbl: *const IRegisteredTaskVtbl,
}

#[repr(C)]
pub struct IRegisteredTaskVtbl {
    pub parent: IDispatchVtbl,
    pub _get_name: *const c_void,
    pub _get_path: *const c_void,
    pub _get_state: *const c_void,
    pub _get_enabled: *const c_void,
    pub _put_enabled: *const c_void,
    pub _run: *const c_void,
    pub _run_ex: *const c_void,
    pub _get_instances: *const c_void,
    pub _get_last_run_time: *const c_void,
    pub _get_last_task_result: *const c_void,
    pub _get_number_of_missed_runs: *const c_void,
    pub _get_next_run_time: *const c_void,
    pub _get_definition: *const c_void,
    pub _get_xml: *const c_void,
    pub _get_security_descriptor: *const c_void,
    pub _set_security_descriptor: *const c_void,
    pub _stop: *const c_void,
    pub _get_run_level: *const c_void,
}

// --- WMI Definitions ---

#[repr(C)]
pub struct IWbemLocator {
    pub lp_vtbl: *const IWbemLocatorVtbl,
}

#[repr(C)]
pub struct IWbemLocatorVtbl {
    pub parent: IUnknownVtbl,
    pub connect_server: unsafe extern "system" fn(
        *mut IWbemLocator,
        BSTR,
        BSTR,
        BSTR,
        BSTR,
        i32,
        BSTR,
        *mut c_void,
        *mut *mut IWbemServices,
    ) -> HRESULT,
}

#[repr(C)]
pub struct IWbemServices {
    pub lp_vtbl: *const IWbemServicesVtbl,
}

#[repr(C)]
pub struct IWbemServicesVtbl {
    pub parent: IUnknownVtbl,
    pub _open_namespace: *const c_void,
    pub _cancel_async_call: *const c_void,
    pub _query_object_sink: *const c_void,
    pub get_object: unsafe extern "system" fn(
        *mut IWbemServices,
        BSTR,
        i32,
        *mut c_void,
        *mut *mut IWbemClassObject,
        *mut c_void,
    ) -> HRESULT,
    pub _get_object_async: *const c_void,
    pub _put_class: *const c_void,
    pub _put_class_async: *const c_void,
    pub _delete_class: *const c_void,
    pub _delete_class_async: *const c_void,
    pub _create_class_enum: *const c_void,
    pub _create_class_enum_async: *const c_void,
    pub put_instance: unsafe extern "system" fn(
        *mut IWbemServices,
        *mut IWbemClassObject,
        i32,
        *mut c_void,
        *mut *mut c_void,
    ) -> HRESULT,
}

#[repr(C)]
pub struct IWbemClassObject {
    pub lp_vtbl: *const IWbemClassObjectVtbl,
}

#[repr(C)]
pub struct IWbemClassObjectVtbl {
    pub parent: IUnknownVtbl,
    pub _get_qualifier_set: *const c_void,
    pub _get: *const c_void,
    pub put: unsafe extern "system" fn(
        *mut IWbemClassObject,
        *const u16, // Name
        i32,
        *const VARIANT,
        i32,
    ) -> HRESULT,
    pub _delete: *const c_void,
    pub _get_names: *const c_void,
    pub _begin_enumeration: *const c_void,
    pub _next: *const c_void,
    pub _end_enumeration: *const c_void,
    pub _get_property_qualifier_set: *const c_void,
    pub _clone: *const c_void,
    pub _get_object_text: *const c_void,
    pub _spawn_derived_class: *const c_void,
    pub spawn_instance: unsafe extern "system" fn(
        *mut IWbemClassObject,
        i32,
        *mut *mut IWbemClassObject,
    ) -> HRESULT,
}
