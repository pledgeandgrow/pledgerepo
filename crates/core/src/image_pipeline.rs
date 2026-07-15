// Image pipeline — image decoding, resizing, format conversion, srcset, blur placeholder.
//
// Features:
//   - Automatic image optimization (WebP, JPEG, PNG re-encoding)
//   - Responsive srcset generation for different viewport sizes
//   - Lazy loading attributes
//   - Blur placeholder generation (LQIP — tiny base64-encoded image)
//   - Format conversion with quality control

use anyhow::{Result, bail};
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use std::io::Cursor;
use std::path::Path;

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    WebP,
    AVIF,
    JPEG,
    PNG,
    GIF,
    SVG,
}

impl ImageFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "webp" => Some(Self::WebP),
            "avif" => Some(Self::AVIF),
            "jpg" | "jpeg" => Some(Self::JPEG),
            "png" => Some(Self::PNG),
            "gif" => Some(Self::GIF),
            "svg" => Some(Self::SVG),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::WebP => "webp",
            Self::AVIF => "avif",
            Self::JPEG => "jpg",
            Self::PNG => "png",
            Self::GIF => "gif",
            Self::SVG => "svg",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::WebP => "image/webp",
            Self::AVIF => "image/avif",
            Self::JPEG => "image/jpeg",
            Self::PNG => "image/png",
            Self::GIF => "image/gif",
            Self::SVG => "image/svg+xml",
        }
    }
}

/// Image processing options
#[derive(Debug, Clone)]
pub struct ImageOptions {
    /// Output formats to generate
    pub formats: Vec<ImageFormat>,
    /// Widths for responsive srcset (in pixels)
    pub widths: Vec<u32>,
    /// Quality (1-100)
    pub quality: u8,
    /// Generate blur placeholder
    pub blur_placeholder: bool,
    /// Progressive JPEG
    pub progressive: bool,
    /// Strip metadata
    pub strip_metadata: bool,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            formats: vec![ImageFormat::WebP, ImageFormat::AVIF],
            widths: vec![640, 750, 828, 1080, 1200, 1920, 2048],
            quality: 80,
            blur_placeholder: true,
            progressive: true,
            strip_metadata: true,
        }
    }
}

/// Result of image processing
#[derive(Debug, Clone)]
pub struct ProcessedImage {
    pub original_path: String,
    pub outputs: Vec<ImageOutput>,
    pub srcset: String,
    pub blur_placeholder: Option<String>,
    pub width: u32,
    pub height: u32,
}

/// A single output variant
#[derive(Debug, Clone)]
pub struct ImageOutput {
    pub format: ImageFormat,
    pub width: u32,
    pub data: Vec<u8>,
    pub path: String,
    pub size_bytes: u64,
}

/// Process an image file — decode, resize for each srcset width, encode to target formats
pub fn process_image(
    source: &[u8],
    original_path: &str,
    opts: &ImageOptions,
) -> Result<ProcessedImage> {
    // Decode the image
    let img = image::load_from_memory(source)
        .map_err(|e| anyhow::anyhow!("Failed to decode image: {}", e))?;

    let (orig_width, orig_height) = img.dimensions();

    let mut outputs: Vec<ImageOutput> = Vec::new();
    let mut all_widths: Vec<u32> = Vec::new();

    // Generate resized variants for each target width
    for &target_width in &opts.widths {
        // Skip widths larger than original
        if target_width >= orig_width {
            continue;
        }
        all_widths.push(target_width);

        // Resize using Lanczos filter for quality
        let resized = img.resize(target_width, u32::MAX, FilterType::Lanczos3);

        // Encode to each target format
        for &target_format in &opts.formats {
            if target_format == ImageFormat::AVIF {
                // AVIF not supported by image crate yet — skip
                continue;
            }

            let encoded = encode_image(&resized, target_format, opts.quality)?;
            let path = generate_output_path(original_path, target_width, target_format);

            outputs.push(ImageOutput {
                format: target_format,
                width: target_width,
                size_bytes: encoded.len() as u64,
                data: encoded,
                path,
            });
        }
    }

    // Also encode the original-size image in target formats
    for &target_format in &opts.formats {
        if target_format == ImageFormat::AVIF {
            continue;
        }

        let encoded = encode_image(&img, target_format, opts.quality)?;
        let path = generate_output_path(original_path, orig_width, target_format);

        outputs.push(ImageOutput {
            format: target_format,
            width: orig_width,
            size_bytes: encoded.len() as u64,
            data: encoded,
            path,
        });
    }

    // Generate blur placeholder (LQIP)
    let blur_placeholder = if opts.blur_placeholder {
        Some(generate_blur_placeholder(&img)?)
    } else {
        None
    };

    // Generate srcset string
    let srcset = generate_srcset(&outputs);

    Ok(ProcessedImage {
        original_path: original_path.to_string(),
        outputs,
        srcset,
        blur_placeholder,
        width: orig_width,
        height: orig_height,
    })
}

/// Encode a DynamicImage to the target format
fn encode_image(img: &DynamicImage, format: ImageFormat, quality: u8) -> Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());

    match format {
        ImageFormat::WebP => {
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
            img.write_with_encoder(encoder)
                .map_err(|e| anyhow::anyhow!("WebP encode error: {}", e))?;
        }
        ImageFormat::JPEG => {
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
            img.write_with_encoder(encoder)
                .map_err(|e| anyhow::anyhow!("JPEG encode error: {}", e))?;
        }
        ImageFormat::PNG => {
            let encoder = image::codecs::png::PngEncoder::new(&mut buf);
            img.write_with_encoder(encoder)
                .map_err(|e| anyhow::anyhow!("PNG encode error: {}", e))?;
        }
        _ => {
            bail!("Unsupported output format: {:?}", format);
        }
    }

    Ok(buf.into_inner())
}

/// Generate a tiny blur placeholder (LQIP) as base64 data URI
/// Resizes to 20px wide, blurs, encodes as JPEG base64
fn generate_blur_placeholder(img: &DynamicImage) -> Result<String> {
    let tiny = img.resize(20, u32::MAX, FilterType::Lanczos3);
    let mut buf = Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 30);
    tiny.write_with_encoder(encoder)
        .map_err(|e| anyhow::anyhow!("LQIP encode error: {}", e))?;

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf.into_inner());
    Ok(format!("data:image/jpeg;base64,{}", b64))
}

/// Generate output path for a resized variant
fn generate_output_path(original: &str, width: u32, format: ImageFormat) -> String {
    let stem = Path::new(original)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    format!("assets/{}.{}.{}", stem, width, format.extension())
}

/// Generate srcset string from processed image outputs
pub fn generate_srcset(outputs: &[ImageOutput]) -> String {
    outputs
        .iter()
        .map(|o| format!("{} {}w", o.path, o.width))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Generate HTML img tag with srcset and lazy loading
pub fn generate_img_tag(src: &str, srcset: &str, alt: &str, width: u32, height: u32) -> String {
    format!(
        r#"<img src="{}" srcset="{}" alt="{}" width="{}" height="{}" loading="lazy" decoding="async" />"#,
        src, srcset, alt, width, height
    )
}

/// Generate HTML picture tag with multiple formats
pub fn generate_picture_tag(outputs: &[ImageOutput], fallback: &str, alt: &str, width: u32, height: u32) -> String {
    let mut sources = Vec::new();
    for format in &[ImageFormat::AVIF, ImageFormat::WebP, ImageFormat::JPEG] {
        let format_outputs: Vec<_> = outputs.iter().filter(|o| o.format == *format).collect();
        if format_outputs.is_empty() {
            continue;
        }
        let srcset = generate_srcset(&format_outputs.iter().map(|o| (*o).clone()).collect::<Vec<_>>());
        sources.push(format!(
            r#"<source srcset="{}" type="{}" />"#,
            srcset,
            format.mime_type()
        ));
    }

    format!(
        r#"<picture>{}
  <img src="{}" alt="{}" width="{}" height="{}" loading="lazy" decoding="async" />
</picture>"#,
        sources.join("\n  "),
        fallback,
        alt,
        width,
        height
    )
}

/// Generate a JS module that exports image metadata (src, srcset, blur placeholder)
pub fn generate_image_module(processed: &ProcessedImage) -> String {
    let blur = processed.blur_placeholder.as_deref().unwrap_or("");
    format!(
        r#"const src = "/{}";
const srcset = "{}";
const blurPlaceholder = "{}";
export {{ src, srcset, blurPlaceholder }};
export default src;"#,
        processed.original_path,
        processed.srcset,
        blur
    )
}

/// Check if a file is an image
pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(ImageFormat::from_extension)
        .is_some()
}

/// Get the image format from a file path
pub fn get_image_format(path: &Path) -> Option<ImageFormat> {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(ImageFormat::from_extension)
}

/// Check if a byte slice is a raster image (not SVG)
pub fn is_raster_image(data: &[u8]) -> bool {
    image::guess_format(data).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_format_from_extension() {
        assert_eq!(ImageFormat::from_extension("webp"), Some(ImageFormat::WebP));
        assert_eq!(ImageFormat::from_extension("jpg"), Some(ImageFormat::JPEG));
        assert_eq!(ImageFormat::from_extension("JPG"), Some(ImageFormat::JPEG));
        assert_eq!(ImageFormat::from_extension("xyz"), None);
    }

    #[test]
    fn test_generate_srcset() {
        let outputs = vec![
            ImageOutput { format: ImageFormat::WebP, width: 640, path: "/img/640.webp".to_string(), size_bytes: 1000, data: vec![] },
            ImageOutput { format: ImageFormat::WebP, width: 1080, path: "/img/1080.webp".to_string(), size_bytes: 2000, data: vec![] },
        ];
        let srcset = generate_srcset(&outputs);
        assert!(srcset.contains("640w"));
        assert!(srcset.contains("1080w"));
    }

    #[test]
    fn test_generate_img_tag() {
        let tag = generate_img_tag("/img/photo.jpg", "/img/photo.webp 640w, /img/photo-2x.webp 1280w", "Photo", 640, 480);
        assert!(tag.contains("loading=\"lazy\""));
        assert!(tag.contains("decoding=\"async\""));
        assert!(tag.contains("alt=\"Photo\""));
    }

    #[test]
    fn test_process_image_png() {
        // Create a small 100x100 red PNG
        let img = DynamicImage::new_rgb8(100, 100);
        let mut buf = Cursor::new(Vec::new());
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        img.write_with_encoder(encoder).unwrap();
        let png_data = buf.into_inner();

        let opts = ImageOptions {
            formats: vec![ImageFormat::WebP, ImageFormat::JPEG],
            widths: vec![50],
            quality: 80,
            blur_placeholder: true,
            progressive: false,
            strip_metadata: true,
        };

        let result = process_image(&png_data, "test.png", &opts);
        assert!(result.is_ok());
        let processed = result.unwrap();
        assert!(!processed.outputs.is_empty());
        assert!(processed.blur_placeholder.is_some());
        assert!(processed.srcset.contains("w"));
    }
}
