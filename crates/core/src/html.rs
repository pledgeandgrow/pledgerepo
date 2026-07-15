// HTML processing — index.html as entry point
//
// Parses index.html to extract:
//   - <script type="module" src="..."> → entry points
//   - <link rel="stylesheet" href="..."> → CSS dependencies
//   - <meta> tags → metadata
//   - <title> → page title
//
// During build, processes the HTML to:
//   - Replace script src with hashed output filenames
//   - Inject CSS <link> tags for extracted styles
//   - Add preload/prefetch hints
//   - Inject meta tags from config

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;

/// Parsed HTML entry point information
#[derive(Debug, Clone)]
pub struct HtmlEntry {
    /// Script src paths found in the HTML (e.g., ["/src/index.tsx"])
    pub scripts: Vec<String>,
    /// CSS stylesheet links found in the HTML
    pub stylesheets: Vec<String>,
    /// Module preload scripts
    pub module_preloads: Vec<String>,
    /// Page title
    pub title: Option<String>,
    /// Meta tags (name → content)
    pub meta: HashMap<String, String>,
    /// Raw HTML content
    pub html: String,
}

/// Process an HTML file as an entry point
pub fn process_html(html_path: &PathBuf) -> Result<HtmlEntry> {
    let html = std::fs::read_to_string(html_path)?;

    let scripts = extract_script_srcs(&html);
    let stylesheets = extract_stylesheet_links(&html);
    let module_preloads = extract_module_preloads(&html);
    let title = extract_title(&html);
    let meta = extract_meta_tags(&html);

    info!(
        "HTML entry: {} scripts, {} stylesheets, {} preloads",
        scripts.len(),
        stylesheets.len(),
        module_preloads.len()
    );

    Ok(HtmlEntry {
        scripts,
        stylesheets,
        module_preloads,
        title,
        meta,
        html,
    })
}

/// Generate production HTML with hashed asset references
pub fn generate_production_html(
    template_html: &str,
    entry_scripts: &[(String, String)], // (original_src, hashed_filename)
    css_files: &[String],
    additional_meta: &HashMap<String, String>,
) -> String {
    let mut html = template_html.to_string();

    // Replace script src references with hashed versions
    for (original, hashed) in entry_scripts {
        let old_src = format!(r#"src="{}""#, original);
        let new_src = format!(r#"src="/{}""#, hashed);
        html = html.replace(&old_src, &new_src);
    }

    // Inject CSS <link> tags before </head>
    let css_links: String = css_files
        .iter()
        .map(|css| format!(r#"    <link rel="stylesheet" href="/{}" />"#, css))
        .collect::<Vec<_>>()
        .join("\n");

    if !css_links.is_empty() {
        html = html.replace("</head>", &format!("{}\n</head>", css_links));
    }

    // Inject additional meta tags
    let meta_tags: String = additional_meta
        .iter()
        .map(|(name, content)| format!(r#"    <meta name="{}" content="{}" />"#, name, content))
        .collect::<Vec<_>>()
        .join("\n");

    if !meta_tags.is_empty() {
        html = html.replace("</head>", &format!("{}\n</head>", meta_tags));
    }

    html
}

/// Generate a default index.html if none exists
pub fn generate_default_html(entry: &str, title: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{title}</title>
</head>
<body>
    <div id="root"></div>
    <script type="module" src="/{entry}"></script>
</body>
</html>"#,
        title = title,
        entry = entry,
    )
}

/// Extract <script type="module" src="..."> src paths
fn extract_script_srcs(html: &str) -> Vec<String> {
    let mut scripts = Vec::new();
    let mut search_pos = 0;

    while let Some(pos) = html[search_pos..].find("<script") {
        let abs_pos = search_pos + pos;
        let rest = &html[abs_pos..];

        if let Some(end) = rest.find('>') {
            let tag = &rest[..end];

            // Check for type="module" and src="..."
            if tag.contains(r#"type="module""#) || tag.contains("type='module'") {
                if let Some(src_start) = tag.find(r#"src=""#) {
                    let src_rest = &tag[src_start + 5..];
                    if let Some(src_end) = src_rest.find('"') {
                        scripts.push(src_rest[..src_end].to_string());
                    }
                } else if let Some(src_start) = tag.find("src='") {
                    let src_rest = &tag[src_start + 5..];
                    if let Some(src_end) = src_rest.find('\'') {
                        scripts.push(src_rest[..src_end].to_string());
                    }
                }
            }
        }

        search_pos = abs_pos + 1;
    }

    scripts
}

/// Extract <link rel="stylesheet" href="..."> href paths
fn extract_stylesheet_links(html: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut search_pos = 0;

    while let Some(pos) = html[search_pos..].find("<link") {
        let abs_pos = search_pos + pos;
        let rest = &html[abs_pos..];

        if let Some(end) = rest.find('>') {
            let tag = &rest[..end];

            if tag.contains(r#"rel="stylesheet""#) || tag.contains("rel='stylesheet'") {
                if let Some(href_start) = tag.find(r#"href=""#) {
                    let href_rest = &tag[href_start + 6..];
                    if let Some(href_end) = href_rest.find('"') {
                        links.push(href_rest[..href_end].to_string());
                    }
                } else if let Some(href_start) = tag.find("href='") {
                    let href_rest = &tag[href_start + 6..];
                    if let Some(href_end) = href_rest.find('\'') {
                        links.push(href_rest[..href_end].to_string());
                    }
                }
            }
        }

        search_pos = abs_pos + 1;
    }

    links
}

/// Extract <link rel="modulepreload" href="..."> href paths
fn extract_module_preloads(html: &str) -> Vec<String> {
    let mut preloads = Vec::new();
    let mut search_pos = 0;

    while let Some(pos) = html[search_pos..].find("<link") {
        let abs_pos = search_pos + pos;
        let rest = &html[abs_pos..];

        if let Some(end) = rest.find('>') {
            let tag = &rest[..end];

            if tag.contains("modulepreload") {
                if let Some(href_start) = tag.find(r#"href=""#) {
                    let href_rest = &tag[href_start + 6..];
                    if let Some(href_end) = href_rest.find('"') {
                        preloads.push(href_rest[..href_end].to_string());
                    }
                }
            }
        }

        search_pos = abs_pos + 1;
    }

    preloads
}

/// Extract <title>...</title>
fn extract_title(html: &str) -> Option<String> {
    if let Some(start) = html.find("<title>") {
        if let Some(end) = html[start..].find("</title>") {
            return Some(html[start + 7..start + end].trim().to_string());
        }
    }
    None
}

/// Extract <meta name="..." content="..."> tags
fn extract_meta_tags(html: &str) -> HashMap<String, String> {
    let mut meta = HashMap::new();
    let mut search_pos = 0;

    while let Some(pos) = html[search_pos..].find("<meta") {
        let abs_pos = search_pos + pos;
        let rest = &html[abs_pos..];

        if let Some(end) = rest.find('>') {
            let tag = &rest[..end];

            let name = extract_attr(tag, "name");
            let content = extract_attr(tag, "content");

            if let (Some(n), Some(c)) = (name, content) {
                meta.insert(n, c);
            }
        }

        search_pos = abs_pos + 1;
    }

    meta
}

/// Extract an attribute value from an HTML tag
fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!(r#"{}=""#, attr);
    if let Some(start) = tag.find(&pattern) {
        let rest = &tag[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }

    let pattern = format!("{}='", attr);
    if let Some(start) = tag.find(&pattern) {
        let rest = &tag[start + pattern.len()..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }

    None
}

/// Minify HTML for production — removes comments, whitespace, redundant attributes
pub fn minify_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_comment = false;
    let mut prev_char = '\0';
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        if in_comment {
            // Check for end of comment -->
            if c == '-' && chars.peek() == Some(&'-') {
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    in_comment = false;
                }
            }
            continue;
        }

        if c == '<' {
            // Check for HTML comment
            if html[result.len().saturating_sub(0)..].ends_with("<")
                && chars.peek() == Some(&'!')
            {
                // Look ahead for <!--
                let mut lookahead = chars.clone();
                lookahead.next(); // !
                if lookahead.next() == Some('-') && lookahead.next() == Some('-') {
                    chars.next(); // consume !
                    chars.next(); // consume -
                    chars.next(); // consume -
                    in_comment = true;
                    continue;
                }
            }
            in_tag = true;
            result.push(c);
            continue;
        }

        if c == '>' && in_tag {
            in_tag = false;
            result.push(c);
            prev_char = '>';
            continue;
        }

        if in_tag {
            result.push(c);
            continue;
        }

        // Outside tags: collapse whitespace
        if c.is_whitespace() {
            if prev_char != ' ' && prev_char != '\n' && prev_char != '\t' && prev_char != '\r' {
                result.push(' ');
                prev_char = ' ';
            }
        } else {
            result.push(c);
            prev_char = c;
        }
    }

    // Remove spaces between tags
    result = result.replace("> <", "><");

    // Remove trailing whitespace before </tag>
    result = result.replace("  </", "</");
    result = result.replace(" </", "</");

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_script_srcs() {
        let html = r#"
            <script type="module" src="/src/index.tsx"></script>
            <script src="legacy.js"></script>
            <script type="module" src="/src/app.ts"></script>
        "#;
        let scripts = extract_script_srcs(html);
        assert_eq!(scripts, vec!["/src/index.tsx", "/src/app.ts"]);
    }

    #[test]
    fn test_extract_title() {
        let html = "<html><head><title>My App</title></head></html>";
        assert_eq!(extract_title(html), Some("My App".to_string()));
    }

    #[test]
    fn test_extract_meta_tags() {
        let html = r#"<meta name="viewport" content="width=device-width" />"#;
        let meta = extract_meta_tags(html);
        assert_eq!(meta.get("viewport"), Some(&"width=device-width".to_string()));
    }
}
