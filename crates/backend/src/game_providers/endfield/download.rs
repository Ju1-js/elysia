#![allow(clippy::non_std_lazy_statics)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::fs;

use anyhow::{Context, Result, anyhow};
use lazy_static::lazy_static;
use reqwest::Client;
use reqwest::header::RANGE;
use serde::{Deserialize, Serialize};
use stream_unpack::zip::read_cd;
use stream_unpack::zip::structures::central_directory::CentralDirectory;
use stream_unpack::zip::{ZipDecodedData, ZipPosition, ZipUnpacker};

use crate::game_providers::endfield::api;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
    pub mb_s: f32,
    pub part_index: usize,
    pub parts_total: usize,
    pub status: String,
    pub is_busy: bool,
}

lazy_static! {
    static ref PROGRESS_MAP: Mutex<HashMap<String, Progress>> = Mutex::new(HashMap::new());
}

#[must_use] 
pub fn get_progress(key: &str) -> Option<Progress> {
    PROGRESS_MAP
        .lock()
        .ok()
        .and_then(|map| map.get(key).cloned())
}

pub fn set_progress(key: &str, progress: Progress) {
    if let Ok(mut map) = PROGRESS_MAP.lock() {
        map.insert(key.to_string(), progress);
    }
}

pub fn clear_progress(key: &str) {
    if let Ok(mut map) = PROGRESS_MAP.lock() {
        map.remove(key);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Pack {
    url: String,
    md5: String,
    size: u64,
}

struct StreamArchive {
    packs: Vec<Pack>,
    output_dir: PathBuf,
    progress_key: String,
    client: Client,
}

impl StreamArchive {
    async fn from_packs(
        packs: Vec<api::Pack>,
        output_dir: &Path,
        progress_key: &str,
    ) -> Result<Self> {
        let packs: Vec<Pack> = packs
            .into_iter()
            .map(|p| {
                let size_str = p.package_size.unwrap_or_else(|| "0".to_string());
                let size = size_str.parse::<u64>().unwrap_or(0);
                Pack {
                    url: p.url,
                    md5: p.md5.unwrap_or_default(),
                    size,
                }
            })
            .collect();

        fs::create_dir_all(output_dir).await?;

        Ok(Self {
            packs,
            output_dir: output_dir.to_path_buf(),
            progress_key: progress_key.to_string(),
            client: Client::new(),
        })
    }

    fn total_size(&self) -> u64 {
        self.packs.iter().map(|p| p.size).sum()
    }

    fn part_sizes(&self) -> Vec<usize> {
        self.packs
            .iter()
            .map(|p| {
                p.size.try_into().unwrap_or_else(|_| {
                    eprintln!("[WARN] Pack size {} exceeds usize::MAX, clamping to usize::MAX", p.size);
                    usize::MAX
                })
            })
            .collect()
    }

    async fn verify_last_part_size(&self, mut sizes: Vec<usize>) -> Result<Vec<usize>> {
        let last_idx = sizes.len() - 1;
        let last_url = &self.packs[last_idx].url;

        let head_resp = self
            .client
            .head(last_url)
            .send()
            .await
            .context("failed to send HEAD request for last part")?;

        if let Some(content_length) = head_resp.headers().get("content-length") {
            let content_length_str = content_length
                .to_str()
                .context("invalid content-length header")?;
            let actual_size = content_length_str
                .parse::<usize>()
                .context("failed to parse content-length")?;

            if actual_size != sizes[last_idx] {
                eprintln!(
                    "[WARN] Last part size mismatch: API reported {} bytes, server has {} bytes",
                    sizes[last_idx], actual_size
                );
                sizes[last_idx] = actual_size;
            }
        }

        Ok(sizes)
    }

    fn create_range_provider(&self) -> impl Fn(ZipPosition, usize) -> Result<Vec<u8>> + '_ {
        move |pos: ZipPosition, len: usize| -> Result<Vec<u8>> {
            let disk = pos.disk;
            if disk >= self.packs.len() {
                return Err(anyhow!("invalid disk index: {disk}"));
            }

            let url = &self.packs[disk].url;
            let range = format!("bytes={}-{}", pos.offset, pos.offset + len - 1);

            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let resp = self
                        .client
                        .get(url)
                        .header(RANGE, &range)
                        .send()
                        .await
                        .with_context(|| {
                            format!("requesting range from part {disk} ({url})")
                        })?;

                    if !resp.status().is_success() {
                        return Err(anyhow!("HTTP error: {}", resp.status()));
                    }

                    Ok(resp.bytes().await?.to_vec())
                })
            })
        }
    }

    async fn read_central_directory(&self) -> Result<CentralDirectory> {
        let sizes = self.part_sizes();
        let verified_sizes = self.verify_last_part_size(sizes).await?;

        let provider = self.create_range_provider();
        let cd = read_cd::from_provider(verified_sizes, true, provider)
            .context("failed to read central directory from cut ZIP")?;

        Ok(cd)
    }

    async fn load_resume_position(&self) -> Result<Option<ZipPosition>> {
        let status_file_path = self.output_dir.join(".elysia_extract_status");

        if !status_file_path.exists() {
            return Ok(None);
        }

        let data = tokio::fs::read(&status_file_path)
            .await
            .context("failed to read resume status file")?;

        if data.len() < 12 {
            eprintln!("[WARN] Invalid resume status file (too small), ignoring");
            return Ok(None);
        }

        let disk_u32 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let offset_u64 = u64::from_le_bytes([
            data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
        ]);

        let disk = disk_u32.try_into().unwrap_or_else(|_| {
            eprintln!("[WARN] Disk index {disk_u32} exceeds usize::MAX, clamping");
            usize::MAX
        });
        let offset = offset_u64.try_into().unwrap_or_else(|_| {
            eprintln!("[WARN] Offset {offset_u64} exceeds usize::MAX, clamping");
            usize::MAX
        });

        Ok(Some(ZipPosition::new(disk, offset)))
    }

    fn setup_extraction_callback(
        unpacker: &mut ZipUnpacker,
        output_dir: PathBuf,
        status_file_path: PathBuf,
    ) {
        let current_file = Arc::new(Mutex::new(None::<std::fs::File>));
        let out_dir = Arc::new(output_dir);
        let status_path = Arc::new(status_file_path);

        let current_file_clone = current_file.clone();
        let out_dir_clone = out_dir.clone();
        let status_path_clone = status_path.clone();

        unpacker.set_callback(move |data| -> Result<()> {
            match data {
                ZipDecodedData::FileHeader(cdfh, _lfh) => {
                    tokio::task::block_in_place(|| -> Result<()> {
                        let mut file_to_flush = current_file_clone
                            .lock()
                            .map_err(|e| anyhow!("mutex poisoned: {e}"))?
                            .take();

                        if let Some(ref mut f) = file_to_flush {
                            std::io::Write::flush(f).context("failed to flush file")?;
                        }

                        let name = &cdfh.filename;
                        let path = out_dir_clone.join(name);

                        if name.ends_with('/') || name.ends_with('\\') {
                            std::fs::create_dir_all(&path).with_context(|| {
                                format!("failed to create directory: {}", path.display())
                            })?;
                            return Ok(());
                        }

                        if let Some(parent) = path.parent() {
                            std::fs::create_dir_all(parent).with_context(|| {
                                format!("failed to create parent directory: {}", parent.display())
                            })?;
                        }

                        let new_file = std::fs::File::create(&path)
                            .with_context(|| format!("failed to create file: {}", path.display()))?;

                        *current_file_clone
                            .lock()
                            .map_err(|e| anyhow!("mutex poisoned: {e}"))? = Some(new_file);

                        let pos = cdfh.header_position();
                        let mut state = Vec::with_capacity(12);
                        
                        let disk_u32: u32 = pos.disk.try_into().unwrap_or_else(|_| {
                            eprintln!("[WARN] Disk index {} exceeds u32::MAX, clamping", pos.disk);
                            u32::MAX
                        });
                        let offset_u64: u64 = pos.offset.try_into().unwrap_or_else(|_| {
                            eprintln!("[WARN] Offset {} exceeds u64::MAX, clamping", pos.offset);
                            u64::MAX
                        });
                        
                        state.extend_from_slice(&disk_u32.to_le_bytes());
                        state.extend_from_slice(&offset_u64.to_le_bytes());
                        let _ = std::fs::write(&*status_path_clone, state);

                        Ok(())
                    })?;
                }
                ZipDecodedData::FileData(data) => {
                    tokio::task::block_in_place(|| -> Result<()> {
                        let mut guard = current_file_clone
                            .lock()
                            .map_err(|e| anyhow!("mutex poisoned: {e}"))?;

                        if let Some(f) = guard.as_mut() {
                            std::io::Write::write_all(f, data)
                                .context("failed to write file data")?;
                        }
                        Ok(())
                    })?;
                }
            }
            Ok(())
        });
    }

    async fn verify_md5_async(
        context: md5::Context,
        expected: String,
        part_idx: usize,
    ) -> Result<()> {
        if expected.is_empty() {
            return Ok(());
        }

        tokio::task::spawn_blocking(move || {
            let digest = context.finalize();
            let actual = format!("{digest:x}");
            if actual.eq_ignore_ascii_case(&expected) {
                eprintln!("[MD5] Part {} verified successfully", part_idx + 1);
                Ok(())
            } else {
                Err(anyhow!(
                    "MD5 verification failed for part {}: expected {}, got {}",
                    part_idx + 1,
                    expected,
                    actual
                ))
            }
        })
        .await?
    }

    fn calculate_resume_point(&self, virtual_pos: ZipPosition) -> (usize, usize, u64) {
        let mut remaining = virtual_pos.offset;
        let mut part_idx = 0;
        let mut total_downloaded = 0u64;

        for (idx, &size) in self.part_sizes().iter().enumerate() {
            if remaining < size {
                part_idx = idx;
                break;
            }
            remaining -= size;
            total_downloaded += size as u64;
        }

        total_downloaded += remaining as u64;

        (part_idx, remaining, total_downloaded)
    }

    fn process_buffer(unpacker: &mut ZipUnpacker, buffer: &mut Vec<u8>) -> Result<bool> {
        loop {
            let (consumed, is_done) = unpacker
                .update(&*buffer)
                .map_err(|e| anyhow!("unpacker error: {e:?}"))?;

            if consumed > 0 {
                buffer.drain(..consumed);
            }

            if is_done {
                return Ok(true);
            }

            if consumed == 0 {
                return Ok(false);
            }
        }
    }

    fn update_progress(&self, current: u64, mb_s: f32, part_index: usize, status: String) {
        set_progress(
            &self.progress_key,
            Progress {
                downloaded: current,
                total: self.total_size(),
                mb_s,
                part_index,
                parts_total: self.packs.len(),
                status,
                is_busy: true,
            },
        );
    }

    #[allow(clippy::too_many_lines)]
    pub async fn stream_unpack(&self) -> Result<()> {
        let total = self.total_size();

        self.update_progress(0, 0.0, 0, "Preparing download..".to_string());

        let cd = self.read_central_directory().await?;
        let sorted = cd.sort();

        let part_sizes = self.part_sizes();
        let virtual_sizes = vec![part_sizes.iter().sum::<usize>()];
        let resume_pos = self.load_resume_position().await?;

        let mut unpacker = if let Some(pos) = resume_pos {
            ZipUnpacker::resume(sorted, virtual_sizes, pos)?
        } else {
            ZipUnpacker::new(sorted, virtual_sizes)
        };

        let status_file_path = self.output_dir.join(".elysia_extract_status");
        Self::setup_extraction_callback(
            &mut unpacker,
            self.output_dir.clone(),
            status_file_path.clone(),
        );

        let (start_part, start_offset, mut total_downloaded) = if let Some(pos) = resume_pos {
            let (part, offset, downloaded) = self.calculate_resume_point(pos);
            self.update_progress(
                downloaded,
                0.0,
                part,
                format!("Resuming from part {}/{}", part + 1, self.packs.len()),
            );
            (part, offset, downloaded)
        } else {
            (0, 0, 0u64)
        };

        let mut buffer = Vec::with_capacity(65536 + 4096);
        let mut last_update = Instant::now();
        let mut last_bytes = total_downloaded;
        let mut md5_context = md5::Context::new();
        let mut verification_tasks = Vec::new();

        for (part_idx, pack) in self.packs.iter().enumerate().skip(start_part) {
            let mut request = self.client.get(&pack.url);

            if part_idx == start_part && start_offset > 0 {
                request = request.header(RANGE, format!("bytes={start_offset}-"));
            }

            let resp = request
                .send()
                .await
                .context("failed to send download request")?;

            if !resp.status().is_success() {
                return Err(anyhow!("download failed with status: {}", resp.status()));
            }

            let mut resp = resp;

            loop {
                let chunk = resp.chunk().await.context("failed to read chunk")?;

                let Some(chunk) = chunk else {
                    break;
                };

                total_downloaded += chunk.len() as u64;
                buffer.extend_from_slice(&chunk);
                md5_context.consume(&chunk);

                let now = Instant::now();
                if now.duration_since(last_update) >= Duration::from_secs(1) {
                    let diff = total_downloaded - last_bytes;
                    #[allow(clippy::cast_precision_loss)]
                    let mb_s = diff as f32 / (1024.0 * 1024.0);
                    last_bytes = total_downloaded;
                    last_update = now;

                    self.update_progress(
                        total_downloaded,
                        mb_s,
                        part_idx + 1,
                        "Downloading..".to_string(),
                    );
                }

                if buffer.len() >= 65536 && Self::process_buffer(&mut unpacker, &mut buffer)? {
                    let _ = tokio::fs::remove_file(&status_file_path).await;
                    self.update_progress(total, 0.0, 0, "Complete".to_string());
                    return Ok(());
                }
            }

            if !pack.md5.is_empty() {
                let is_partial_resume = part_idx == start_part && start_offset > 0;

                if is_partial_resume {
                    eprintln!(
                        "[MD5] Skipping verification for part {} (resumed mid-part)",
                        part_idx + 1
                    );
                } else {
                    eprintln!(
                        "[MD5] Spawning verification task for part {}...",
                        part_idx + 1
                    );
                    let context = md5_context.clone();
                    let expected = pack.md5.clone();
                    let task = tokio::spawn(async move {
                        Self::verify_md5_async(context, expected, part_idx).await
                    });
                    verification_tasks.push((part_idx, task));
                }
            }
            md5_context = md5::Context::new();
        }

        if !verification_tasks.is_empty() {
            eprintln!(
                "[MD5] Waiting for {} verification tasks to complete...",
                verification_tasks.len()
            );
        }

        for (part_idx, task) in verification_tasks {
            match task.await.context("verification task panicked")? {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("[MD5] Part {} verification failed: {}", part_idx + 1, e);
                    return Err(e);
                }
            }
        }

        if !buffer.is_empty() {
            let _ = Self::process_buffer(&mut unpacker, &mut buffer)?;
        }

        let _ = tokio::fs::remove_file(&status_file_path).await;
        self.update_progress(total, 0.0, 0, "Complete".to_string());
        Ok(())
    }
}

pub async fn download_and_extract_streaming(
    packs: Vec<api::Pack>,
    dest: &Path,
    progress_key: &str,
    game_id: &str,
) -> Result<(), String> {
    let archive = StreamArchive::from_packs(packs, dest, progress_key)
        .await
        .map_err(|e| format!("initialization error: {e}"))?;

    let res = archive.stream_unpack().await;

    if res.is_ok() {
        use crate::game_providers::installer::InstallationManifest;

        let manifest = InstallationManifest {
            game_id: game_id.to_string(),
        };

        let marker_path = dest.join(".elysia_installed");
        let json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("Failed to serialize installation marker: {e}"))?;

        tokio::fs::write(&marker_path, json)
            .await
            .map_err(|e| format!("Failed to write installation marker: {e}"))?;

        eprintln!("[INFO] Created installation marker at {}", marker_path.display());
    } else {
        clear_progress(progress_key);
    }

    res.map_err(|e| format!("download/extraction error: {e}"))
}
