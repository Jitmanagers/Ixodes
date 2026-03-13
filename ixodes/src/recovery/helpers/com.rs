use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr;
use windows_sys::Win32::Foundation::S_OK;
use windows_sys::Win32::System::Com::{
    CLSCTX, COINIT, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows_sys::Win32::System::Variant::{VARIANT, VariantClear, VariantInit};
use windows_sys::core::HRESULT;

#[link(name = "ole32")]
unsafe extern "system" {
    pub fn CoSetProxyBlanket(
        p_proxy: *mut c_void,
        dw_authn_svc: u32,
        dw_authz_svc: u32,
        p_server_princ_name: *const u16,
        dw_authn_level: u32,
        dw_imp_level: u32,
        p_auth_info: *mut c_void,
        dw_capabilities: u32,
    ) -> HRESULT;
}

#[link(name = "oleaut32")]
unsafe extern "system" {
    pub fn SysAllocString(psz: *const u16) -> *mut u16;
    pub fn SysFreeString(bstr_string: *mut u16);
}

#[allow(dead_code)]
pub struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    #[allow(dead_code)]
    pub fn new(coinit: COINIT) -> Result<Self, HRESULT> {
        let hr = unsafe { CoInitializeEx(ptr::null(), coinit as u32) };
        if hr == S_OK || hr == 0x00000001
        /* S_FALSE - already initialized */
        {
            Ok(Self { initialized: true })
        } else {
            Err(hr)
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { CoUninitialize() };
        }
    }
}

#[allow(dead_code)]
pub struct BStr {
    inner: *mut u16,
}

impl BStr {
    #[allow(dead_code)]
    pub fn new(s: &str) -> Option<Self> {
        let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
        let inner = unsafe { SysAllocString(wide.as_ptr()) };
        if inner.is_null() {
            None
        } else {
            Some(Self { inner })
        }
    }

    #[allow(dead_code)]
    pub fn as_ptr(&self) -> *mut u16 {
        self.inner
    }
}

impl Drop for BStr {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { SysFreeString(self.inner) };
        }
    }
}

#[repr(transparent)]
pub struct Variant(pub VARIANT);

impl Variant {
    pub fn new() -> Self {
        let mut v = MaybeUninit::<VARIANT>::zeroed();
        unsafe {
            VariantInit(v.as_mut_ptr());
            Self(v.assume_init())
        }
    }
}

impl Default for Variant {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe {
            let _ = VariantClear(&mut self.0);
        }
    }
}

#[allow(dead_code)]
pub unsafe fn create_instance<T>(
    r_clsid: *const windows_sys::core::GUID,
    p_unk_outer: *mut c_void,
    dw_cls_context: CLSCTX,
    r_iid: *const windows_sys::core::GUID,
) -> Result<*mut T, HRESULT> {
    let mut instance: *mut c_void = ptr::null_mut();
    let hr =
        unsafe { CoCreateInstance(r_clsid, p_unk_outer, dw_cls_context, r_iid, &mut instance) };
    if hr == S_OK {
        Ok(instance as *mut T)
    } else {
        Err(hr)
    }
}
