use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use bytes::Bytes;
use freya::prelude::*;
use libwebp::WebPDecodeRGBA;
use reqwest::{Url, header::CONTENT_TYPE};
use skia_safe::{AlphaType, ColorType, Data, EncodedImageFormat, ImageInfo};

use backend::settings::GlobalSettings;

static MEMORY_CACHE: once_cell::sync::Lazy<Arc<RwLock<HashMap<String, Bytes>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

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

    // Use Arc to avoid cloning the entire path
    let cache_path = use_memo(move || {
        let settings = ctx.read();
        Arc::new(settings.read().unwrap().cache_directory.clone())
    });

    let a11y_id = focus.attribute();

    let image_resource = use_resource(move || {
        let url_value = url.read().clone();
        let cache_path = cache_path.read().clone();
        let key = url_value.to_string();
        
        async move {
            // Check memory cache first
            if let Some(cached_bytes) = MEMORY_CACHE.read().unwrap().get(&key).cloned() {
                return Ok(cached_bytes);
            }

            // Try disk cache, if it fails, fetch from network
            let bytes = match cacache::read(&*cache_path, &key).await {
                Ok(asset) => {
                    let bytes: Bytes = asset.into();
                    // Populate memory cache from disk
                    MEMORY_CACHE.write().unwrap().insert(key.clone(), bytes.clone());
                    bytes
                }
                Err(_) => {
                    // Fetch from network
                    let asset_bytes = fetch_image(url_value).await?;
                    
                    // Write to disk cache (log errors instead of silently ignoring)
                    if let Err(e) = cacache::write(&*cache_path, &key, &asset_bytes).await {
                        eprintln!("Failed to write to disk cache for {}: {}", key, e);
                    }
                    
                    // Populate memory cache
                    MEMORY_CACHE.write().unwrap().insert(key.clone(), asset_bytes.clone());
                    asset_bytes
                }
            };

            Ok::<Bytes, String>(bytes)
        }
    });
    
    let key_for_ui = url.read().to_string();

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
                    cache_key: "{key_for_ui}",
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
    let res = reqwest::get(url.clone())
        .await
        .map_err(|e| format!("Failed to fetch image: {e}"))?;

    let content_type = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("")
        .to_owned();

    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image bytes: {e}"))?;

    // Only transcode WebP, pass through other formats
    if content_type == "image/webp" {
        transcode_webp_to_png(&bytes)
    } else {
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