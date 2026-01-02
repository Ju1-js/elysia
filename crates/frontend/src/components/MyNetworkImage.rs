use std::sync::{Arc, RwLock};
use bytes::Bytes;
use freya::prelude::*;
use libwebp::WebPDecodeRGBA;
use reqwest::{Url, header::CONTENT_TYPE};
use skia_safe::{AlphaType, ColorType, Data, EncodedImageFormat, ImageInfo};

use backend::settings::GlobalSettings;

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
    let settings_signal = use_context::<Signal<Arc<RwLock<GlobalSettings>>>>();

    let cache_path = use_memo(move || {
        settings_signal.read()
            .read()
            .ok()
            .map(|s| s.cache_directory.clone())
            .unwrap_or_default()
    });

    let a11y_id = focus.attribute();

    let image_resource = use_resource(use_reactive!(|url| async move {
        let url_value = url.read().clone();
        let cache_path_value = cache_path();
        let cache_key = url_value.to_string();
        
        let bytes = match cacache::read(&cache_path_value, &cache_key).await {
            Ok(cached_bytes) => Bytes::from(cached_bytes),
            Err(_) => {
                let fetched_bytes = fetch_image(url_value).await?;
                
                let cache_path_clone = cache_path_value.clone();
                let key_clone = cache_key.clone();
                let bytes_clone = fetched_bytes.clone();
                
                tokio::spawn(async move {
                    let _ = cacache::write(&cache_path_clone, &key_clone, &bytes_clone).await;
                });
                
                fetched_bytes
            }
        };

        Ok::<Bytes, String>(bytes)
    }));
    
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
        Some(Err(error)) => {
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

pub async fn fetch_image(url: Url) -> Result<Bytes, String> {
    let response = reqwest::get(url.clone())
        .await
        .map_err(|err| format!("Failed to fetch image from {}: {}", url, err))?;

    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|ct| ct.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    let bytes = response
        .bytes()
        .await
        .map_err(|err| format!("Failed to read image bytes from {}: {}", url, err))?;

    if content_type.contains("webp") {
        transcode_webp_to_png(&bytes)
    } else {
        Ok(bytes)
    }
}

fn transcode_webp_to_png(webp_bytes: &[u8]) -> Result<Bytes, String> {
    let (width, height, rgba_pixels) = WebPDecodeRGBA(webp_bytes)
        .map_err(|err| format!("Failed to decode WebP image: {}", err))?;

    let image_info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );

    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or_else(|| format!("Image dimensions too large: {}x{}", width, height))?;
    
    let pixel_data = Data::new_copy(&rgba_pixels);
    
    let image = skia_safe::images::raster_from_data(&image_info, pixel_data, row_bytes)
        .ok_or_else(|| "Failed to create Skia image from raw data".to_string())?;

    let encoded_data = image
        .encode(None, EncodedImageFormat::PNG, None)
        .ok_or_else(|| "Failed to encode image to PNG".to_string())?;

    Ok(encoded_data.as_bytes().to_vec().into())
}
