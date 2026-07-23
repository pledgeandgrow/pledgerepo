// Favicon generation — generate all favicon sizes from a single source image.
//
// Features:
//   - Generate 16x16, 32x32, 180x180, 512x512 favicon sizes
//   - PWA-compatible output (manifest icons, apple-touch-icon, favicon.ico)
//   - HTML <link> tag generation for all sizes
//   - Web App Manifest icon entries

use anyhow::{Result, bail};
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use std::io::Cursor;

/// Standard favicon sizes to generate
pub const FAVICON_SIZES: &[u32] = &[16, 32, 180, 512];

/// Result of favicon generation
#[derive(Debug, Clone)]
pub struct FaviconOutput {
    pub size: u32,
    pub data: Vec<u8>,
    pub filename: String,
    pub mime_type: &'static str,
}

/// Generate all favicon sizes from a source image
pub fn generate_favicons(source: &[u8], _source_filename: &str) -> Result<Vec<FaviconOutput>> {
    let img = image::load_from_memory(source)
        .map_err(|e| anyhow::anyhow!("Failed to decode favicon source: {}", e))?;

    let (orig_w, orig_h) = img.dimensions();
    if orig_w < 16 || orig_h < 16 {
        bail!("Favicon source image must be at least 16x16 pixels");
    }

    let mut outputs = Vec::new();

    for &size in FAVICON_SIZES {
        let resized = img.resize_exact(size, size, FilterType::Lanczos3);
        let mut buf = Cursor::new(Vec::new());
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        resized
            .write_with_encoder(encoder)
            .map_err(|e| anyhow::anyhow!("PNG encode error for {}x{}: {}", size, size, e))?;

        let filename = match size {
            16 | 32 => format!("favicon-{}.png", size),
            180 => "apple-touch-icon.png".to_string(),
            512 => "favicon-512.png".to_string(),
            _ => format!("favicon-{}.png", size),
        };

        outputs.push(FaviconOutput {
            size,
            data: buf.into_inner(),
            filename,
            mime_type: "image/png",
        });
    }

    // Generate ICO (multi-resolution) containing 16x16 and 32x32
    let ico = generate_ico(&img, &[16, 32])?;
    outputs.push(FaviconOutput {
        size: 0,
        data: ico,
        filename: "favicon.ico".to_string(),
        mime_type: "image/x-icon",
    });

    Ok(outputs)
}

/// Generate a multi-resolution ICO file
fn generate_ico(img: &DynamicImage, sizes: &[u32]) -> Result<Vec<u8>> {
    let mut ico = Vec::new();

    // ICO header: reserved(2) = 0, type(2) = 1 (icon), count(2)
    ico.extend_from_slice(&[0, 0, 1, 0]);
    ico.extend_from_slice(&(sizes.len() as u16).to_le_bytes());

    // Reserve space for directory entries (16 bytes each)
    let dir_offset = 6 + sizes.len() * 16;
    let mut image_data_offset = dir_offset;
    let mut image_data_list: Vec<Vec<u8>> = Vec::new();

    for &size in sizes {
        let resized = img.resize_exact(size, size, FilterType::Lanczos3);
        let mut buf = Cursor::new(Vec::new());
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        resized
            .write_with_encoder(encoder)
            .map_err(|e| anyhow::anyhow!("ICO PNG encode error: {}", e))?;
        let png_data = buf.into_inner();
        let data_len = png_data.len() as u32;

        // Directory entry: width(1), height(1), colors(1)=0, reserved(1)=0, planes(2)=1, bpp(2)=32, size(4), offset(4)
        ico.push(if size >= 256 { 0 } else { size as u8 }); // width
        ico.push(if size >= 256 { 0 } else { size as u8 }); // height
        ico.push(0); // color palette
        ico.push(0); // reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // color planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        ico.extend_from_slice(&data_len.to_le_bytes()); // image size
        ico.extend_from_slice(&(image_data_offset as u32).to_le_bytes()); // offset

        image_data_offset += data_len as usize;
        image_data_list.push(png_data);
    }

    // Append image data
    for data in image_data_list {
        ico.extend_from_slice(&data);
    }

    Ok(ico)
}

/// Generate HTML <link> tags for all favicon sizes
pub fn generate_favicon_html(outputs: &[FaviconOutput]) -> String {
    let mut html = String::new();

    for output in outputs {
        if output.filename == "favicon.ico" {
            html.push_str(&format!(
                r#"<link rel="icon" href="/{}" type="{}" />"#,
                output.filename, output.mime_type
            ));
            html.push('\n');
        } else if output.filename == "apple-touch-icon.png" {
            html.push_str(&format!(
                r#"<link rel="apple-touch-icon" sizes="{}x{}" href="/{}" />"#,
                output.size, output.size, output.filename
            ));
            html.push('\n');
        } else {
            html.push_str(&format!(
                r#"<link rel="icon" type="image/png" sizes="{}x{}" href="/{}" />"#,
                output.size, output.size, output.filename
            ));
            html.push('\n');
        }
    }

    html
}

/// Generate Web App Manifest icon entries
pub fn generate_manifest_icons(outputs: &[FaviconOutput]) -> Vec<serde_json::Value> {
    outputs
        .iter()
        .filter(|o| o.size > 0)
        .map(|o| {
            serde_json::json!({
                "src": format!("/{}", o.filename),
                "sizes": format!("{}x{}", o.size, o.size),
                "type": o.mime_type,
                "purpose": "any maskable",
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_favicons() {
        let img = DynamicImage::new_rgb8(512, 512);
        let mut buf = Cursor::new(Vec::new());
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        img.write_with_encoder(encoder).unwrap();
        let png_data = buf.into_inner();

        let result = generate_favicons(&png_data, "icon.png");
        assert!(result.is_ok());
        let outputs = result.unwrap();
        assert!(outputs.len() >= 4);
        assert!(outputs.iter().any(|o| o.filename == "favicon.ico"));
        assert!(outputs.iter().any(|o| o.filename == "apple-touch-icon.png"));
    }

    #[test]
    fn test_generate_favicon_html() {
        let outputs = vec![
            FaviconOutput { size: 32, data: vec![], filename: "favicon-32.png".to_string(), mime_type: "image/png" },
            FaviconOutput { size: 180, data: vec![], filename: "apple-touch-icon.png".to_string(), mime_type: "image/png" },
            FaviconOutput { size: 0, data: vec![], filename: "favicon.ico".to_string(), mime_type: "image/x-icon" },
        ];
        let html = generate_favicon_html(&outputs);
        assert!(html.contains("rel=\"icon\""));
        assert!(html.contains("apple-touch-icon"));
        assert!(html.contains("favicon.ico"));
    }
}
