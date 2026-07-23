// Security & Integrity features: #81 SRI hashes, #82 CSP generation,
// #83 dependency vulnerability scanning, #84 license compliance checking.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use regex::Regex;
use std::sync::OnceLock;
use tracing::{info, warn};
use sha2::{Sha256, Digest};
use base64::{Engine, engine::general_purpose};

// ── Feature 81: Subresource Integrity (SRI) hashes ────────────────────

/// Generate SRI hash for a file's content.
pub fn generate_sri_hash(content: &[u8]) -> String {
    let hash = simple_sha256(content);
    let b64 = base64_encode(&hash);
    format!("sha256-{}", b64)
}

/// Generate SRI integrity attributes for all script and link tags in HTML.
pub fn inject_sri_into_html(html: &str, out_dir: &Path) -> String {
    let mut result = html.to_string();

    static SCRIPT_RE: OnceLock<Regex> = OnceLock::new();
    let re = SCRIPT_RE.get_or_init(|| {
        Regex::new(r#"<script\s+src="([^"]+)")"#).unwrap()
    });

    for cap in re.captures_iter(html) {
        let src = &cap[1];
        let file_path = out_dir.join(src.trim_start_matches('/'));
        if file_path.is_file() {
            if let Ok(content) = std::fs::read(&file_path) {
                let integrity = generate_sri_hash(&content);
                let old = format!(r#"<script src="{}""#, src);
                let new = format!(r#"<script src="{}" integrity="{}" crossorigin="anonymous""#, src, integrity);
                result = result.replace(&old, &new);
            }
        }
    }

    static LINK_RE: OnceLock<Regex> = OnceLock::new();
    let re = LINK_RE.get_or_init(|| {
        Regex::new(r#"<link\s+[^>]*rel="stylesheet"[^>]*href="([^"]+)")"#).unwrap()
    });

    for cap in re.captures_iter(html) {
        let href = &cap[1];
        let file_path = out_dir.join(href.trim_start_matches('/'));
        if file_path.is_file() {
            if let Ok(content) = std::fs::read(&file_path) {
                let integrity = generate_sri_hash(&content);
                let old = format!(r#"href="{}""#, href);
                let new = format!(r#"href="{}" integrity="{}" crossorigin="anonymous""#, href, integrity);
                result = result.replace(&old, &new);
            }
        }
    }

    result
}

// ── Feature 82: Content Security Policy generation ────────────────────

pub struct CspGenerator {
    script_src: Vec<String>,
    style_src: Vec<String>,
    img_src: Vec<String>,
    font_src: Vec<String>,
    connect_src: Vec<String>,
    inline_script_hashes: Vec<String>,
    inline_style_hashes: Vec<String>,
}

impl CspGenerator {
    pub fn new() -> Self {
        Self {
            script_src: vec!["'self'".to_string()],
            style_src: vec!["'self'".to_string()],
            img_src: vec!["'self'".to_string(), "data:".to_string()],
            font_src: vec!["'self'".to_string()],
            connect_src: vec!["'self'".to_string()],
            inline_script_hashes: Vec::new(),
            inline_style_hashes: Vec::new(),
        }
    }

    pub fn analyze_html(&mut self, html: &str) {
        static INLINE_SCRIPT_RE: OnceLock<Regex> = OnceLock::new();
        let re = INLINE_SCRIPT_RE.get_or_init(|| {
            Regex::new(r"<script[^>]*>([\s\S]*?)</script>").unwrap()
        });

        for cap in re.captures_iter(html) {
            // Skip scripts with src= attribute (external scripts)
            let full_match = cap.get(0).map(|m| m.as_str()).unwrap_or("");
            if full_match.contains("src=") {
                continue;
            }
            let inline_code = cap[1].trim();
            if !inline_code.is_empty() {
                let hash = generate_sri_hash(inline_code.as_bytes());
                self.inline_script_hashes.push(format!("'{}'", hash));
            }
        }

        static INLINE_STYLE_RE: OnceLock<Regex> = OnceLock::new();
        let re = INLINE_STYLE_RE.get_or_init(|| {
            Regex::new(r"<style[^>]*>([\s\S]*?)</style>").unwrap()
        });

        for cap in re.captures_iter(html) {
            let inline_css = cap[1].trim();
            if !inline_css.is_empty() {
                let hash = generate_sri_hash(inline_css.as_bytes());
                self.inline_style_hashes.push(format!("'{}'", hash));
            }
        }
    }

    pub fn add_script_src(&mut self, src: &str) {
        self.script_src.push(src.to_string());
    }

    pub fn add_style_src(&mut self, src: &str) {
        self.style_src.push(src.to_string());
    }

    pub fn generate(&self) -> String {
        let mut directives = Vec::new();
        directives.push("default-src 'self'".to_string());

        let mut scripts = self.script_src.clone();
        scripts.extend(self.inline_script_hashes.clone());
        directives.push(format!("script-src {}", scripts.join(" ")));

        let mut styles = self.style_src.clone();
        styles.extend(self.inline_style_hashes.clone());
        directives.push(format!("style-src {}", styles.join(" ")));

        directives.push(format!("img-src {}", self.img_src.join(" ")));
        directives.push(format!("font-src {}", self.font_src.join(" ")));
        directives.push(format!("connect-src {}", self.connect_src.join(" ")));
        directives.push("object-src 'none'".to_string());
        directives.push("base-uri 'self'".to_string());

        directives.join("; ")
    }

    pub fn generate_headers_file(&self, _out_dir: &Path) -> String {
        let csp = self.generate();
        format!(
            "/*\n  Content-Security-Policy: {}\n  X-Content-Type-Options: nosniff\n  X-Frame-Options: DENY\n  Referrer-Policy: strict-origin-when-cross-origin\n",
            csp
        )
    }
}

impl Default for CspGenerator {
    fn default() -> Self { Self::new() }
}

pub fn generate_csp_from_build(html: &str, out_dir: &Path) -> String {
    let mut csp_gen = CspGenerator::new();
    csp_gen.analyze_html(html);
    let headers = csp_gen.generate_headers_file(out_dir);

    let headers_path = out_dir.join("_headers");
    if let Err(e) = std::fs::write(&headers_path, &headers) {
        warn!("Failed to write _headers file: {}", e);
    } else {
        info!("Generated CSP _headers file at {}", headers_path.display());
    }

    csp_gen.generate()
}

// ── Feature 83: Dependency vulnerability scanning ─────────────────────

#[derive(Debug, Clone)]
pub struct Vulnerability {
    pub package: String,
    pub version: String,
    pub severity: VulnerabilitySeverity,
    pub title: String,
    pub cve: Option<String>,
    pub url: Option<String>,
    pub patch_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VulnerabilitySeverity {
    Critical, High, Medium, Low, Info,
}

impl VulnerabilitySeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL", Self::High => "HIGH",
            Self::Medium => "MEDIUM", Self::Low => "LOW", Self::Info => "INFO",
        }
    }
    pub fn color(&self) -> &'static str {
        match self {
            Self::Critical | Self::High => "\x1b[31m",
            Self::Medium => "\x1b[33m", Self::Low => "\x1b[36m", Self::Info => "\x1b[90m",
        }
    }
}

pub fn scan_vulnerabilities(root: &Path) -> Vec<Vulnerability> {
    let mut vulns = Vec::new();
    let pkg_json = root.join("package.json");
    if !pkg_json.is_file() { return vulns; }

    let content = match std::fs::read_to_string(&pkg_json) {
        Ok(c) => c, Err(_) => return vulns,
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j, Err(_) => return vulns,
    };

    let mut deps = HashMap::new();
    for key in &["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(obj) = json.get(key).and_then(|v| v.as_object()) {
            for (name, version) in obj {
                deps.insert(name.clone(), version.as_str().unwrap_or("*").to_string());
            }
        }
    }

    for (name, version) in &deps {
        if let Some(known) = check_advisory_database(name, version) {
            vulns.extend(known);
        }
    }

    if vulns.is_empty() {
        info!("No known vulnerabilities found in {} packages", deps.len());
    } else {
        warn!("Found {} vulnerabilities in {} packages", vulns.len(), deps.len());
    }
    vulns
}

fn check_advisory_database(package: &str, version: &str) -> Option<Vec<Vulnerability>> {
    let advisories: &[(&str, &str, &str, VulnerabilitySeverity, &str, &str, &str)] = &[
        ("lodash", "<4.17.21", "CVE-2021-23337", VulnerabilitySeverity::High, "Command injection via template", "https://npmjs.com/advisories/1673", "4.17.21"),
        ("minimist", "<1.2.6", "CVE-2022-21222", VulnerabilitySeverity::Medium, "Prototype pollution", "https://npmjs.com/advisories/2392", "1.2.6"),
        ("axios", "<0.21.1", "CVE-2021-3749", VulnerabilitySeverity::High, "SSRF vulnerability", "https://npmjs.com/advisories/1594", "0.21.1"),
        ("ws", "<7.4.6", "CVE-2021-32615", VulnerabilitySeverity::Medium, "DoS via large WebSocket message", "https://npmjs.com/advisories/1748", "7.4.6"),
        ("node-forge", "<1.3.0", "CVE-2022-24772", VulnerabilitySeverity::High, "Prototype pollution", "https://npmjs.com/advisories/2501", "1.3.0"),
        ("minimatch", "<3.0.5", "CVE-2022-3517", VulnerabilitySeverity::High, "ReDoS via pattern", "https://npmjs.com/advisories/2513", "3.0.5"),
        ("json-schema", "<0.4.0", "CVE-2021-27787", VulnerabilitySeverity::High, "Prototype pollution", "https://npmjs.com/advisories/1671", "0.4.0"),
    ];

    let mut found = Vec::new();
    for (pkg, affected, cve, severity, title, url, patch) in advisories {
        if *pkg == package && version_matches(version, affected) {
            found.push(Vulnerability {
                package: package.to_string(), version: version.to_string(),
                severity: *severity, title: title.to_string(),
                cve: Some(cve.to_string()), url: Some(url.to_string()),
                patch_version: Some(patch.to_string()),
            });
        }
    }
    if found.is_empty() { None } else { Some(found) }
}

fn version_matches(version: &str, range: &str) -> bool {
    if range.starts_with('<') {
        let target = range.trim_start_matches('<').trim();
        return semver_less_than(version, target);
    }
    false
}

fn semver_less_than(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.split('.').filter_map(|p| p.split('-').next().and_then(|n| n.parse().ok())).collect()
    };
    let va = parse(a);
    let vb = parse(b);
    for i in 0..va.len().min(vb.len()) {
        if va[i] < vb[i] { return true; }
        if va[i] > vb[i] { return false; }
    }
    va.len() < vb.len()
}

pub fn format_vulnerability_report(vulns: &[Vulnerability]) -> String {
    if vulns.is_empty() {
        return "  \x1b[32m✓\x1b[0m No known vulnerabilities found".to_string();
    }
    let mut out = format!("  \x1b[33m⚠\x1b[0m Found {} vulnerabilities\n\n", vulns.len());
    for v in vulns {
        out.push_str(&format!("  {}{}  {} {}\x1b[0m\n", v.severity.color(), v.severity.label(), v.package, v.version));
        out.push_str(&format!("    {}\n", v.title));
        if let Some(ref cve) = v.cve { out.push_str(&format!("    CVE: {}\n", cve)); }
        if let Some(ref url) = v.url { out.push_str(&format!("    URL: {}\n", url)); }
        out.push('\n');
    }
    out
}

// ── Feature 84: License compliance checking ───────────────────────────

#[derive(Debug, Clone)]
pub struct LicenseInfo {
    pub package: String,
    pub version: String,
    pub license: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LicenseCheckResult {
    pub compliant: bool,
    pub licenses: Vec<LicenseInfo>,
    pub violations: Vec<LicenseViolation>,
}

#[derive(Debug, Clone)]
pub struct LicenseViolation {
    pub package: String,
    pub license: String,
    pub reason: String,
}

pub fn scan_licenses(root: &Path) -> Vec<LicenseInfo> {
    let mut licenses = Vec::new();
    let nm = root.join("node_modules");
    if !nm.is_dir() { return licenses; }
    scan_node_modules(&nm, &mut licenses);
    licenses
}

fn scan_node_modules(dir: &Path, licenses: &mut Vec<LicenseInfo>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('@') {
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    for sub in sub_entries.flatten() {
                        let sub_path = sub.path();
                        if sub_path.is_dir() {
                            if let Some(info) = read_package_license(&sub_path) {
                                licenses.push(info);
                            }
                        }
                    }
                }
                continue;
            }
            if let Some(info) = read_package_license(&path) {
                licenses.push(info);
            }
        }
    }
}

fn read_package_license(pkg_dir: &Path) -> Option<LicenseInfo> {
    let pkg_json = pkg_dir.join("package.json");
    if !pkg_json.is_file() { return None; }
    let content = std::fs::read_to_string(&pkg_json).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    Some(LicenseInfo {
        package: json.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        version: json.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0").to_string(),
        license: json.get("license").and_then(|v| {
            v.as_str().map(|s| s.to_string())
                .or_else(|| v.as_object().and_then(|o| o.get("type")).and_then(|t| t.as_str()).map(|s| s.to_string()))
        }).unwrap_or_else(|| "UNKNOWN".to_string()),
        path: pkg_dir.to_path_buf(),
    })
}

pub fn check_license_compliance(
    licenses: &[LicenseInfo], whitelist: &[&str], blacklist: &[&str],
) -> LicenseCheckResult {
    let mut violations = Vec::new();
    for info in licenses {
        let license_upper = info.license.to_uppercase();
        for bl in blacklist {
            if license_upper.contains(&bl.to_uppercase()) {
                violations.push(LicenseViolation {
                    package: info.package.clone(), license: info.license.clone(),
                    reason: format!("License '{}' is blacklisted", info.license),
                });
                break;
            }
        }
        if !whitelist.is_empty() {
            if info.license == "UNKNOWN" {
                violations.push(LicenseViolation {
                    package: info.package.clone(), license: info.license.clone(),
                    reason: "License is UNKNOWN — not in whitelist".to_string(),
                });
            } else if !whitelist.iter().any(|w| license_upper.contains(&w.to_uppercase())) {
                violations.push(LicenseViolation {
                    package: info.package.clone(), license: info.license.clone(),
                    reason: format!("License '{}' is not in whitelist", info.license),
                });
            }
        }
    }
    let compliant = violations.is_empty();
    if compliant { info!("License check: all {} packages compliant", licenses.len()); }
    else { warn!("License check: {} violations out of {} packages", violations.len(), licenses.len()); }
    LicenseCheckResult { compliant, licenses: licenses.to_vec(), violations }
}

pub fn format_license_report(result: &LicenseCheckResult) -> String {
    let mut out = String::new();
    if result.compliant {
        out.push_str(&format!("  \x1b[32m✓\x1b[0m All {} packages have compliant licenses\n", result.licenses.len()));
    } else {
        out.push_str(&format!("  \x1b[33m⚠\x1b[0m {} license violations found\n\n", result.violations.len()));
        for v in &result.violations {
            out.push_str(&format!("  \x1b[31m✗\x1b[0m {} — {}\n", v.package, v.reason));
        }
    }
    let mut by_type: HashMap<String, usize> = HashMap::new();
    for info in &result.licenses {
        *by_type.entry(info.license.clone()).or_default() += 1;
    }
    out.push_str("\n  License summary:\n");
    let mut sorted: Vec<_> = by_type.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    for (license, count) in sorted {
        out.push_str(&format!("    {} ({}): {} packages\n", license, if license == "UNKNOWN" { "\x1b[33m" } else { "\x1b[32m" }, count));
    }
    out
}

// ── Utility ───────────────────────────────────────────────────────────

fn simple_sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

fn base64_encode(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sri_hash() {
        let hash = generate_sri_hash(b"test content");
        assert!(hash.starts_with("sha256-"));
    }

    #[test]
    fn test_csp_generation() {
        let mut csp_gen = CspGenerator::new();
        csp_gen.analyze_html(r#"<script>console.log("hello")</script>"#);
        let csp = csp_gen.generate();
        assert!(csp.contains("script-src"));
        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("object-src 'none'"));
    }

    #[test]
    fn test_version_matches() {
        assert!(version_matches("1.2.3", "<1.3.0"));
        assert!(!version_matches("1.3.0", "<1.3.0"));
    }

    #[test]
    fn test_advisory_database() {
        assert!(check_advisory_database("lodash", "4.17.20").is_some());
        assert!(check_advisory_database("lodash", "4.17.21").is_none());
    }

    #[test]
    fn test_license_check() {
        let licenses = vec![
            LicenseInfo { package: "react".into(), version: "18.0.0".into(), license: "MIT".into(), path: PathBuf::from("nm/react") },
            LicenseInfo { package: "gpl-pkg".into(), version: "1.0.0".into(), license: "GPL-3.0".into(), path: PathBuf::from("nm/gpl") },
        ];
        let result = check_license_compliance(&licenses, &["MIT", "Apache-2.0"], &["GPL"]);
        assert!(!result.compliant);
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(base64_encode(b"hi"), "aGk=");
    }
}
