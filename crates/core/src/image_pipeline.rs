// Image pipeline — Sharp/Squoosh integration, responsive srcset generation.
//
// Features:
//   - Automatic image optimization (WebP, AVIF, JPEG, PNG)
//   - Responsive srcset generation for different viewport sizes
//   - Lazy loading attributes
//   - Blur placeholder generation
//   - Format conversion

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
}

/// A single output variant
#[derive(Debug, Clone)]
pub struct ImageOutput {
    pub format: ImageFormat,
    pub width: u32,
    pub path: String,
    pub size_bytes: u64,
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
            ImageOutput { format: ImageFormat::WebP, width: 640, path: "/img/640.webp".to_string(), size_bytes: 1000 },
            ImageOutput { format: ImageFormat::WebP, width: 1080, path: "/img/1080.webp".to_string(), size_bytes: 2000 },
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
}
