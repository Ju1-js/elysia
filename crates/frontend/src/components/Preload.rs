use dashmap::DashMap;
use bytes::Bytes;
use reqwest::Url;

pub static MEMORY_CACHE: once_cell::sync::Lazy<DashMap<String, Bytes>> =
    once_cell::sync::Lazy::new(|| DashMap::new());

pub fn preload_images(
    urls: Vec<Url>,
    cache_path: String,
    fetch_image_fn: impl Fn(Url) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Bytes, String>> + Send>> + Send + 'static,
) {
    let urls_to_load: Vec<Url> = urls.into_iter()
        .filter(|url| !MEMORY_CACHE.contains_key(&url.to_string()))
        .collect();
    
    if urls_to_load.is_empty() {
        return;
    }
    
    tokio::spawn(async move {
        for url in urls_to_load {
            let key = url.to_string();
            
            if let Ok(asset) = cacache::read(&cache_path, &key).await {
                let bytes = Bytes::from(asset);
                MEMORY_CACHE.insert(key, bytes);
                continue;
            }
            
            if let Ok(bytes) = fetch_image_fn(url).await {
                MEMORY_CACHE.insert(key.clone(), bytes.clone());
                let _ = cacache::write(&cache_path, &key, &bytes).await;
            }
        }
    });
}

pub fn preload_videos(urls: Vec<Url>, cache_path: String, max_videos: usize) {
    let urls_to_load: Vec<Url> = urls.into_iter()
        .filter(|url| !MEMORY_CACHE.contains_key(&url.to_string()))
        .take(max_videos)
        .collect();
    
    if urls_to_load.is_empty() {
        return;
    }
    
    tokio::spawn(async move {
        for url in urls_to_load.into_iter() {
            let key = url.to_string();
            
            if let Ok(asset) = cacache::read(&cache_path, &key).await {
                let bytes = Bytes::from(asset);
                MEMORY_CACHE.insert(key, bytes);
                continue;
            }

            match reqwest::get(url.clone()).await {
                Ok(res) => match res.bytes().await {
                    Ok(bytes) => {
                        MEMORY_CACHE.insert(key.clone(), bytes.clone());

                        let cache_path_clone = cache_path.clone();
                        tokio::spawn(async move {
                            if let Err(e) = cacache::write(&cache_path_clone, &key, &bytes).await {
                                eprintln!("Failed to cache video {}: {}", key, e);
                            }
                        });
                    }
                    Err(e) => eprintln!("Failed to download video bytes from {}: {}", url, e),
                },
                Err(e) => eprintln!("Failed to fetch video from {}: {}", url, e),
            }
        }
    });
}
