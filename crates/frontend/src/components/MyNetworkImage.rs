use std::sync::{Arc, RwLock};
use dashmap::DashMap;
use bytes::Bytes;
use freya::prelude::*;
use libwebp::WebPDecodeRGBA;
use reqwest::{Url, header::CONTENT_TYPE};
use skia_safe::{AlphaType, ColorType, Data, EncodedImageFormat, ImageInfo};

use backend::settings::GlobalSettings;

// More efficient concurrent cache than RwLock<HashMap>
static MEMORY_CACHE: once_cell::sync::Lazy<DashMap<String, Bytes>> =
    once_cell::sync::Lazy::new(|| DashMap::new());

#[derive(Props, Clone, PartialEq)]
pub struct MyNetworkImageProps {
    #[props(default = "auto".into())]
    pub width: String,
    #[props(default = "auto".into())]
    pub height: String,
    pub min_width: Option<String>,
    pub min_height: Option<String>,
    pub url: ReadOnlySignal<Url>,
    pub fallback: Option<Element>,
    pub loading: Option<Element>,
    pub alt: Option<String>,
    pub aspect_ratio: Option<String>,
    pub cover: Option<String>,
    pub sampling: Option<String>,
}

#[component]
pub fn MyNetworkImage(
    MyNetworkImageProps {
        width,
        height,
        min_width,
        min_height,
        url,
        fallback,
        loading,
        alt,
        aspect_ratio,
        cover,
        sampling,
    }: MyNetworkImageProps,
) -> Element {
    let focus = use_focus();
    let ctx = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();

    // Memoize cache_path - it never changes during component lifetime
    // This avoids recomputing on every render
    let cache_path = use_memo(move || {
        ctx.read()
            .read()
            .ok()
            .map(|s| s.cache_directory.clone())
            .unwrap_or_default()
    });

    let a11y_id = focus.attribute();

    // use_resource automatically re-runs when url changes
    let image_resource = use_resource(move || {
        let url_value = url.read().clone();
        let cache_path_value = cache_path();
        let key = url_value.to_string();
        
        async move {
            // Check memory cache first (fast path)
            if let Some(cached_bytes) = MEMORY_CACHE.get(&key) {
                return Ok(cached_bytes.value().clone());
            }

            // Try disk cache
            let bytes = match cacache::read(&cache_path_value, &key).await {
                Ok(asset) => {
                    let bytes = Bytes::from(asset);
                    // Populate memory cache from disk
                    MEMORY_CACHE.insert(key, bytes.clone());
                    bytes
                }
                Err(_) => {
                    // Fetch from network
                    let asset_bytes = fetch_image(url_value).await?;
                    
                    // Write to disk cache asynchronously (don't block UI)
                    let cache_path_clone = cache_path_value.clone();
                    let key_clone = key.clone();
                    let bytes_clone = asset_bytes.clone();
                    tokio::spawn(async move {
                        if let Err(e) = cacache::write(&cache_path_clone, &key_clone, &bytes_clone).await {
                            eprintln!("Failed to write to disk cache for {}: {}", key_clone, e);
                        }
                    });
                    
                    // Populate memory cache
                    MEMORY_CACHE.insert(key, asset_bytes.clone());
                    asset_bytes
                }
            };

            Ok::<Bytes, String>(bytes)
        }
    });
    
    // Derive url_string inline - it's cheap and depends on url signal
    let url_string = url.read().to_string();

    match &*image_resource.read_unchecked() {
        Some(Ok(bytes)) => {
            let image_data = dynamic_bytes(bytes.clone());
            rsx! {
                image {
                    height,
                    width,
                    min_width,
                    min_height,
                    a11y_id,
                    image_data,
                    a11y_role: "image",
                    a11y_name: alt,
                    aspect_ratio,
                    cover,
                    cache_key: "{url_string}",
                    sampling,
                }
            }
        }
        Some(Err(_)) => {
            if let Some(fallback_element) = fallback {
                rsx! {{ fallback_element }}
            } else {
                rsx! {
                    rect {
                        height,
                        width,
                        min_width,
                        min_height,
                        main_align: "center",
                        cross_align: "center",
                        label {
                            text_align: "center",
                            "Error loading image"
                        }
                    }
                }
            }
        }
        None => {
            if let Some(loading_element) = loading {
                rsx! {{ loading_element }}
            } else {
                rsx! {
                    rect {
                        height,
                        width,
                        min_width,
                        min_height,
                        main_align: "center",
                        cross_align: "center",
                        Loader {}
                    }
                }
            }
        }
    }
}

async fn fetch_image(url: Url) -> Result<Bytes, String> {
    let res = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to fetch image: {e}"))?;

    let content_type = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image bytes: {e}"))?;

    // WebP transcoding: Freya's image element may not natively support WebP format,
    // so we convert it to PNG. This adds processing time but ensures compatibility.
    // 
    // Performance consideration: If Freya adds native WebP support in the future,
    // this conversion can be removed for a significant speed boost.
    // 
    // Alternative: You could try removing this conversion and testing if Freya
    // handles WebP natively - if it works, you'll get faster loading!
    if content_type.contains("webp") {
        transcode_webp_to_png(&bytes)
    } else {
        // Pass through PNG, JPEG, and other formats directly - fastest path
        Ok(bytes)
    }
}

fn transcode_webp_to_png(bytes: &[u8]) -> Result<Bytes, String> {
    let (width, height, buf) = WebPDecodeRGBA(bytes)
        .map_err(|e| format!("Failed to decode WebP image: {e}"))?;

    let info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );

    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| "Image dimensions too large".to_string())?;
    
    let data = Data::new_copy(&buf);
    let image = skia_safe::images::raster_from_data(&info, data, row_bytes)
        .ok_or_else(|| "Failed to create Skia image from raw data".to_string())?;

    let encoded_data = image
        .encode(None, EncodedImageFormat::PNG, None)
        .ok_or_else(|| "Failed to encode image to PNG".to_string())?;

    Ok(encoded_data.as_bytes().to_vec().into())
}

// Preload images in the background for better UX
pub fn preload_images(urls: Vec<Url>, cache_path: String) {
    tokio::spawn(async move {
        for url in urls {
            let key = url.to_string();
            
            // Skip if already in memory cache
            if MEMORY_CACHE.contains_key(&key) {
                continue;
            }
            
            // Try disk cache first
            if let Ok(asset) = cacache::read(&cache_path, &key).await {
                let bytes = Bytes::from(asset);
                MEMORY_CACHE.insert(key, bytes);
                continue;
            }
            
            // Fetch from network in background
            if let Ok(bytes) = fetch_image(url).await {
                MEMORY_CACHE.insert(key.clone(), bytes.clone());
                let _ = cacache::write(&cache_path, &key, &bytes).await;
            }
        }
    });
}