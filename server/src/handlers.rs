use axum::{
    extract::Multipart,
    http::{StatusCode, header},
    response::{IntoResponse, Response, Json},
};
use serde::Serialize;
use std::io::Write as IoWrite;
use tempfile::NamedTempFile;

use image_preparer_core::config::{ProcessingConfig, StripMode};
use image_preparer_core::converter::{ConvertFormat, convert_image};
use image_preparer_core::format::ImageFormat;
use image_preparer_core::pipeline::Pipeline;
use image_preparer_core::processor::png::PngProcessor;
use image_preparer_core::processor::jpg::JpgProcessor;
use image_preparer_core::processor::webp::WebpProcessor;
use image_preparer_core::processor::mp3::Mp3Processor;
use image_preparer_core::processor::mp4::Mp4Processor;
use image_preparer_core::processor::wav::WavProcessor;

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct InspectResult {
    format: String,
    size: u64,
    metadata: serde_json::Value,
}

/// POST /compress
///
/// Compress uploaded image or video.
///
/// Form fields:
/// - file: binary file data
/// - quality (optional): 0-100 (default: 80)
/// - speed (optional): 1-10 (default: 3)
/// - no_lossy (optional): true/false (default: false)
/// - strip (optional): all/safe/none (default: all)
pub async fn compress(mut multipart: Multipart) -> Result<Response, StatusCode> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut quality = 80u8;
    let mut speed = 3i32;
    let mut no_lossy = false;
    let mut strip = StripMode::All;

    // Parse multipart form
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        };

        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                file_data = Some(bytes.to_vec());
            }
            "quality" => {
                if let Ok(text) = field.text().await {
                    quality = text.parse::<u8>().unwrap_or(80).clamp(0, 100);
                }
            }
            "speed" => {
                if let Ok(text) = field.text().await {
                    speed = text.parse::<i32>().unwrap_or(3).clamp(1, 10);
                }
            }
            "no_lossy" => {
                if let Ok(text) = field.text().await {
                    no_lossy = text == "true";
                }
            }
            "strip" => {
                if let Ok(text) = field.text().await {
                    strip = match text.as_str() {
                        "safe" => StripMode::Safe,
                        "none" => StripMode::None,
                        _ => StripMode::All,
                    };
                }
            }
            _ => {}
        }
    }

    let data = file_data.ok_or(StatusCode::BAD_REQUEST)?;

    // Create temp file to detect format
    let mut temp_file = NamedTempFile::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    temp_file.write_all(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _format = ImageFormat::from_path(temp_file.path())
        .ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;

    // Build pipeline
    let mut pipeline = Pipeline::new();
    pipeline.register(Box::new(PngProcessor));
    pipeline.register(Box::new(JpgProcessor));
    pipeline.register(Box::new(WebpProcessor));
    pipeline.register(Box::new(Mp3Processor));
    pipeline.register(Box::new(WavProcessor));
    pipeline.register(Box::new(Mp4Processor));

    // Create config
    let config = ProcessingConfig {
        quality,
        speed,
        no_lossy,
        strip,
        dry_run: false,
        backup: false,
        extract_frames: false,
        fps: 0.0,
        resize_width: None,
        resize_height: None,
    };

    // Process file
    match pipeline.process_file(temp_file.path(), &data, &config) {
        Ok(compressed) => {
            Ok((
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/octet-stream")],
                compressed,
            ).into_response())
        }
        Err(e) => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(e.to_string()),
            };
            Ok(Json(response).into_response())
        }
    }
}

/// POST /convert
///
/// Convert image between formats (PNG, JPG, WebP).
///
/// Form fields:
/// - file: binary file data
/// - to: target format (png, jpg, jpeg, webp)
/// - quality (optional): 0-100 (default: 80)
/// - no_lossy (optional): true/false (default: false)
pub async fn convert(mut multipart: Multipart) -> Result<Response, StatusCode> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut target_format: Option<String> = None;
    let mut quality = 80u8;
    let mut no_lossy = false;

    // Parse multipart form
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        };

        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                file_data = Some(bytes.to_vec());
            }
            "to" => {
                let text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                target_format = Some(text);
            }
            "quality" => {
                if let Ok(text) = field.text().await {
                    quality = text.parse::<u8>().unwrap_or(80).clamp(0, 100);
                }
            }
            "no_lossy" => {
                if let Ok(text) = field.text().await {
                    no_lossy = text == "true";
                }
            }
            _ => {}
        }
    }

    let data = file_data.ok_or(StatusCode::BAD_REQUEST)?;
    let target_format_str = target_format.ok_or(StatusCode::BAD_REQUEST)?;

    let target_format = ConvertFormat::from_str(&target_format_str)
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Create config
    let config = ProcessingConfig {
        quality,
        speed: 3,
        no_lossy,
        strip: StripMode::All,
        dry_run: false,
        backup: false,
        extract_frames: false,
        fps: 0.0,
        resize_width: None,
        resize_height: None,
    };

    // Convert
    match convert_image(&data, target_format, &config) {
        Ok(converted) => {
            Ok((
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/octet-stream")],
                converted,
            ).into_response())
        }
        Err(e) => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(e.to_string()),
            };
            Ok(Json(response).into_response())
        }
    }
}

/// POST /inspect
///
/// View file metadata.
///
/// Form fields:
/// - file: binary file data
pub async fn inspect(mut multipart: Multipart) -> Result<Response, StatusCode> {
    let mut file_data: Option<Vec<u8>> = None;

    // Parse multipart form
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        };

        if field.name() == Some("file") {
            let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            file_data = Some(bytes.to_vec());
            break;
        }
    }

    let data = file_data.ok_or(StatusCode::BAD_REQUEST)?;
    let size = data.len() as u64;

    // Create temp file to detect format
    let mut temp_file = NamedTempFile::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    temp_file.write_all(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let format = ImageFormat::from_path(temp_file.path())
        .ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;

    // For now, return basic info
    // TODO: Implement proper metadata extraction for each format
    let result = InspectResult {
        format: format.as_str().to_string(),
        size,
        metadata: serde_json::json!({
            "note": "Detailed metadata extraction coming soon"
        }),
    };

    let response = ApiResponse {
        success: true,
        data: Some(result),
        error: None,
    };

    Ok(Json(response).into_response())
}

/// POST /extract
///
/// Extract frames from MP4 video.
///
/// Form fields:
/// - file: binary MP4 file
/// - fps (optional): frames per second (default: 1, 0=all frames)
pub async fn extract(mut multipart: Multipart) -> Result<Response, StatusCode> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut _fps = 1.0f32;

    // Parse multipart form
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        };

        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                file_data = Some(bytes.to_vec());
            }
            "fps" => {
                if let Ok(text) = field.text().await {
                    _fps = text.parse::<f32>().unwrap_or(1.0);
                }
            }
            _ => {}
        }
    }

    let _data = file_data.ok_or(StatusCode::BAD_REQUEST)?;

    // TODO: Implement frame extraction
    // This requires saving temp files and using extract_frames_to_png from CLI

    let response = ApiResponse::<()> {
        success: false,
        data: None,
        error: Some("Frame extraction not yet implemented for web API".to_string()),
    };

    Ok(Json(response).into_response())
}
