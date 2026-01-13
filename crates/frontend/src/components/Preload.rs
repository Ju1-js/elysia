use bytes::Bytes;
use reqwest::Url;
use std::path::PathBuf;

use super::fetch_image;

/// Preload images into the cache
pub fn preload_images(urls: Vec<Url>, cache_path: String) {
    tokio::spawn(async move {
        for url in urls.into_iter() {
            let key = url.to_string();

            if cacache::read(&cache_path, &key).await.is_ok() {
                continue;
            }

            if let Ok(bytes) = fetch_image(url).await {
                let _ = cacache::write(&cache_path, &key, &bytes).await;
                drop(bytes);
            }

            tokio::task::yield_now().await;
        }
    });
}

/// Preload videos into the cache directory
pub fn preload_videos(urls: Vec<Url>, cache_dir: PathBuf, max_videos: usize) {
    tokio::spawn(async move {
        let videos_cache_dir = cache_dir.join("videos");
        if tokio::fs::create_dir_all(&videos_cache_dir).await.is_err() {
            return;
        }

        for url in urls.into_iter().take(max_videos) {
            let url_str = url.to_string();

            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            url_str.hash(&mut hasher);

            let cache_path = videos_cache_dir.join(format!("video_{:x}.webm", hasher.finish()));

            if cache_path.exists() {
                continue;
            }

            let temp_path = cache_path.with_extension("tmp");

            if let Ok(response) = reqwest::get(url).await {
                if let Ok(mut file) = tokio::fs::File::create(&temp_path).await {
                    use tokio::io::AsyncWriteExt;
                    use tokio_stream::StreamExt;

                    let mut stream = response.bytes_stream();
                    let mut success = true;

                    while let Some(chunk_result) = stream.next().await {
                        if let Ok(chunk) = chunk_result {
                            if file.write_all(&chunk).await.is_err() {
                                success = false;
                                break;
                            }
                        } else {
                            success = false;
                            break;
                        }
                    }

                    if success {
                        success = file.flush().await.is_ok();
                    }

                    drop(file);

                    if success {
                        let _ = tokio::fs::rename(&temp_path, &cache_path).await;
                    } else {
                        let _ = tokio::fs::remove_file(&temp_path).await;
                    }
                }
            }

            tokio::task::yield_now().await;
        }
    });
}
