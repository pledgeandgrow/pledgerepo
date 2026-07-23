// Plugin registry — npm-scoped registry for community plugins (#67)
//
// Features:
//   - Search npm for @pledgepack-plugin-* packages
//   - Install plugins via npm/pnpm/yarn
//   - List installed plugins
//   - Plugin discovery without leaving the CLI

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// NPM search result from registry API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmSearchResult {
    pub objects: Vec<NpmSearchObject>,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmSearchObject {
    pub package: NpmPackage,
    pub score: NpmScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmPackage {
    pub name: String,
    pub version: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub links: NpmPackageLinks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmPackageLinks {
    pub npm: String,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmScore {
    #[serde(rename = "final")]
    pub final_score: f64,
    pub quality: f64,
    pub popularity: f64,
    pub maintenance: f64,
}

/// Plugin info displayed to user
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub score: f64,
    pub url: String,
}

/// Search the npm registry for PledgePack plugins
pub fn search_plugins(query: Option<&str>) -> Result<Vec<PluginInfo>> {
    let search_term = match query {
        Some(q) => format!("pledgepack-plugin-{}", q),
        None => "pledgepack-plugin".to_string(),
    };

    let url = format!(
        "https://registry.npmjs.org/-/v1/search?text={}&size=25",
        urlencode(&search_term)
    );

    let response = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .call()?;

    let body = response.into_string()?;
    let result: NpmSearchResult = serde_json::from_str(&body)?;

    let plugins: Vec<PluginInfo> = result
        .objects
        .into_iter()
        .filter(|obj| {
            obj.package.name.contains("pledgepack-plugin")
                || obj.package.keywords.iter().any(|k| k == "pledgepack")
        })
        .map(|obj| PluginInfo {
            name: obj.package.name,
            version: obj.package.version,
            description: obj.package.description,
            score: obj.score.final_score,
            url: obj.package.links.npm,
        })
        .collect();

    Ok(plugins)
}

/// Install a plugin using the detected package manager
pub fn install_plugin(plugin_name: &str, dev: bool) -> Result<()> {
    let (pm, _args) = detect_package_manager();

    let mut cmd_args = vec!["install".to_string()];
    if dev {
        cmd_args.push("--save-dev".to_string());
    } else if pm == "pnpm" {
        cmd_args.push("--save".to_string());
    }
    cmd_args.push(plugin_name.to_string());

    println!("  \x1b[36m→\x1b[0m Installing {} via {}...", plugin_name, pm);

    let status = Command::new(&pm)
        .args(&cmd_args)
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to install {} with {}", plugin_name, pm);
    }

    println!("  \x1b[32m✓\x1b[0m Installed {}", plugin_name);

    // Suggest adding to pledge.config.ts
    println!("\n  \x1b[90m→\x1b[0m Add to pledge.config.ts:");
    println!("  \x1b[90m  plugins: [\"{}\"]\x1b[0m\n", plugin_name);

    Ok(())
}

/// List installed PledgePack plugins from node_modules
pub fn list_installed_plugins(root: &std::path::Path) -> Result<Vec<PluginInfo>> {
    let node_modules = root.join("node_modules");
    if !node_modules.exists() {
        return Ok(Vec::new());
    }

    let mut plugins = Vec::new();

    // Check @pledgepack scope
    let scoped = node_modules.join("@pledgepack");
    if scoped.exists() {
        if let Ok(entries) = std::fs::read_dir(&scoped) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("plugin-") {
                    let full_name = format!("@pledgepack/{}", name);
                    if let Some(info) = read_package_json(&node_modules, &full_name) {
                        plugins.push(info);
                    }
                }
            }
        }
    }

    // Check unscoped pledgepack-plugin-* packages
    if let Ok(entries) = std::fs::read_dir(&node_modules) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("pledgepack-plugin-") {
                if let Some(info) = read_package_json(&node_modules, &name) {
                    plugins.push(info);
                }
            }
        }
    }

    Ok(plugins)
}

/// Read package.json from node_modules to get plugin info
fn read_package_json(node_modules: &std::path::Path, name: &str) -> Option<PluginInfo> {
    let pkg_path = node_modules.join(name).join("package.json");
    let content = std::fs::read_to_string(&pkg_path).ok()?;
    let pkg: serde_json::Value = serde_json::from_str(&content).ok()?;

    Some(PluginInfo {
        name: name.to_string(),
        version: pkg.get("version").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
        description: pkg.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        score: 0.0,
        url: format!("https://www.npmjs.com/package/{}", name),
    })
}

/// Detect which package manager is in use (npm, pnpm, yarn)
fn detect_package_manager() -> (String, Vec<String>) {
    let cwd = std::env::current_dir().unwrap_or_default();

    // Check for lock files
    if cwd.join("pnpm-lock.yaml").exists() {
        return ("pnpm".to_string(), vec![]);
    }
    if cwd.join("yarn.lock").exists() {
        return ("yarn".to_string(), vec![]);
    }
    // Default to npm
    ("npm".to_string(), vec![])
}

/// Format plugin list for terminal output
pub fn format_plugin_list(plugins: &[PluginInfo]) -> String {
    if plugins.is_empty() {
        return "  No plugins found.".to_string();
    }

    let mut out = String::new();
    for p in plugins {
        out.push_str(&format!(
            "  \x1b[36m{}\x1b[0m \x1b[90mv{}\x1b[0m\n    {}\n    \x1b[90m{}\x1b[0m\n\n",
            p.name, p.version, p.description, p.url
        ));
    }
    out
}

/// URL-encode a string for query parameters
fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect()
}
