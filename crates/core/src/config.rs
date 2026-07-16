use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level configuration for the Pledge bundler.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PledgeConfig {
    /// Entry points (e.g., ["src/index.tsx"])
    #[serde(default)]
    pub entry: Vec<String>,

    /// Output directory (default: ".pledge")
    #[serde(default = "default_out_dir")]
    pub out_dir: PathBuf,

    /// Root directory of the project (default: cwd)
    #[serde(default)]
    pub root: PathBuf,

    /// Whether this is a dev or production build
    #[serde(default)]
    pub mode: BuildMode,

    /// Framework adapter ("react", "vue", "svelte", "solid", "auto")
    #[serde(default)]
    pub framework: Framework,

    /// Path aliases from tsconfig/jsconfig
    #[serde(default)]
    pub alias: Vec<PathAlias>,

    /// File extensions to resolve (default: [".tsx", ".ts", ".jsx", ".js", ".json", ".css"])
    #[serde(default)]
    pub extensions: Vec<String>,

    /// Whether to enable the persistent filesystem cache
    #[serde(default)]
    pub cache: CacheConfig,

    /// Dev server configuration
    #[serde(default)]
    pub dev_server: DevServerConfig,

    /// Whether to enable source maps
    #[serde(default)]
    pub source_maps: bool,

    /// Resolve aliases (e.g., { "@": "./src" })
    #[serde(default)]
    pub resolve_alias: Vec<PathAlias>,

    /// Proxy rules for dev server (path prefix → target URL)
    #[serde(default)]
    pub proxy: Vec<ProxyConfig>,

    /// Build profiling (timing per phase)
    #[serde(default)]
    pub profile: bool,

    /// Output format ("esm" or "cjs")
    #[serde(default)]
    pub output_format: OutputFormat,

    /// Conditions for package.json exports resolution
    #[serde(default)]
    pub conditions: Vec<String>,

    /// Environment variable prefixes to inject (default: ["PLEDGE_"])
    #[serde(default)]
    pub env_prefix: Vec<String>,

    /// Whether to generate .d.ts for import.meta.env (default: true)
    #[serde(default)]
    pub env_dts: bool,

    /// HTML entry point (default: "index.html")
    #[serde(default)]
    pub html_entry: Option<String>,

    /// Whether to generate .gz compressed output (default: false)
    #[serde(default)]
    pub compress_gzip: bool,

    /// Whether to generate .br compressed output (default: false)
    #[serde(default)]
    pub compress_brotli: bool,

    /// Image optimization config
    #[serde(default)]
    pub image: ImageConfig,

    /// Edge deployment target ("cloudflare", "vercel", "deno", or null)
    #[serde(default)]
    pub edge_target: Option<String>,

    /// Plugin paths (JS/TS plugins to load)
    #[serde(default)]
    pub plugins: Vec<String>,

    /// Library mode configuration (for building npm packages)
    #[serde(default)]
    pub library: Option<LibraryConfig>,

    /// HTTPS configuration for dev server
    #[serde(default)]
    pub https: Option<HttpsConfig>,

    /// Node.js polyfills for browser builds
    #[serde(default)]
    pub node_polyfills: bool,

    /// Compile-time constant replacement (define plugin)
    #[serde(default)]
    pub define: std::collections::HashMap<String, String>,

    /// Watch mode configuration for production builds
    #[serde(default)]
    pub watch: WatchConfig,

    /// Build configuration for chunk splitting, source maps, asset inlining
    #[serde(default)]
    pub build: BuildConfig,

    /// Test configuration (Vitest-compatible)
    #[serde(default)]
    pub test: TestConfig,

    /// App directory for file-based routing (e.g., "app" or "src/app")
    /// When set, enables Next.js/Expo-style file-based routing:
    ///   app/page.tsx          → /
    ///   app/about/page.tsx    → /about
    ///   app/blog/[slug]/page.tsx → /blog/:slug
    ///   app/layout.tsx        → shared layout wrapper
    ///
    /// Auto-detection order (when not explicitly set):
    ///   1. src/app/  — if src/ exists, colocate routes with source
    ///   2. app/      — flat structure at project root
    #[serde(default)]
    pub app_dir: Option<String>,

    /// Build event webhooks (#105)
    /// POST build results to external services on completion
    #[serde(default)]
    pub webhooks: WebhookConfig,

    /// i18n-aware bundling (#106)
    /// Split bundles by locale, only load current locale's strings
    #[serde(default)]
    pub i18n: I18nConfig,

    /// CSS RTL auto-generation (#107)
    #[serde(default)]
    pub css: CssConfig,

    /// Accessibility linting during build (#108)
    #[serde(default)]
    pub a11y: A11yConfig,

    /// Build-time string encryption (#109)
    /// Encrypt sensitive strings in source at build time
    #[serde(default)]
    pub encrypt: EncryptConfig,

    /// Bundle size budgets (#102)
    /// Exit non-zero on budget violations in CI
    #[serde(default)]
    pub budgets: BudgetConfig,
}

/// Test configuration (Vitest-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Test environment: "node" (default), "jsdom", "happy-dom"
    #[serde(default = "default_test_environment")]
    pub environment: String,

    /// Setup files to run before each test file
    #[serde(default)]
    pub setup_files: Vec<String>,

    /// Whether to run tests with globals (describe, it, expect) instead of imports
    #[serde(default)]
    pub globals: bool,

    /// Test isolation mode: "file" (each file in own context), "pool" (shared pool), "none" (no isolation)
    #[serde(default = "default_test_isolation")]
    pub isolation: String,

    /// Whether to collect code coverage
    #[serde(default)]
    pub coverage: bool,

    /// Coverage report format: "text", "json", "html", "lcov"
    #[serde(default = "default_coverage_reporter")]
    pub coverage_reporter: String,

    /// Whether to enable snapshot testing
    #[serde(default = "default_true")]
    pub snapshot: bool,

    /// Directory for snapshot files (default: "__snapshots__")
    #[serde(default = "default_snapshot_dir")]
    pub snapshot_dir: String,

    /// Whether to update snapshots automatically
    #[serde(default)]
    pub update_snapshots: bool,

    /// Test file patterns (default: ["**/*.{test,spec}.{js,ts,jsx,tsx}"])
    #[serde(default = "default_test_patterns")]
    pub include: Vec<String>,

    /// Test file patterns to exclude
    #[serde(default = "default_test_exclude")]
    pub exclude: Vec<String>,
}

fn default_test_environment() -> String {
    "node".to_string()
}

fn default_out_dir() -> PathBuf {
    PathBuf::from(".pledge")
}

fn default_test_isolation() -> String {
    "file".to_string()
}

fn default_coverage_reporter() -> String {
    "text".to_string()
}

fn default_snapshot_dir() -> String {
    "__snapshots__".to_string()
}

fn default_test_patterns() -> Vec<String> {
    vec![
        "**/*.{test,spec}.{js,ts,jsx,tsx}".to_string(),
    ]
}

fn default_test_exclude() -> Vec<String> {
    vec![
        "**/node_modules/**".to_string(),
        "**/target/**".to_string(),
        "**/.pledge/**".to_string(),
    ]
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            environment: default_test_environment(),
            setup_files: Vec::new(),
            globals: false,
            isolation: default_test_isolation(),
            coverage: false,
            coverage_reporter: default_coverage_reporter(),
            snapshot: true,
            snapshot_dir: default_snapshot_dir(),
            update_snapshots: false,
            include: default_test_patterns(),
            exclude: default_test_exclude(),
        }
    }
}

/// Build configuration for production builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Manual chunk splitting configuration
    /// Maps chunk name to list of module paths/globs to include
    #[serde(default)]
    pub manual_chunks: std::collections::HashMap<String, Vec<String>>,

    /// Inline dynamic imports into parent chunk instead of creating async chunks
    #[serde(default)]
    pub inline_dynamic_imports: bool,

    /// Source map mode: "external" (default), "hidden", "inline", "nosources"
    #[serde(default = "default_source_map_mode")]
    pub source_map_mode: String,

    /// Asset inlining threshold in bytes (default: 4096)
    /// Assets smaller than this are inlined as base64 data URIs
    #[serde(default = "default_assets_inline_limit")]
    pub assets_inline_limit: usize,

    /// Minify JSON modules in production (default: true)
    #[serde(default = "default_true")]
    pub json_minify: bool,

    /// Generate modulepreload link tags for async chunks (default: true)
    #[serde(default = "default_true")]
    pub module_preload: bool,

    /// Generate preload link tags for critical assets (default: false)
    #[serde(default)]
    pub preload: bool,

    /// Generate prefetch link tags for assets (default: false)
    #[serde(default)]
    pub prefetch: bool,

    /// Polyfill modulepreload for older browsers (default: false)
    #[serde(default)]
    pub module_preload_polyfill: bool,

    /// Enable font subsetting for production builds (default: false)
    #[serde(default)]
    pub font_subsetting: bool,

    /// Enable SVG sprite generation (default: false)
    #[serde(default)]
    pub svg_sprite: bool,

    /// Inline process.env.* variables at build time (default: true in production)
    /// Replaces process.env.NODE_ENV with "production" / "development" and
    /// tree-shakes unreachable branches (if (DEV) { ... } eliminated)
    #[serde(default = "default_true")]
    pub env_inline: bool,

    /// Module preloading strategy for entry chunks (default: "lazy")
    /// "eager" — preload all entry + async chunks via <link rel="modulepreload">
    /// "lazy" — only preload entry chunks, async chunks loaded on demand
    /// "manual" — don't auto-generate preload tags, user controls via HTML
    #[serde(default = "default_preload_strategy")]
    pub preload_strategy: String,

    /// Verify build output integrity after emit (default: false)
    /// Checks all chunks exist, no broken import references, all assets resolved
    #[serde(default)]
    pub verify_output: bool,

    /// Incremental output: skip writing unchanged chunks in watch mode (default: true)
    /// Compares content hashes and only writes files that changed
    #[serde(default = "default_true")]
    pub incremental_output: bool,

    /// WASM SIMD optimization (default: "auto")
    /// "auto" — detect SIMD support from build target, generate optimized WASM
    /// "always" — always generate SIMD-optimized WASM instantiation
    /// "never" — always use non-SIMD fallback
    #[serde(default = "default_wasm_simd")]
    pub wasm_simd: String,
}

fn default_source_map_mode() -> String {
    "external".to_string()
}

fn default_assets_inline_limit() -> usize {
    4096
}

fn default_true() -> bool {
    true
}

fn default_preload_strategy() -> String {
    "lazy".to_string()
}

fn default_wasm_simd() -> String {
    "auto".to_string()
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            manual_chunks: std::collections::HashMap::new(),
            inline_dynamic_imports: false,
            source_map_mode: "external".to_string(),
            assets_inline_limit: 4096,
            json_minify: true,
            module_preload: true,
            preload: false,
            prefetch: false,
            module_preload_polyfill: false,
            font_subsetting: false,
            svg_sprite: false,
            env_inline: true,
            preload_strategy: "lazy".to_string(),
            verify_output: false,
            incremental_output: true,
            wasm_simd: "auto".to_string(),
        }
    }
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
#[serde(default)]
pub struct CacheConfig {
    /// Enable filesystem cache (default: true)
    pub enabled: bool,
    /// Cache directory (default: "node_modules/.pledge-cache")
    pub dir: PathBuf,
    /// Remote cache configuration (optional, for CI/team cache sharing)
    #[serde(default)]
    pub remote: RemoteCacheSettings,
}

/// Settings for remote cache (S3/GCS/HTTP)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteCacheSettings {
    /// Enable remote cache (default: false)
    pub enabled: bool,
    /// Backend: "http", "s3", "gcs"
    pub backend: String,
    /// Endpoint URL
    pub endpoint: String,
    /// Bucket name (for S3/GCS)
    pub bucket: Option<String>,
    /// Region (for S3)
    pub region: Option<String>,
    /// Namespace prefix for cache keys
    pub namespace: Option<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: PathBuf::from("node_modules/.pledge-cache"),
            remote: RemoteCacheSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DevServerConfig {
    /// Port (default: 3000)
    #[serde(default = "default_dev_port")]
    pub port: u16,
    /// Host (default: "localhost")
    #[serde(default = "default_dev_host")]
    pub host: String,
    /// Enable HMR (default: true)
    #[serde(default = "default_true")]
    pub hmr: bool,
    /// Open browser on start (default: false)
    #[serde(default)]
    pub open: bool,
    /// HTTPS support (default: false)
    #[serde(default)]
    pub https: bool,
    /// Public directory for static assets (default: "public")
    #[serde(default = "default_public_dir")]
    pub public_dir: String,
    /// Middleware functions to apply to the dev server (JS source code)
    #[serde(default)]
    pub middleware: Vec<String>,
}

fn default_dev_port() -> u16 {
    3000
}

fn default_dev_host() -> String {
    "localhost".to_string()
}

fn default_public_dir() -> String {
    "public".to_string()
}

impl Default for DevServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "localhost".to_string(),
            hmr: true,
            open: false,
            https: false,
            public_dir: "public".to_string(),
            middleware: Vec::new(),
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
#[serde(default)]
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
#[serde(default)]
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

/// Webhook configuration for build events (#105)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookConfig {
    /// Enable webhooks (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// URL to POST build results to on completion
    #[serde(default)]
    pub on_build: Option<String>,
    /// URL to POST build errors to
    #[serde(default)]
    pub on_error: Option<String>,
    /// Additional headers to send
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
}

/// i18n configuration for locale-aware bundling (#106)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct I18nConfig {
    /// Enable i18n-aware bundling (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// Supported locales (e.g., ["en", "fr", "ja"])
    #[serde(default)]
    pub locales: Vec<String>,
    /// Default locale (default: "en")
    #[serde(default = "default_locale")]
    pub default_locale: String,
    /// Message file pattern (default: "./messages.${locale}.json")
    #[serde(default = "default_message_pattern")]
    pub message_pattern: String,
}

impl Default for I18nConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            locales: Vec::new(),
            default_locale: default_locale(),
            message_pattern: default_message_pattern(),
        }
    }
}

fn default_locale() -> String {
    "en".to_string()
}

fn default_message_pattern() -> String {
    "./messages.${locale}.json".to_string()
}

/// CSS configuration for RTL auto-generation (#107)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssConfig {
    /// RTL CSS generation mode: "auto", "manual", "off" (default: "off")
    #[serde(default = "default_rtl_mode")]
    pub rtl: String,
}

impl Default for CssConfig {
    fn default() -> Self {
        Self {
            rtl: default_rtl_mode(),
        }
    }
}

fn default_rtl_mode() -> String {
    "off".to_string()
}

/// Accessibility linting configuration (#108)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A11yConfig {
    /// Enable a11y linting during build (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// Fail build on a11y errors (default: true)
    #[serde(default = "default_true")]
    pub fail_on_error: bool,
    /// Check for missing alt attributes on images
    #[serde(default = "default_true")]
    pub check_alt: bool,
    /// Check for ARIA labels on interactive elements
    #[serde(default = "default_true")]
    pub check_aria: bool,
    /// Check for sufficient color contrast
    #[serde(default)]
    pub check_contrast: bool,
}

impl Default for A11yConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            fail_on_error: true,
            check_alt: true,
            check_aria: true,
            check_contrast: false,
        }
    }
}

/// Build-time string encryption configuration (#109)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptConfig {
    /// Enable string encryption (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// Keys to encrypt (from process.env or define)
    #[serde(default)]
    pub keys: Vec<String>,
    /// Encryption key (32-byte hex string). If not set, generated at build time.
    #[serde(default)]
    pub key: Option<String>,
}

impl Default for EncryptConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            keys: Vec::new(),
            key: None,
        }
    }
}

/// Bundle size budget configuration (#102)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Enable budget checking (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// Maximum total bundle size in bytes (0 = no limit)
    #[serde(default)]
    pub max_bundle_size: usize,
    /// Maximum per-chunk size in bytes (0 = no limit)
    #[serde(default)]
    pub max_chunk_size: usize,
    /// Maximum number of chunks (0 = no limit)
    #[serde(default)]
    pub max_chunk_count: usize,
    /// Per-entry-point budgets (entry name → max bytes)
    #[serde(default)]
    pub entry_budgets: std::collections::HashMap<String, usize>,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_bundle_size: 0,
            max_chunk_size: 0,
            max_chunk_count: 0,
            entry_budgets: std::collections::HashMap::new(),
        }
    }
}

impl Default for PledgeConfig {
    fn default() -> Self {
        // Auto-detect entry point based on project structure
        // Priority: app/entry.tsx → src/app/entry.tsx → src/index.tsx → index.tsx
        let entry = {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let candidates = [
                cwd.join("app").join("entry.tsx"),
                cwd.join("src").join("app").join("entry.tsx"),
                cwd.join("src").join("index.tsx"),
                cwd.join("index.tsx"),
            ];
            candidates.iter()
                .find(|p| p.exists())
                .and_then(|p| p.strip_prefix(&cwd).ok())
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|| "src/index.tsx".to_string())
        };
        Self {
            entry: vec![entry],
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
            build: BuildConfig::default(),
            test: TestConfig::default(),
            app_dir: None,
            webhooks: WebhookConfig::default(),
            i18n: I18nConfig::default(),
            css: CssConfig::default(),
            a11y: A11yConfig::default(),
            encrypt: EncryptConfig::default(),
            budgets: BudgetConfig::default(),
        }
    }
}

impl PledgeConfig {
    /// Resolve the app directory for file-based routing.
    /// If `app_dir` is explicitly set, use that.
    /// Otherwise auto-detect in priority order:
    ///   1. app/ at project root (Next.js convention)
    ///   2. src/app/ inside src
    pub fn resolve_app_dir(&self) -> Option<String> {
        if let Some(dir) = &self.app_dir {
            return Some(dir.clone());
        }
        let root_app = self.root.join("app");
        let src_app = self.root.join("src").join("app");
        if root_app.is_dir() {
            Some("app".to_string())
        } else if src_app.is_dir() {
            Some("src/app".to_string())
        } else {
            None
        }
    }

    /// Resolve the source directory for non-app projects.
    /// Returns the base directory containing components/, lib/, etc.
    /// Priority: src/ if it exists, otherwise project root.
    pub fn resolve_src_dir(&self) -> String {
        if self.root.join("src").is_dir() {
            "src".to_string()
        } else {
            ".".to_string()
        }
    }

    /// Detect the project structure convention.
    /// Returns the base directory for index.html and module resolution.
    /// Priority: app/ → src/app/ → src/ → root
    pub fn resolve_base_dir(&self) -> Option<String> {
        let root_app = self.root.join("app");
        let src_app = self.root.join("src").join("app");
        let src_dir = self.root.join("src");

        if root_app.is_dir() {
            Some("app".to_string())
        } else if src_app.is_dir() {
            Some("src/app".to_string())
        } else if src_dir.is_dir() {
            Some("src".to_string())
        } else {
            None
        }
    }

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
                if b == string_char {
                    // Convert closing single-quote/backtick to double-quote
                    if string_char != b'"' {
                        result.push('"');
                    } else {
                        result.push(b as char);
                    }
                    in_string = false;
                } else if b == b'\r' {
                    // Skip \r inside strings (CRLF line endings)
                } else if b == b'"' && string_char != b'"' {
                    // Escape double quotes inside single-quoted/backtick strings
                    result.push('\\');
                    result.push('"');
                } else {
                    result.push(b as char);
                }
                i += 1;
                continue;
            }

            // Skip \r outside strings (CRLF line endings)
            if b == b'\r' {
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
