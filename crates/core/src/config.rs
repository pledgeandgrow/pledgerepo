use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level configuration for the Pledge bundler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PledgeConfig {
    /// Entry points (e.g., ["src/index.tsx"])
    pub entry: Vec<String>,

    /// Output directory (default: ".pledge")
    pub out_dir: PathBuf,

    /// Root directory of the project (default: cwd)
    pub root: PathBuf,

    /// Whether this is a dev or production build
    pub mode: BuildMode,

    /// Framework adapter ("react", "vue", "svelte", "solid", "auto")
    pub framework: Framework,

    /// Path aliases from tsconfig/jsconfig
    pub alias: Vec<PathAlias>,

    /// File extensions to resolve (default: [".tsx", ".ts", ".jsx", ".js", ".json", ".css"])
    pub extensions: Vec<String>,

    /// Whether to enable the persistent filesystem cache
    pub cache: CacheConfig,

    /// Dev server configuration
    pub dev_server: DevServerConfig,

    /// Whether to enable source maps
    pub source_maps: bool,

    /// Resolve aliases (e.g., { "@": "./src" })
    pub resolve_alias: Vec<PathAlias>,

    /// Proxy rules for dev server (path prefix → target URL)
    pub proxy: Vec<ProxyConfig>,

    /// Build profiling (timing per phase)
    pub profile: bool,

    /// Output format ("esm" or "cjs")
    pub output_format: OutputFormat,

    /// Conditions for package.json exports resolution
    pub conditions: Vec<String>,

    /// Environment variable prefixes to inject (default: ["PLEDGE_"])
    pub env_prefix: Vec<String>,

    /// Whether to generate .d.ts for import.meta.env (default: true)
    pub env_dts: bool,

    /// HTML entry point (default: "index.html")
    pub html_entry: Option<String>,

    /// Whether to generate .gz compressed output (default: false)
    pub compress_gzip: bool,

    /// Whether to generate .br compressed output (default: false)
    pub compress_brotli: bool,

    /// Image optimization config
    pub image: ImageConfig,

    /// Edge deployment target ("cloudflare", "vercel", "deno", or null)
    pub edge_target: Option<String>,

    /// Plugin paths (JS/TS plugins to load)
    pub plugins: Vec<String>,

    /// Library mode configuration (for building npm packages)
    pub library: Option<LibraryConfig>,

    /// HTTPS configuration for dev server
    pub https: Option<HttpsConfig>,

    /// Node.js polyfills for browser builds
    pub node_polyfills: bool,

    /// Compile-time constant replacement (define plugin)
    pub define: std::collections::HashMap<String, String>,

    /// Watch mode configuration for production builds
    pub watch: WatchConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BuildMode {
    #[default]
    Development,
    Production,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Framework {
    React,
    Vue,
    Svelte,
    Solid,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathAlias {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable filesystem cache (default: true)
    pub enabled: bool,
    /// Cache directory (default: "node_modules/.pledge-cache")
    pub dir: PathBuf,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: PathBuf::from("node_modules/.pledge-cache"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevServerConfig {
    /// Port (default: 3000)
    pub port: u16,
    /// Host (default: "localhost")
    pub host: String,
    /// Enable HMR (default: true)
    pub hmr: bool,
    /// Open browser on start (default: false)
    pub open: bool,
    /// HTTPS support (default: false)
    pub https: bool,
}

impl Default for DevServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "localhost".to_string(),
            hmr: true,
            open: false,
            https: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Path prefix to match (e.g., "/api")
    pub path: String,
    /// Target URL to proxy to (e.g., "http://localhost:8080")
    pub target: String,
    /// Whether to rewrite the path (remove prefix)
    #[serde(default)]
    pub rewrite: bool,
    /// Additional headers to add to proxied requests
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    /// Enable WebSocket proxying (default: false)
    #[serde(default)]
    pub ws: bool,
}

/// Image optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Enable image optimization (default: false)
    pub enabled: bool,
    /// Default quality (1-100, default: 80)
    pub quality: u32,
    /// Enable WebP conversion (default: true when enabled)
    pub webp: bool,
    /// Enable AVIF conversion (default: false)
    pub avif: bool,
    /// Max width in pixels (default: 1920)
    pub max_width: u32,
    /// Max height in pixels (default: 1080)
    pub max_height: u32,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            quality: 80,
            webp: true,
            avif: false,
            max_width: 1920,
            max_height: 1080,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Esm,
    Cjs,
    Iife,
}

/// Library mode configuration for building npm packages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConfig {
    /// Entry point for the library
    pub entry: String,
    /// Output formats (esm, cjs, umd)
    pub formats: Vec<OutputFormat>,
    /// Global variable name for UMD/IIFE
    pub name: Option<String>,
    /// External dependencies (not bundled)
    pub external: Vec<String>,
    /// Generate TypeScript declaration files
    pub declarations: bool,
}

/// HTTPS configuration for dev server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpsConfig {
    /// Path to SSL certificate file
    pub cert: PathBuf,
    /// Path to SSL key file
    pub key: PathBuf,
}

/// Watch mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Enable watch mode
    pub enabled: bool,
    /// Debounce interval in milliseconds
    pub debounce_ms: u64,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            debounce_ms: 100,
        }
    }
}

impl Default for PledgeConfig {
    fn default() -> Self {
        Self {
            entry: vec!["src/index.tsx".to_string()],
            out_dir: PathBuf::from(".pledge"),
            root: PathBuf::from("."),
            mode: BuildMode::Development,
            framework: Framework::Auto,
            alias: vec![],
            extensions: vec![
                ".tsx".to_string(),
                ".ts".to_string(),
                ".jsx".to_string(),
                ".js".to_string(),
                ".mjs".to_string(),
                ".json".to_string(),
                ".css".to_string(),
            ],
            cache: CacheConfig::default(),
            dev_server: DevServerConfig::default(),
            source_maps: true,
            resolve_alias: vec![],
            proxy: vec![],
            profile: false,
            output_format: OutputFormat::Esm,
            conditions: vec!["browser".to_string(), "import".to_string()],
            env_prefix: vec!["PLEDGE_".to_string()],
            env_dts: true,
            html_entry: None,
            compress_gzip: false,
            compress_brotli: false,
            image: ImageConfig::default(),
            edge_target: None,
            plugins: vec![],
            library: None,
            https: None,
            node_polyfills: false,
            define: std::collections::HashMap::new(),
            watch: WatchConfig::default(),
        }
    }
}

impl PledgeConfig {
    /// Load config from pledge.config.ts, pledge.config.js, pledge.config.json, pledge.json, or defaults
    /// Supports TypeScript config files by extracting the JSON-like config object.
    pub fn load(root: &PathBuf) -> anyhow::Result<Self> {
        // Check for TS/JS config files first (higher priority)
        let ts_candidates = [
            root.join("pledge.config.ts"),
            root.join("pledge.config.js"),
            root.join("pledge.config.mjs"),
        ];

        for path in &ts_candidates {
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                let config = Self::parse_ts_config(&content)?;
                return Ok(config);
            }
        }

        // Fall back to JSON config files
        let json_candidates = [
            root.join("pledge.json"),
            root.join("pledge.config.json"),
            root.join(".pledge.json"),
        ];

        for path in &json_candidates {
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                let config: PledgeConfig = serde_json::from_str(&content)?;
                return Ok(config);
            }
        }

        Ok(Self::default())
    }

    /// Parse a TypeScript/JS config file by extracting the config object.
    /// Handles `export default defineConfig({...})`, `export default {...}`, `module.exports = {...}`.
    pub fn parse_ts_config(content: &str) -> anyhow::Result<Self> {
        // Try to extract the config object from common patterns:
        // 1. export default defineConfig({...})
        // 2. export default {...}
        // 3. module.exports = {...}
        let trimmed = content.trim();

        // Find the first '{' after 'defineConfig' or 'export default' or 'module.exports'
        let config_start = if let Some(pos) = trimmed.find("defineConfig") {
            // Find the opening brace after defineConfig
            trimmed[pos..].find('{').map(|p| pos + p)
        } else if let Some(pos) = trimmed.find("export default") {
            trimmed[pos..].find('{').map(|p| pos + p)
        } else if let Some(pos) = trimmed.find("module.exports") {
            trimmed[pos..].find('{').map(|p| pos + p)
        } else if let Some(pos) = trimmed.find('{') {
            Some(pos)
        } else {
            None
        };

        let start = config_start.ok_or_else(|| {
            anyhow::anyhow!("Could not find config object in pledge.config.ts/js")
        })?;

        // Find matching closing brace (account for nested braces)
        let mut depth = 0i32;
        let mut end = start;
        let bytes = trimmed.as_bytes();
        let mut in_string = false;
        let mut string_char = b' ';
        let mut escaped = false;

        for i in start..bytes.len() {
            let b = bytes[i];
            if escaped {
                escaped = false;
                continue;
            }
            if b == b'\\' {
                escaped = true;
                continue;
            }
            if in_string {
                if b == string_char {
                    in_string = false;
                }
                continue;
            }
            if b == b'"' || b == b'\'' || b == b'`' {
                in_string = true;
                string_char = b;
                continue;
            }
            if b == b'{' {
                depth += 1;
            } else if b == b'}' {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
        }

        let json_str = &trimmed[start..end];

        // Convert TS/JS object syntax to valid JSON:
        // - Remove trailing commas
        // - Convert unquoted keys to quoted keys
        // - Remove JS comments
        let json_str = Self::js_object_to_json(json_str);

        let config: PledgeConfig = serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse pledge.config.ts/js: {}", e))?;

        Ok(config)
    }

    /// Convert JavaScript/TypeScript object literal syntax to valid JSON.
    fn js_object_to_json(input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let bytes = input.as_bytes();
        let mut i = 0;
        let mut in_string = false;
        let mut string_char = b' ';
        let mut escaped = false;
        let mut prev_significant = ' ';

        while i < bytes.len() {
            let b = bytes[i];

            if escaped {
                result.push(b as char);
                escaped = false;
                i += 1;
                continue;
            }

            if b == b'\\' && in_string {
                result.push('\\');
                escaped = true;
                i += 1;
                continue;
            }

            if in_string {
                result.push(b as char);
                if b == string_char {
                    in_string = false;
                }
                i += 1;
                continue;
            }

            // Handle single-line comments
            if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }

            // Handle multi-line comments
            if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
                continue;
            }

            if b == b'"' || b == b'\'' || b == b'`' {
                in_string = true;
                string_char = b;
                if b != b'"' {
                    result.push('"');
                } else {
                    result.push(b as char);
                }
                i += 1;
                continue;
            }

            // Convert single-quoted/backtick strings to double-quoted
            if in_string && b == string_char && string_char != b'"' {
                result.push('"');
                in_string = false;
                i += 1;
                continue;
            }

            // Remove trailing commas before } or ]
            if b == b',' {
                // Look ahead for next non-whitespace
                let mut j = i + 1;
                while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\n' || bytes[j] == b'\r' || bytes[j] == b'\t') {
                    j += 1;
                }
                if j < bytes.len() && (bytes[j] == b'}' || bytes[j] == b']') {
                    i += 1;
                    continue;
                }
            }

            // Quote unquoted keys: word followed by :
            if (b.is_ascii_alphabetic() || b == b'_' || b == b'$') && !in_string {
                let mut j = i;
                while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'$') {
                    j += 1;
                }
                // Check if followed by : (key)
                let mut k = j;
                while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\n' || bytes[k] == b'\r' || bytes[k] == b'\t') {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b':' {
                    // Quote the key
                    result.push('"');
                    result.push_str(&input[i..j]);
                    result.push('"');
                    i = j;
                    continue;
                }
            }

            result.push(b as char);
            if !b.is_ascii_whitespace() {
                prev_significant = b as char;
            }
            i += 1;
        }

        result
    }
}
