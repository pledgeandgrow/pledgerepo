// Font optimization — auto-subsetting, font-display:swap, preload hints.
//
// Features:
//   - Automatic font subsetting (Latin, Latin Extended, Cyrillic, etc.)
//   - font-display: swap injection
//   - Preload hints for critical fonts
//   - WOFF2 optimization
//   - @font-face generation

use std::path::Path;

/// Font format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFormat {
    WOFF2,
    WOFF,
    TTF,
    OTF,
    EOT,
}

impl FontFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "woff2" => Some(Self::WOFF2),
            "woff" => Some(Self::WOFF),
            "ttf" => Some(Self::TTF),
            "otf" => Some(Self::OTF),
            "eot" => Some(Self::EOT),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::WOFF2 => "woff2",
            Self::WOFF => "woff",
            Self::TTF => "ttf",
            Self::OTF => "otf",
            Self::EOT => "eot",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::WOFF2 => "font/woff2",
            Self::WOFF => "font/woff",
            Self::TTF => "font/ttf",
            Self::OTF => "font/otf",
            Self::EOT => "application/vnd.ms-fontobject",
        }
    }
}

/// Font subset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSubset {
    Latin,
    LatinExtended,
    Cyrillic,
    Greek,
    Vietnamese,
    Full,
}

impl FontSubset {
    pub fn unicode_range(&self) -> &'static str {
        match self {
            Self::Latin => "U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD",
            Self::LatinExtended => "U+0100-024F, U+0259, U+1E00-1EFF",
            Self::Cyrillic => "U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116",
            Self::Greek => "U+0370-03FF",
            Self::Vietnamese => "U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+1EA0-1EF9, U+20AB",
            Self::Full => "U+0000-FFFF",
        }
    }
}

/// Font face declaration
#[derive(Debug, Clone)]
pub struct FontFace {
    pub family: String,
    pub weight: Option<u32>,
    pub style: Option<String>,
    pub src: Vec<FontSrc>,
    pub display: FontDisplay,
    pub subset: Option<FontSubset>,
    pub preload: bool,
}

/// Font source
#[derive(Debug, Clone)]
pub struct FontSrc {
    pub url: String,
    pub format: FontFormat,
}

/// font-display strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontDisplay {
    Auto,
    Block,
    Swap,
    Fallback,
    Optional,
}

impl Default for FontDisplay {
    fn default() -> Self {
        Self::Swap
    }
}

impl FontDisplay {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Block => "block",
            Self::Swap => "swap",
            Self::Fallback => "fallback",
            Self::Optional => "optional",
        }
    }
}

/// Generate @font-face CSS
pub fn generate_font_face(face: &FontFace) -> String {
    let mut css = String::from("@font-face {\n");

    css.push_str(&format!("  font-family: '{}';\n", face.family));

    if let Some(weight) = face.weight {
        css.push_str(&format!("  font-weight: {};\n", weight));
    }

    if let Some(ref style) = face.style {
        css.push_str(&format!("  font-style: {};\n", style));
    }

    // Source URLs
    let src_entries: Vec<String> = face
        .src
        .iter()
        .map(|s| format!("url('{}') format('{}')", s.url, s.format.mime_type()))
        .collect();
    css.push_str(&format!("  src: {};\n", src_entries.join(", ")));

    // font-display
    css.push_str(&format!("  font-display: {};\n", face.display.as_str()));

    // Unicode range for subset
    if let Some(subset) = face.subset {
        css.push_str(&format!("  unicode-range: {};\n", subset.unicode_range()));
    }

    css.push_str("}");
    css
}

/// Generate preload link tag for a font
pub fn generate_preload_tag(url: &str, format: FontFormat) -> String {
    format!(
        r#"<link rel="preload" href="{}" as="font" type="{}" crossorigin />"#,
        url,
        format.mime_type()
    )
}

/// Optimize font CSS — inject font-display: swap if missing
pub fn optimize_font_css(css: &str) -> String {
    let mut result = css.to_string();

    // Check if @font-face blocks are missing font-display
    // Simple heuristic: add font-display: swap after font-family if not present
    let mut modified = String::new();
    let mut in_font_face = false;
    let mut has_display = false;

    for line in result.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("@font-face") {
            in_font_face = true;
            has_display = false;
        }

        if in_font_face && trimmed.contains("font-display") {
            has_display = true;
        }

        if in_font_face && trimmed == "}" && !has_display {
            modified.push_str("  font-display: swap;\n");
            in_font_face = false;
        }

        modified.push_str(line);
        modified.push('\n');
    }

    result = modified;
    result
}

/// Check if a file is a font file
pub fn is_font(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(FontFormat::from_extension)
        .is_some()
}

/// Get font format from file path
pub fn get_font_format(path: &Path) -> Option<FontFormat> {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(FontFormat::from_extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_format_from_extension() {
        assert_eq!(FontFormat::from_extension("woff2"), Some(FontFormat::WOFF2));
        assert_eq!(FontFormat::from_extension("ttf"), Some(FontFormat::TTF));
        assert_eq!(FontFormat::from_extension("xyz"), None);
    }

    #[test]
    fn test_generate_font_face() {
        let face = FontFace {
            family: "Inter".to_string(),
            weight: Some(400),
            style: Some("normal".to_string()),
            src: vec![FontSrc {
                url: "/fonts/inter.woff2".to_string(),
                format: FontFormat::WOFF2,
            }],
            display: FontDisplay::Swap,
            subset: Some(FontSubset::Latin),
            preload: true,
        };

        let css = generate_font_face(&face);
        assert!(css.contains("font-family: 'Inter'"));
        assert!(css.contains("font-weight: 400"));
        assert!(css.contains("font-display: swap"));
        assert!(css.contains("unicode-range"));
    }

    #[test]
    fn test_generate_preload_tag() {
        let tag = generate_preload_tag("/fonts/inter.woff2", FontFormat::WOFF2);
        assert!(tag.contains("rel=\"preload\""));
        assert!(tag.contains("as=\"font\""));
        assert!(tag.contains("font/woff2"));
        assert!(tag.contains("crossorigin"));
    }

    #[test]
    fn test_optimize_font_css_adds_display() {
        let css = "@font-face {\n  font-family: 'Test';\n  src: url('test.woff2');\n}";
        let optimized = optimize_font_css(css);
        assert!(optimized.contains("font-display: swap"));
    }
}
