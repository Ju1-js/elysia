use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use reqwest::{Client, Url, header::RANGE};
use stream_unpack::zip::{
    ZipDecodedData, ZipPosition, ZipUnpacker, read_cd,
    structures::central_directory::CentralDirectory,
};
use tokio::sync::mpsc::Sender;

mod progress;
pub use crate::progress::Progress;

#[derive(Debug, Clone)]
pub struct Archive {
    pub url: Url,
    pub hash: Option<String>,
    pub size: u64,
}

fn total_size(archives: &[Archive]) -> u64 {
    archives.iter().map(|p| p.size).sum()
}

fn part_sizes(archives: &[Archive]) -> Vec<usize> {
    archives.iter().map(|p| p.size as usize).collect()
}

async fn verify_last_part_size(
    client: &Client,
    archives: &[Archive],
    mut sizes: Vec<usize>,
) -> Result<Vec<usize>> {
    let last_idx = sizes.len() - 1;
    let last_url = &archives[last_idx].url;

    let head_resp = client
        .head(last_url.to_owned())
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

fn create_range_provider(
    client: Client,
    archives: &[Archive],
) -> impl Fn(ZipPosition, usize) -> Result<Vec<u8>> + '_ {
    move |pos: ZipPosition, len: usize| -> Result<Vec<u8>> {
        let disk = pos.disk;
        if disk >= archives.len() {
            return Err(anyhow!("invalid disk index: {}", disk));
        }

        let url = &archives[disk].url;
        let range = format!("bytes={}-{}", pos.offset, pos.offset + len - 1);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let resp = client
                    .get(url.to_owned())
                    .header(RANGE, &range)
                    .send()
                    .await
                    .with_context(|| format!("requesting range from part {} ({})", disk, url))?;

                if !resp.status().is_success() {
                    return Err(anyhow!("HTTP error: {}", resp.status()));
                }

                Ok(resp.bytes().await?.to_vec())
            })
        })
    }
}

async fn read_central_directory(client: &Client, archives: &[Archive]) -> Result<CentralDirectory> {
    let sizes = part_sizes(archives);

    println!("Part sizes: {:?}", sizes);
    let verified_sizes = verify_last_part_size(client, archives, sizes).await?;
    println!("Verified part sizes: {:?}", verified_sizes);

    let provider = create_range_provider(client.to_owned(), archives);
    let cd = read_cd::from_provider(verified_sizes, true, provider)
        .context("failed to read central directory from cut ZIP")?;

    Ok(cd)
}

async fn load_resume_position(output_dir: &Path) -> Result<Option<ZipPosition>> {
    let status_file_path = output_dir.join(".elysia_extract_status");

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

    let disk = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let offset = u64::from_le_bytes([
        data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
    ]) as usize;

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

    unpacker.set_callback(move |data| -> Result<()> {
        match data {
            ZipDecodedData::FileHeader(cdfh, _lfh) => {
                tokio::task::block_in_place(|| -> Result<()> {
                    let mut file_to_flush = current_file_clone
                        .lock()
                        .map_err(|e| anyhow!("mutex poisoned: {}", e))?
                        .take();

                    if let Some(ref mut f) = file_to_flush {
                        std::io::Write::flush(f).context("failed to flush file")?;
                    }

                    let name = &cdfh.filename;
                    let path = out_dir_clone.join(name);

                    if name.ends_with('/') || name.ends_with('\\') {
                        std::fs::create_dir_all(&path)
                            .with_context(|| format!("failed to create directory: {:?}", path))?;
                        return Ok(());
                    }

                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent).with_context(|| {
                            format!("failed to create parent directory: {:?}", parent)
                        })?;
                    }

                    let new_file = std::fs::File::create(&path)
                        .with_context(|| format!("failed to create file: {:?}", path))?;

                    *current_file_clone
                        .lock()
                        .map_err(|e| anyhow!("mutex poisoned: {}", e))? = Some(new_file);

                    let pos = cdfh.header_position();
                    let mut state = Vec::with_capacity(12);
                    state.extend_from_slice(&(pos.disk as u32).to_le_bytes());
                    state.extend_from_slice(&(pos.offset as u64).to_le_bytes());
                    let _ = std::fs::write(&*status_path, state);

                    Ok(())
                })?;
            }
            ZipDecodedData::FileData(data) => {
                tokio::task::block_in_place(|| -> Result<()> {
                    let mut guard = current_file_clone
                        .lock()
                        .map_err(|e| anyhow!("mutex poisoned: {}", e))?;

                    if let Some(f) = guard.as_mut() {
                        std::io::Write::write_all(f, data).context("failed to write file data")?;
                    }
                    Ok(())
                })?;
            }
        }
        Ok(())
    });
}

fn calculate_resume_point(archives: &[Archive], virtual_pos: ZipPosition) -> (usize, usize, u64) {
    let mut remaining = virtual_pos.offset;
    let mut part_idx = 0;
    let mut total_downloaded = 0u64;

    for (idx, &size) in part_sizes(archives).iter().enumerate() {
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
            .map_err(|e| anyhow!("unpacker error: {:?}", e))?;

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

pub async fn stream_unpack(
    progress_sender: Sender<Progress>,
    client: Client,
    archives: Vec<Archive>,
    output_dir: PathBuf,
) -> Result<()> {
    std::fs::create_dir_all(&output_dir)?;
    let total = total_size(&archives);

    progress_sender.send(Progress::preparing()).await?;

    let cd = read_central_directory(&client, &archives).await?;
    let sorted = cd.sort();

    let part_sizes = part_sizes(&archives);
    let virtual_sizes = vec![part_sizes.iter().sum::<usize>()];
    let resume_pos = load_resume_position(&output_dir).await?;

    let mut unpacker = if let Some(pos) = resume_pos {
        ZipUnpacker::resume(sorted, virtual_sizes, pos)?
    } else {
        ZipUnpacker::new(sorted, virtual_sizes)
    };

    let status_file_path = output_dir.join(".elysia_extract_status");
    setup_extraction_callback(
        &mut unpacker,
        output_dir.to_path_buf(),
        status_file_path.clone(),
    );

    let (archive_start, start_offset, mut total_downloaded) = if let Some(pos) = resume_pos {
        let (part, offset, downloaded) = calculate_resume_point(&archives, pos);
        progress_sender
            .send(Progress::resuming(downloaded, total, part, archives.len()))
            .await?;
        (part, offset, downloaded)
    } else {
        progress_sender
            .send(Progress::downloading(0, total, 0.0, 0, archives.len()))
            .await?;

        (0, 0, 0u64)
    };

    let mut buffer = Vec::with_capacity(65536 + 4096);
    let mut last_update = Instant::now();
    let mut last_bytes = total_downloaded;
    let mut hasher = md5::Context::new();

    for (archive_idx, archive) in archives.iter().enumerate().skip(archive_start) {
        let mut request = client.get(archive.url.clone());

        if archive_idx == archive_start && start_offset > 0 {
            request = request.header(RANGE, format!("bytes={}-", start_offset));
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
            hasher.consume(&chunk);

            let now = Instant::now();
            if now.duration_since(last_update) >= Duration::from_secs(1) {
                let diff = total_downloaded - last_bytes;
                let mb_s = diff as f32 / (1024.0 * 1024.0);
                last_bytes = total_downloaded;
                last_update = now;

                progress_sender
                    .send(Progress::downloading(
                        total_downloaded,
                        total,
                        mb_s,
                        archive_idx + 1,
                        archives.len(),
                    ))
                    .await?;
            }

            if buffer.len() >= 65536 && process_buffer(&mut unpacker, &mut buffer)? {
                let _ = tokio::fs::remove_file(&status_file_path).await;
                progress_sender.send(Progress::complete(total)).await?;
                return Ok(());
            }
        }

        if let Some(expected) = &archive.hash {
            let expected = expected.to_ascii_lowercase();
            let is_partial_resume = archive_idx == archive_start && start_offset > 0;

            if is_partial_resume {
                eprintln!(
                    "[MD5] Skipping verification for part {} (resumed mid-part)",
                    archive_idx + 1
                );
            } else {
                eprintln!(
                    "[MD5] Spawning verification task for part {}...",
                    archive_idx + 1
                );

                let actual = format!("{:x}", hasher.finalize());
                if !actual.eq(&expected) {
                    return Err(anyhow!(
                        "MD5 verification failed for part {}: expected {}, got {}",
                        archive_idx + 1,
                        &expected,
                        &actual
                    ));
                };
                eprintln!("[MD5] Part {} verified successfully", archive_idx + 1);
            }
        }
        hasher = md5::Context::new()
    }

    if !buffer.is_empty() {
        let _ = process_buffer(&mut unpacker, &mut buffer)?;
    }

    let _ = tokio::fs::remove_file(&status_file_path).await;
    progress_sender.send(Progress::complete(total)).await?;
    Ok(())
}
