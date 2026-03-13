use crate::recovery::task::{RecoveryArtifact, RecoveryError};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::warn;

pub async fn copy_dir_limited(
    src: &Path,
    dst: &Path,
    label: &str,
    artifacts: &mut Vec<RecoveryArtifact>,
    max_depth: usize,
    file_limit: usize,
) -> Result<(), RecoveryError> {
    let mut stack = vec![(src.to_path_buf(), dst.to_path_buf(), 0usize)];

    while let Some((current_src, current_dst, depth)) = stack.pop() {
        if reached_limit(artifacts.len(), file_limit) {
            break;
        }

        if max_depth > 0 && depth >= max_depth {
            continue;
        }

        let mut entries = match fs::read_dir(&current_src).await {
            Ok(dir) => dir,
            Err(err) => {
                warn!(path=?current_src, error=?err, "failed to read directory");
                continue;
            }
        };

        let mut dir_created = false;

        while let Some(entry) = entries.next_entry().await? {
            if reached_limit(artifacts.len(), file_limit) {
                break;
            }

            let path = entry.path();
            let file_name = entry.file_name();
            let target = current_dst.join(&file_name);

            let metadata = match entry.metadata().await {
                Ok(metadata) => metadata,
                Err(err) => {
                    warn!(path=?path, error=?err, "failed to read metadata");
                    continue;
                }
            };

            if metadata.is_dir() {
                stack.push((path, target, depth + 1));
            } else if metadata.is_file() {
                if !dir_created {
                    fs::create_dir_all(&current_dst).await?;
                    dir_created = true;
                }

                if let Err(err) = fs::copy(&path, &target).await {
                    warn!(src=?path, dst=?target, error=?err, "failed to copy file");
                    continue;
                }
                let copied_meta = fs::metadata(&target).await?;
                artifacts.push(RecoveryArtifact {
                    label: label.to_string(),
                    path: target,
                    size_bytes: copied_meta.len(),
                    modified: copied_meta.modified().ok(),
                });
            }
        }
    }

    Ok(())
}

pub async fn copy_file(
    label: &str,
    src: &Path,
    dst_root: &Path,
    artifacts: &mut Vec<RecoveryArtifact>,
) -> Result<(), RecoveryError> {
    if !src.exists() {
        return Ok(());
    }

    let file_name = src
        .file_name()
        .ok_or_else(|| RecoveryError::Custom("invalid source filename".into()))?
        .to_string_lossy();

    fs::create_dir_all(dst_root).await?;
    let destination = resolve_unique_destination(dst_root, &file_name).await?;

    fs::copy(src, &destination).await?;

    let meta = fs::metadata(&destination).await?;
    artifacts.push(RecoveryArtifact {
        label: label.to_string(),
        path: destination,
        size_bytes: meta.len(),
        modified: meta.modified().ok(),
    });

    Ok(())
}

pub async fn copy_file_with_structure(
    label: &str,
    src: &Path,
    base_root: &Path,
    dst_root: &Path,
    artifacts: &mut Vec<RecoveryArtifact>,
) -> Result<(), RecoveryError> {
    if !src.exists() {
        return Ok(());
    }

    let relative = src.strip_prefix(base_root).unwrap_or(src);
    let destination = dst_root.join(relative);

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::copy(src, &destination).await?;

    let meta = fs::metadata(&destination).await?;
    artifacts.push(RecoveryArtifact {
        label: label.to_string(),
        path: destination,
        size_bytes: meta.len(),
        modified: meta.modified().ok(),
    });

    Ok(())
}

pub async fn resolve_unique_destination(
    dest_root: &Path,
    file_name: &str,
) -> Result<PathBuf, RecoveryError> {
    let mut candidate = dest_root.join(file_name);
    let mut counter = 0;
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(file_name);
    let extension = Path::new(file_name)
        .extension()
        .and_then(OsStr::to_str)
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default();

    while fs::metadata(&candidate).await.is_ok() {
        counter += 1;
        let suffix = format!("{stem}_{counter}{extension}");
        candidate = dest_root.join(suffix);
    }

    Ok(candidate)
}

pub async fn copy_named_dir(
    label: &str,
    src: &Path,
    dst: &Path,
    artifacts: &mut Vec<RecoveryArtifact>,
) -> Result<(), RecoveryError> {
    match fs::metadata(src).await {
        Ok(meta) if meta.is_dir() => {
            copy_dir_limited(src, dst, label, artifacts, usize::MAX, 0).await?;
        }
        _ => {}
    }
    Ok(())
}

pub async fn copy_dir_filtered<F>(
    src: &Path,
    dst: &Path,
    label: &str,
    artifacts: &mut Vec<RecoveryArtifact>,
    max_depth: usize,
    file_limit: usize,
    filter: F,
) -> Result<(), RecoveryError>
where
    F: Fn(&str) -> bool,
{
    let mut stack = vec![(src.to_path_buf(), dst.to_path_buf(), 0usize)];

    while let Some((current_src, current_dst, depth)) = stack.pop() {
        if reached_limit(artifacts.len(), file_limit) {
            break;
        }

        if max_depth > 0 && depth >= max_depth {
            continue;
        }

        if !current_src.exists() {
            continue;
        }

        let mut entries = match fs::read_dir(&current_src).await {
            Ok(dir) => dir,
            Err(err) => {
                warn!(path=?current_src, error=?err, "failed to read directory");
                continue;
            }
        };

        let mut dir_created = false;

        while let Some(entry) = match entries.next_entry().await {
            Ok(Some(entry)) => Some(entry),
            _ => None,
        } {
            if reached_limit(artifacts.len(), file_limit) {
                break;
            }

            let path = entry.path();
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            let target = current_dst.join(&file_name);

            if !filter(&name_str) {
                continue;
            }

            let metadata = match entry.metadata().await {
                Ok(metadata) => metadata,
                Err(err) => {
                    warn!(path=?path, error=?err, "failed to read metadata");
                    continue;
                }
            };

            if metadata.is_dir() {
                stack.push((path, target, depth + 1));
            } else if metadata.is_file() {
                if !dir_created {
                    fs::create_dir_all(&current_dst).await?;
                    dir_created = true;
                }

                if let Err(err) = fs::copy(&path, &target).await {
                    warn!(src=?path, dst=?target, error=?err, "failed to copy file");
                    continue;
                }

                artifacts.push(RecoveryArtifact {
                    label: label.to_string(),
                    path: target,
                    size_bytes: metadata.len(),
                    modified: metadata.modified().ok(),
                });
            }
        }
    }

    Ok(())
}

fn reached_limit(current: usize, limit: usize) -> bool {
    limit > 0 && current >= limit
}

pub fn sanitize_label(label: &str) -> String {
    let filtered: String = label
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect();

    filtered.trim_matches('.').trim().to_string()
}
