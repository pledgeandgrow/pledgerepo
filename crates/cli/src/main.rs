// Pledge CLI — the entry point
//
// Usage:
//   pledge dev          Start dev server with HMR
//   pledge build        Production build
//   pledge serve        Preview production build
//   pledge cache clear  Clear the filesystem cache
//   pledge bench        Run benchmarks

// Global allocator — mimalloc by default for better multi-threaded performance.
// Use `--features jemalloc` for heap profiling and leak detection.
#[cfg(not(feature = "jemalloc"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use pledgepack_core::{BuildEngine, PledgeConfig};
use pledgepack_core::config::BuildMode;
use pledgepack_core::env::EnvVars;
use pledgepack_core::edge;
use pledgepack_core::compression;
use pledgepack_core::html;
use pledgepack_core::dep_bundler::DepBundler;
use pledgepack_core::detect;
use pledgepack_core::doctor;
use pledgepack_core::migrate;
use pledgepack_core::config_validate;
use pledgepack_js_plugin_host::JsPluginHost;
use camino::Utf8PathBuf;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "pledge",
    version,
    about = "A Rust+Zig bundler with incremental computation, WASM plugins, and Rollup-quality output"
)]
struct Cli {
    /// Project root directory (default: current directory)
    #[arg(long, global = true)]
    root: Option<Utf8PathBuf>,

    /// Config file path (default: auto-detect pledge.json)
    #[arg(long, global = true)]
    config: Option<Utf8PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the dev server with HMR
    Dev {
        /// Port to serve on
        #[arg(short, long)]
        port: Option<u16>,

        /// Host to bind to
        #[arg(long)]
        host: Option<String>,

        /// Open browser on start
        #[arg(long)]
        open: bool,
    },

    /// Create a production build
    Build {
        /// Output directory
        #[arg(short, long)]
        out_dir: Option<Utf8PathBuf>,

        /// Disable source maps
        #[arg(long)]
        no_sourcemap: bool,

        /// Enable build profiling (timing per phase)
        #[arg(long)]
        profile: bool,

        /// Watch mode — rebuild on file change
        #[arg(long)]
        watch: bool,

        /// Verify build output integrity after emit
        #[arg(long)]
        verify: bool,

        /// Check bundle size budgets and exit non-zero on violations (#102)
        #[arg(long)]
        check_budgets: bool,
    },

    /// Preview a production build with compression
    Preview {
        /// Port to serve on
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// Serve a production build (alias for preview)
    Serve {
        /// Port to serve on
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// Scaffold a new project (auto-detects framework if no template given)
    Create {
        /// Template name (react, vue, svelte, solid, vanilla, next, tanstack)
        /// If omitted, Pledgepack auto-detects the framework from existing files
        template: Option<String>,

        /// Project name / directory
        name: Option<String>,
    },

    /// Add Pledgepack to an existing project (migrates config from Vite/webpack/CRA)
    Init {
        /// Force overwrite existing pledge.config.ts
        #[arg(long)]
        force: bool,

        /// Skip framework detection, use this framework
        #[arg(long)]
        framework: Option<String>,
    },

    /// Diagnose build issues, config problems, and performance bottlenecks
    Doctor,

    /// Migrate config from Vite/webpack/CRA/Next.js to pledge.config.ts
    Migrate {
        /// Dry run — show what would be migrated without writing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Validate config and show field suggestions
    Config,

    /// Analyze bundle size with interactive treemap
    Analyze {
        /// Port to serve the analysis on
        #[arg(short, long)]
        port: Option<u16>,

        /// Generate interactive dependency graph instead of treemap (#104)
        #[arg(long)]
        graph: bool,
    },

    /// Run tests (Vitest-compatible API)
    Test {
        /// Test file pattern (glob)
        #[arg(short, long)]
        pattern: Option<String>,

        /// Watch mode
        #[arg(short, long)]
        watch: bool,

        /// UI mode (browser)
        #[arg(long)]
        ui: bool,
    },

    /// Manage the cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Run benchmarks
    Bench {
        /// Compare against a baseline ref (git commit hash or name) (#103)
        #[arg(long)]
        baseline: Option<String>,

        /// Regression threshold percentage (default: 10.0) (#103)
        #[arg(long)]
        threshold: Option<f64>,
    },

    /// Serve the build telemetry dashboard (#101)
    Dashboard {
        /// Port to serve the dashboard on
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// Generate TypeScript declarations for env variables
    GenerateEnvTypes,

    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(short, long)]
        shell: Option<String>,
    },
}

#[derive(Subcommand)]
enum CacheAction {
    /// Clear the filesystem cache
    Clear,
    /// Show cache statistics
    Stats,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let root = cli.root.unwrap_or_else(|| Utf8PathBuf::from("."));
    let root_path = root.as_std_path().to_path_buf();
    let mut config = if let Some(config_path) = cli.config {
        let content = std::fs::read_to_string(&config_path)?;
        serde_json::from_str(&content)?
    } else {
        PledgeConfig::load(&root_path)?
    };
    config.root = std::fs::canonicalize(&root_path).unwrap_or(root_path);

    match cli.command {
        Commands::Dev { port, host, open } => {
            if let Some(p) = port {
                config.dev_server.port = p;
            }
            if let Some(h) = host {
                config.dev_server.host = h;
            }
            if open {
                config.dev_server.open = true;
            }
            config.mode = pledgepack_core::config::BuildMode::Development;

            println!(
                "\n  \x1b[36mpledge\x1b[0m dev server starting...\n  \x1b[90m→\x1b[0m http://{}:{}\n",
                config.dev_server.host, config.dev_server.port
            );

            let engine = BuildEngine::new(Arc::new(config.clone()));
            pledgepack_dev_server::serve(engine, &config).await?;
        }

        Commands::Build { out_dir, no_sourcemap, profile, watch, verify, check_budgets } => {
            if let Some(out) = out_dir {
                config.out_dir = out.into_std_path_buf();
            }
            if no_sourcemap {
                config.source_maps = false;
            }
            if profile {
                config.profile = true;
            }
            if verify {
                config.build.verify_output = true;
            }
            if check_budgets {
                config.budgets.enabled = true;
            }
            config.mode = pledgepack_core::config::BuildMode::Production;

            println!("\n  \x1b[36mpledge\x1b[0m building for production...\n");

            use indicatif::{ProgressBar, ProgressStyle};
            let pb = ProgressBar::new(4);
            pb.set_style(
                ProgressStyle::with_template("  {spinner:.green} {msg} [{bar:30.cyan/blue}] {pos}/{len}")
                    .unwrap()
                    .progress_chars("█░"),
            );

            let profile_start = std::time::Instant::now();

            pb.set_message("Building modules");
            let mut engine = BuildEngine::new(Arc::new(config.clone()));
            let result = engine.build().await?;
            pb.inc(1);

            pb.set_message("Optimizing chunks");
            let optimize_start = std::time::Instant::now();
            // Run optimizer (tree shaking, code splitting, vendor/shared chunks)
            // Use optimize_with_config for manual_chunks and inline_dynamic_imports support
            let entry_ids: Vec<pledgepack_core::ModuleId> = engine.modules().values()
                .take(1).map(|m| m.id).collect();
            let mut optimizer = pledgepack_optimizer::Optimizer::new();
            let chunks = optimizer.optimize_with_config(
                &entry_ids,
                engine.modules(),
                engine.graph(),
                &config.build,
            )?;
            tracing::info!("Optimizer: {} chunks", chunks.len());
            let optimize_ms = optimize_start.elapsed().as_millis();
            pb.inc(1);

            pb.set_message("Emitting output");
            let emit_start = std::time::Instant::now();
            // Emit production artifacts to .pledge/
            engine.emit()?;
            let emit_ms = emit_start.elapsed().as_millis();
            pb.inc(1);

            // Generate env type declarations if enabled
            if config.env_dts {
                let env = EnvVars::load(&config.root, BuildMode::Production, &config.env_prefix);
                let dts = env.generate_dts(&config.env_prefix);
                let dts_path = config.root.join("pledge-env.d.ts");
                std::fs::write(&dts_path, &dts)?;
            }

            // Process HTML entry point if it exists — supports multiple script entries
            // Convention: auto-detect from app/ → src/app/ → src/ → root
            let html_path = if let Some(entry) = &config.html_entry {
                config.root.join(entry)
            } else {
                let base = config.resolve_base_dir();
                match &base {
                    Some(b) => config.root.join(b).join("index.html"),
                    None => config.root.join("index.html"),
                }
            };
            if html_path.exists() {
                let html_entry = html::process_html(&html_path)?;
                tracing::info!("HTML entry: {} scripts, {} stylesheets, {} preloads",
                    html_entry.scripts.len(), html_entry.stylesheets.len(), html_entry.module_preloads.len());

                // If HTML has multiple script entries, add them to config entry
                if html_entry.scripts.len() > 1 {
                    tracing::info!("Multi-script entry: {} entry points detected", html_entry.scripts.len());
                }
            }

            // File-based routing: scan app/ directory if auto-detected or configured
            if let Some(app_dir) = config.resolve_app_dir() {
                let route_table = pledgepack_core::router::scan_app_dir(&config.root, &app_dir)?;
                if !route_table.routes.is_empty() {
                    tracing::info!("App router: {} routes from {}/", route_table.routes.len(), app_dir);
                    for route in &route_table.routes {
                        tracing::info!("  {} → {}", route.pattern, route.file);
                    }
                    // Generate virtual router module
                    let router_module = route_table.generate_router_module();
                    let router_path = config.out_dir.join("__pledge_router.js");
                    std::fs::write(&router_path, &router_module)?;
                    tracing::info!("  Generated router: {}", router_path.display());
                }
            }

            // Pre-bundle dependencies
            let dep_start = std::time::Instant::now();
            pb.set_message("Pre-bundling dependencies");
            let mut dep_bundler = DepBundler::new();
            let _bundled_deps = dep_bundler.pre_bundle(&config)?;
            let dep_ms = dep_start.elapsed().as_millis();
            pb.inc(1);
            pb.finish_and_clear();

            // Load JS plugins if configured
            if !config.plugins.is_empty() {
                let mut plugin_host = JsPluginHost::new();
                plugin_host.load_plugins(&config.plugins)?;
                plugin_host.build_start();
                tracing::info!("Loaded {} JS plugins", plugin_host.plugins().len());
            }

            // Generate edge bundle if configured
            if let Some(ref edge_target) = config.edge_target {
                if let Some(target) = edge::EdgeTarget::from_str(edge_target) {
                    let out_dir = config.root.join(&config.out_dir);
                    let bundle_code = String::new(); // Would use engine.collect_module_code()
                    edge::generate_edge_bundle(target, &bundle_code, None, &out_dir)?;
                }
            }

            // Generate compressed output if configured
            if config.compress_gzip || config.compress_brotli {
                let compress_start = std::time::Instant::now();
                let out_dir = config.root.join(&config.out_dir);
                if out_dir.exists() {
                    let stats = compression::compress_directory(
                        &out_dir, config.compress_gzip, config.compress_brotli,
                    )?;
                    if stats.files_compressed > 0 {
                        println!("  \x1b[32m✓\x1b[0m Compressed {} files (gzip: {:.1}KB, brotli: {:.1}KB)",
                            stats.files_compressed,
                            stats.gzipped_bytes as f64 / 1024.0,
                            stats.brotli_bytes as f64 / 1024.0,
                        );
                    }
                }
                let compress_ms = compress_start.elapsed().as_millis();
                tracing::info!("Compression: {}ms", compress_ms);
            }

            let total_ms = profile_start.elapsed().as_millis();

            // Record build telemetry (#101)
            let out_dir_full = config.root.join(&config.out_dir);
            let bundle_size: u64 = if out_dir_full.exists() {
                std::fs::read_dir(&out_dir_full)
                    .ok()
                    .map(|entries| {
                        entries.filter_map(|e| e.ok())
                            .filter_map(|e| e.metadata().ok())
                            .map(|m| m.len())
                            .sum()
                    })
                    .unwrap_or(0)
            } else { 0 };
            let _ = pledgepack_core::telemetry::record_build(
                &config.root,
                total_ms,
                result.modules_built,
                result.modules_cached,
                bundle_size as usize,
                "production",
                chunks.len(),
                true,
                None,
            );

            // Send build webhook (#105)
            if config.webhooks.enabled {
                let event = pledgepack_core::webhooks::BuildEvent {
                    event: "build.complete".to_string(),
                    success: true,
                    duration_ms: total_ms,
                    modules_built: result.modules_built,
                    modules_cached: result.modules_cached,
                    bundle_size: bundle_size as usize,
                    chunk_count: chunks.len(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    error: None,
                };
                let _ = pledgepack_core::webhooks::send_webhook(&config.webhooks, event).await;
            }

            // Check bundle size budgets (#102)
            if config.budgets.enabled {
                let chunk_sizes: Vec<(String, usize)> = chunks.iter()
                    .map(|c| {
                        let size: usize = c.modules.iter()
                            .filter_map(|mid| engine.modules().get(mid))
                            .map(|m| m.source.len())
                            .sum();
                        (c.id.clone(), size)
                    })
                    .collect();
                let violations = pledgepack_core::budgets::check_budgets(
                    &out_dir_full,
                    &config.budgets,
                    &chunk_sizes,
                )?;

                if !violations.is_empty() {
                    // GitHub Actions annotation format
                    if std::env::var("GITHUB_ACTIONS").is_ok() {
                        print!("{}", pledgepack_core::budgets::format_github_annotations(&violations));
                    }
                    // Print violations
                    for v in &violations {
                        println!("  \x1b[31m✗ budget\x1b[0m {}", v.message);
                    }
                    anyhow::bail!("Budget check failed with {} violation(s)", violations.len());
                } else {
                    println!("  \x1b[32m✓ budgets\x1b[0m all within limits");
                }
            }

            // a11y linting (#108)
            if config.a11y.enabled {
                let html_path = config.root.join(&config.out_dir).join("index.html");
                if html_path.exists() {
                    let html_content = std::fs::read_to_string(&html_path)?;
                    let violations = pledgepack_core::a11y::lint_html(&html_content, &config.a11y)?;
                    if !violations.is_empty() {
                        print!("{}", pledgepack_core::a11y::format_violations(&violations));
                        if config.a11y.fail_on_error && violations.iter().any(|v| v.severity == pledgepack_core::a11y::Severity::Error) {
                            anyhow::bail!("a11y linting failed with {} error(s)", violations.iter().filter(|v| v.severity == pledgepack_core::a11y::Severity::Error).count());
                        }
                    } else {
                        println!("  \x1b[32m✓ a11y\x1b[0m no violations found");
                    }
                }
            }

            if config.profile {
                println!("  \x1b[32m✓\x1b[0m Build profile:\n");
                println!("    Parse + Transform: {}ms", result.duration_ms);
                println!("    Optimize:          {}ms", optimize_ms);
                println!("    Emit:              {}ms", emit_ms);
                println!("    Dep Pre-bundle:    {}ms", dep_ms);
                println!("    Total:             {}ms\n", total_ms);
            } else {
                println!(
                    "  \x1b[32m✓\x1b[0m Built {} modules ({} cached) in {}ms\n",
                    result.modules_built, result.modules_cached, total_ms
                );
            }

            // Watch mode: monitor files and rebuild on change with debounce
            if watch {
                println!("  \x1b[90mWatching for changes... (Ctrl+C to exit)\x1b[0m\n");
                use notify::{Watcher, RecursiveMode, EventKind, recommended_watcher};
                use std::sync::mpsc::channel;
                use std::time::{Duration, Instant};
                use std::collections::HashSet;

                let (tx, rx) = channel::<notify::Result<notify::Event>>();
                let mut watcher = recommended_watcher(move |res: notify::Result<notify::Event>| {
                    let _ = tx.send(res);
                })?;

                watcher.watch(&config.root, RecursiveMode::Recursive)?;

                let debounce_ms = Duration::from_millis(200);
                let mut last_rebuild: Option<Instant> = None;
                let mut pending_paths: HashSet<std::path::PathBuf> = HashSet::new();

                loop {
                    match rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(Ok(event)) => {
                            let is_relevant = matches!(
                                event.kind,
                                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                            );
                            if !is_relevant { continue; }

                            for path in &event.paths {
                                let path_str = path.to_string_lossy();
                                if path_str.contains("node_modules")
                                    || path_str.contains(".pledge")
                                    || path_str.contains("target")
                                    || path_str.contains(".git")
                                {
                                    continue;
                                }
                                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                                if !matches!(ext, "ts" | "tsx" | "js" | "jsx" | "css" | "scss" | "less" | "vue" | "svelte" | "html" | "json") {
                                    continue;
                                }
                                pending_paths.insert(path.clone());
                            }

                            last_rebuild = Some(Instant::now());
                        }
                        Ok(Err(e)) => {
                            tracing::warn!("Watch error: {}", e);
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            if let Some(last) = last_rebuild {
                                if last.elapsed() >= debounce_ms && !pending_paths.is_empty() {
                                    let changed: Vec<_> = pending_paths.drain().collect();
                                    let changed_count = changed.len();

                                    for path in &changed {
                                        let rel = path.strip_prefix(&config.root)
                                            .unwrap_or(path)
                                            .to_string_lossy()
                                            .replace('\\', "/");
                                        println!("  \x1b[33mchanged:\x1b[0m {}", rel);
                                    }

                                    println!("  \x1b[36mrebuilding...\x1b[0m");
                                    let rebuild_start = std::time::Instant::now();

                                    match engine.build().await {
                                        Ok(_result) => {
                                            let elapsed = rebuild_start.elapsed();
                                            println!("  \x1b[32m✓ rebuilt\x1b[0m {} file(s) in {:.0?}\n", changed_count, elapsed);
                                        }
                                        Err(e) => {
                                            println!("  \x1b[31m✗ rebuild failed:\x1b[0m {}\n", e);
                                        }
                                    }

                                    last_rebuild = None;
                                }
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                            println!("  \x1b[33mWatch channel disconnected, exiting...\x1b[0m");
                            break;
                        }
                    }
                }
            }
        }

        Commands::Preview { port } => {
            let port = port.unwrap_or(4000);
            let out_dir = config.out_dir.clone();

            if !out_dir.exists() {
                println!("\n  \x1b[33mpledge preview\x1b[0m — .pledge/ not found. Run `pledge build` first.\n");
                return Ok(());
            }

            println!(
                "\n  \x1b[36mpledge preview\x1b[0m — serving {} on http://localhost:{}\n",
                out_dir.display(), port
            );

            let app = axum::Router::new()
                .fallback_service(tower_http::services::ServeDir::new(&out_dir));

            let addr = format!("127.0.0.1:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }

        Commands::Create { template, name } => {
            use dialoguer::{Select, Input, Confirm};
            use console::style;

            // If no template provided and we're in a TTY, run interactive wizard
            let (template, project_name) = if template.is_none() && atty::is(atty::Stream::Stdin) {
                println!("\n  {} Pledgepack Create Wizard\n", style("pledge create").cyan().bold());

                // Project name
                let default_name = name.clone().unwrap_or_else(|| "my-app".to_string());
                let project_name: String = Input::new()
                    .with_prompt("Project name")
                    .default(default_name)
                    .interact_text()?;

                // Framework selection
                let frameworks = [
                    "React",
                    "Vue",
                    "Svelte",
                    "Solid",
                    "Next.js (React adapter)",
                    "TanStack Router (React adapter)",
                    "Vanilla (no framework)",
                ];
                let framework_idx = Select::new()
                    .with_prompt("Select framework")
                    .items(&frameworks)
                    .default(0)
                    .interact()?;

                let template = match framework_idx {
                    0 => "react",
                    1 => "vue",
                    2 => "svelte",
                    3 => "solid",
                    4 => "next",
                    5 => "tanstack",
                    _ => "vanilla",
                }.to_string();

                // TypeScript toggle
                let typescript = Confirm::new()
                    .with_prompt("Use TypeScript?")
                    .default(true)
                    .interact()?;

                // CSS framework
                let css_frameworks = [
                    "None (plain CSS)",
                    "Tailwind CSS",
                    "UnoCSS",
                    "Panda CSS",
                    "Vanilla Extract",
                ];
                let css_idx = Select::new()
                    .with_prompt("CSS framework")
                    .items(&css_frameworks)
                    .default(0)
                    .interact()?;

                let _css = match css_idx {
                    1 => "tailwind",
                    2 => "unocss",
                    3 => "panda-css",
                    4 => "vanilla-extract",
                    _ => "none",
                };

                // Package manager
                let pkg_managers = ["npm", "yarn", "pnpm", "bun"];
                let pm_idx = Select::new()
                    .with_prompt("Package manager")
                    .items(&pkg_managers)
                    .default(0)
                    .interact()?;

                let _pm = pkg_managers[pm_idx];

                // Summary
                println!("\n  {} Project will be created with:", style("Summary").dim());
                println!("    Name:       {}", project_name);
                println!("    Framework:  {}", template);
                println!("    TypeScript: {}", if typescript { "yes" } else { "no" });
                println!();

                let proceed = Confirm::new()
                    .with_prompt("Proceed?")
                    .default(true)
                    .interact()?;

                if !proceed {
                    println!("\n  {}\n", style("Cancelled.").dim());
                    return Ok(());
                }

                (template, project_name)
            } else {
                let project_name = name.unwrap_or_else(|| "my-app".to_string());
                let template = template.unwrap_or_else(|| {
                    let detection = detect::detect_project(std::path::Path::new("."));
                    detection.framework.as_str().to_string()
                });
                (template, project_name)
            };

            let project_dir = std::path::Path::new(&project_name);

            if project_dir.exists() {
                println!("\n  \x1b[31mError\x1b[0m Directory '{}' already exists\n", project_name);
                return Ok(());
            }

            std::fs::create_dir_all(project_dir)?;
            std::fs::create_dir_all(project_dir.join("src"))?;

            // Create package.json
            let pkg = serde_json::json!({
                "name": project_name,
                "version": "0.1.0",
                "scripts": {
                    "dev": "pledge dev",
                    "build": "pledge build",
                    "preview": "pledge preview"
                }
            });
            std::fs::write(project_dir.join("package.json"), serde_json::to_string_pretty(&pkg)?)?;

            // Create pledge.config.ts
            let pledge_config = match template.as_str() {
                "vue" => r#"import { defineConfig } from 'pledge';

export default defineConfig({
  entry: ['src/index.tsx'],
  framework: 'vue',
  devServer: {
    port: 3000,
    hmr: true,
  },
});
"#,
                "svelte" => r#"import { defineConfig } from 'pledge';

export default defineConfig({
  entry: ['src/index.tsx'],
  framework: 'svelte',
  devServer: {
    port: 3000,
    hmr: true,
  },
});
"#,
                "solid" => r#"import { defineConfig } from 'pledge';

export default defineConfig({
  entry: ['src/index.tsx'],
  framework: 'solid',
  devServer: {
    port: 3000,
    hmr: true,
  },
});
"#,
                "next" => r#"import { defineConfig } from 'pledge';

export default defineConfig({
  entry: ['src/index.tsx'],
  framework: 'react',
  devServer: {
    port: 3000,
    hmr: true,
  },
  plugins: [],
});
"#,
                "tanstack" => r#"import { defineConfig } from 'pledge';

export default defineConfig({
  entry: ['src/index.tsx'],
  framework: 'react',
  devServer: {
    port: 3000,
    hmr: true,
  },
});
"#,
                _ => r#"import { defineConfig } from 'pledge';

export default defineConfig({
  entry: ['src/index.tsx'],
  framework: 'react',
  devServer: {
    port: 3000,
    hmr: true,
  },
});
"#,
            };
            std::fs::write(project_dir.join("pledge.config.ts"), pledge_config)?;

            // Create .env file
            std::fs::write(project_dir.join(".env"), r#"# Pledge environment variables
PLEDGE_APP_NAME=My App
PLEDGE_API_URL=http://localhost:8080
"#)?;

            // Create .env.local (gitignored)
            std::fs::write(project_dir.join(".env.local"), r#"# Local environment variables (not committed)
PLEDGE_API_URL=http://localhost:3000
"#)?;

            // Create index.html
            std::fs::write(project_dir.join("index.html"), &html::generate_default_html(
                "src/index.tsx",
                &project_name,
            ))?;

            // Create entry file based on template
            let entry = match template.as_str() {
                "vue" => r##"// Vue template
const root = document.getElementById("root");
if (root) {
  root.innerHTML = `<h1 style="color:#6366f1;">${project_name}</h1>`;
}
export default {};
"##,
                "svelte" => r##"// Svelte template
const root = document.getElementById("root");
if (root) {
  root.innerHTML = `<h1 style="color:#ff3e00;">${project_name}</h1>`;
}
export default {};
"##,
                "solid" => r##"// Solid template
const root = document.getElementById("root");
if (root) {
  root.innerHTML = `<h1 style="color:#2c4f7c;">${project_name}</h1>`;
}
export default {};
"##,
                "vanilla" => r#"// Vanilla template
const root = document.getElementById("root");
if (root) {
  root.innerHTML = `<h1>${project_name}</h1>`;
}
export default {};
"#,
                _ => r##"// React template
function App() {
  return React.createElement("h1", null, "Hello from Pledge!");
}

const root = document.getElementById("root");
if (root) {
  root.innerHTML = "";
  const h1 = document.createElement("h1");
  h1.textContent = "Hello from Pledge!";
  h1.style.color = "#6366f1";
  root.appendChild(h1);
}
export default App;
"##,
            };

            let entry_content = entry.replace("${project_name}", &project_name);
            std::fs::write(project_dir.join("src/index.tsx"), entry_content)?;

            // Create utils.ts
            std::fs::write(project_dir.join("src/utils.ts"), r#"export function greet(name: string): string {
  return `Hello, ${name}!`;
}
"#)?;

            // Create .gitignore
            std::fs::write(project_dir.join(".gitignore"), ".pledge/\ntarget/\nnode_modules/\n.env.local\npledge-env.d.ts\n")?;

            println!("\n  \x1b[32m✓\x1b[0m Created {} project: {}\n", template, project_name);
            println!("  \x1b[90mcd {} && pledge dev\x1b[0m\n", project_name);
        }

        Commands::Init { force, framework } => {
            let root = std::path::Path::new(".");

            // Check if pledge.config.ts already exists
            let config_exists = root.join("pledge.config.ts").exists()
                || root.join("pledge.config.js").exists()
                || root.join("pledge.config.json").exists()
                || root.join("pledge.json").exists();

            if config_exists && !force {
                println!("\n  \x1b[33mpledge config already exists\x1b[0m\n");
                println!("  Use \x1b[36mpledge init --force\x1b[0m to overwrite\n");
                return Ok(());
            }

            // Detect project framework
            let detection = detect::detect_project(root);

            let framework_name = framework.unwrap_or_else(|| detection.framework.as_str().to_string());

            println!("\n  \x1b[36mpledge init\x1b[0m — adding Pledgepack to your project\n");
            println!("  \x1b[90mDetected:\x1b[0m");
            println!("    Framework:      {}", framework_name);
            println!("    TypeScript:     {}", if detection.typescript { "yes" } else { "no" });
            println!("    CSS:            {}", detection.css_preprocessor.as_str());
            println!("    Package mgr:    {}", detection.package_manager.as_str());
            println!("    Build tool:     {}", detection.build_tool.as_str());
            println!("    Entry:          {}", detection.entry_file);
            println!();

            // Interactive confirm if in a TTY
            if atty::is(atty::Stream::Stdin) && !force {
                let proceed = inquire::Confirm::new("Proceed with these settings?")
                    .with_default(true)
                    .prompt();
                match proceed {
                    Ok(true) => {}
                    Ok(false) => {
                        println!("\n  \x1b[33mCancelled.\x1b[0m\n");
                        return Ok(());
                    }
                    Err(_) => {}
                }
            }

            // Generate config
            let config_content = detect::generate_config(&detection);
            let config_path = root.join("pledge.config.ts");
            std::fs::write(&config_path, &config_content)?;
            println!("  \x1b[32m✓\x1b[0m Created pledge.config.ts");

            // Update package.json scripts
            let pkg_path = root.join("package.json");
            if pkg_path.exists() {
                let pkg_content = std::fs::read_to_string(&pkg_path)?;
                if let Ok(mut pkg) = serde_json::from_str::<serde_json::Value>(&pkg_content) {
                    if let Some(scripts) = pkg.get_mut("scripts").and_then(|s| s.as_object_mut()) {
                        scripts.insert("dev".to_string(), serde_json::Value::String("pledge dev".to_string()));
                        scripts.insert("build".to_string(), serde_json::Value::String("pledge build".to_string()));
                        scripts.insert("preview".to_string(), serde_json::Value::String("pledge preview".to_string()));
                    }
                    let new_pkg = serde_json::to_string_pretty(&pkg)?;
                    std::fs::write(&pkg_path, new_pkg)?;
                    println!("  \x1b[32m✓\x1b[0m Updated package.json scripts");
                }
            }

            // Update .gitignore
            let gitignore_path = root.join(".gitignore");
            let gitignore = if gitignore_path.exists() {
                std::fs::read_to_string(&gitignore_path).unwrap_or_default()
            } else {
                String::new()
            };

            let mut new_gitignore = gitignore.clone();
            for entry in [".pledge/", "pledge-env.d.ts"] {
                if !new_gitignore.contains(entry) {
                    if !new_gitignore.is_empty() && !new_gitignore.ends_with('\n') {
                        new_gitignore.push('\n');
                    }
                    new_gitignore.push_str(entry);
                    new_gitignore.push('\n');
                }
            }
            if new_gitignore != gitignore {
                std::fs::write(&gitignore_path, &new_gitignore)?;
                println!("  \x1b[32m✓\x1b[0m Updated .gitignore");
            }

            // Create .env if it doesn't exist
            let env_path = root.join(".env");
            if !env_path.exists() {
                std::fs::write(&env_path, "# Pledge environment variables\nPLEDGE_APP_NAME=My App\n")?;
                println!("  \x1b[32m✓\x1b[0m Created .env");
            }

            // Create index.html if it doesn't exist
            let html_path = root.join("index.html");
            if !html_path.exists() {
                let project_name = std::path::Path::new(".")
                    .canonicalize()
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .unwrap_or_else(|| "My App".to_string());
                std::fs::write(&html_path, html::generate_default_html(&detection.entry_file, &project_name))?;
                println!("  \x1b[32m✓\x1b[0m Created index.html");
            }

            println!("\n  \x1b[90mNext steps:\x1b[0m");
            println!("    {} pledgepack", detection.package_manager.install_cmd());
            println!("    {} pledge dev\n", detection.package_manager.dev_cmd());
        }

        Commands::Doctor => {
            println!("\n  \x1b[36mpledge doctor\x1b[0m — running diagnostics...\n");

            let root = std::path::Path::new(".");
            let report = doctor::run_diagnostics(root, &config);

            // Group by category
            let categories = [
                doctor::DiagnosticCategory::Config,
                doctor::DiagnosticCategory::Dependencies,
                doctor::DiagnosticCategory::Performance,
                doctor::DiagnosticCategory::Project,
                doctor::DiagnosticCategory::Security,
            ];

            for category in &categories {
                let checks: Vec<&doctor::DiagnosticCheck> = report.checks.iter()
                    .filter(|c| &c.category == category)
                    .collect();

                if checks.is_empty() {
                    continue;
                }

                println!("  \x1b[1m{}\x1b[0m", category.label());
                for check in checks {
                    print!("    {} {} ", check.status.icon(), check.name);
                    if check.status == doctor::DiagnosticStatus::Pass {
                        println!("\x1b[90m{}\x1b[0m", check.message);
                    } else {
                        println!("{}", check.message);
                    }
                    if let Some(ref suggestion) = check.suggestion {
                        println!("       \x1b[36m→ {}\x1b[0m", suggestion);
                    }
                }
                println!();
            }

            // Summary
            println!("  \x1b[1mSummary\x1b[0m");
            println!("    {} passed, {} warnings, {} failed, {} info",
                report.summary.passed,
                report.summary.warnings,
                report.summary.failed,
                report.summary.info,
            );

            if report.summary.failed > 0 {
                println!("\n  \x1b[31m{} issue(s) need attention\x1b[0m\n", report.summary.failed);
            } else if report.summary.warnings > 0 {
                println!("\n  \x1b[33m{} warning(s) — project should work but consider fixing\x1b[0m\n", report.summary.warnings);
            } else {
                println!("\n  \x1b[32mAll checks passed!\x1b[0m\n");
            }
        }

        Commands::Migrate { dry_run } => {
            println!("\n  \x1b[36mpledge migrate\x1b[0m — migrating config to Pledgepack\n");

            let root = std::path::Path::new(".");
            match migrate::migrate_config(root) {
                Ok(result) => {
                    println!("  \x1b[90mSource:\x1b[0m detected {}\n", result.config_path);

                    if !result.migrated_fields.is_empty() {
                        println!("  \x1b[32mMigrated fields:\x1b[0m");
                        for field in &result.migrated_fields {
                            println!("    • {}", field);
                        }
                    }

                    if !result.warnings.is_empty() {
                        println!("\n  \x1b[33mWarnings:\x1b[0m");
                        for warning in &result.warnings {
                            println!("    • {}", warning);
                        }
                    }

                    if dry_run {
                        println!("\n  \x1b[90mDry run — no files written\x1b[0m");
                        println!("\n  \x1b[1mGenerated pledge.config.ts:\x1b[0m\n");
                        println!("{}", result.config_content);
                    } else {
                        let out_path = root.join("pledge.config.ts");
                        std::fs::write(&out_path, &result.config_content)?;
                        println!("\n  \x1b[32m✓\x1b[0m Written to pledge.config.ts\n");
                        println!("  \x1b[90mNext: update package.json scripts to use `pledge dev` / `pledge build`\x1b[0m\n");
                    }
                }
                Err(e) => {
                    println!("\n  \x1b[31m✗\x1b[0m {}\n", e);
                    println!("  \x1b[90mSupported: vite.config.{{ts,js,mjs}}, webpack.config.{{ts,js,cjs,mjs}}, config-overrides.js, next.config.{{ts,js,mjs}}\x1b[0m\n");
                }
            }
        }

        Commands::Config => {
            println!("\n  \x1b[36mpledge config\x1b[0m — validating configuration\n");

            let root = std::path::Path::new(".");

            // Try to read the raw config file for field-level validation
            let config_files = [
                root.join("pledge.config.ts"),
                root.join("pledge.config.js"),
                root.join("pledge.config.json"),
                root.join("pledge.json"),
            ];

            let mut found_config = false;
            for config_path in &config_files {
                if config_path.exists() {
                    found_config = true;
                    let content = std::fs::read_to_string(config_path)?;

                    // Try to parse as JSON (for .json files)
                    let config_json = if config_path.extension().map(|e| e == "json").unwrap_or(false) {
                        serde_json::from_str(&content).ok()
                    } else {
                        // For TS/JS files, try to extract the config object
                        pledgepack_core::config::PledgeConfig::parse_ts_config(&content).ok()
                            .and_then(|c| serde_json::to_value(&c).ok())
                    };

                    if let Some(ref json) = config_json {
                        let errors = config_validate::validate_config_json(json);
                        if errors.is_empty() {
                            println!("  \x1b[32m✓\x1b[0m Config is valid — no issues found\n");
                        } else {
                            println!("  \x1b[33m{} validation issue(s) found:\x1b[0m\n", errors.len());
                            print!("{}", config_validate::format_errors(&errors));
                        }
                    } else {
                        println!("  \x1b[33m⚠\x1b[0m Could not parse config for validation — check syntax\n");
                    }
                    break;
                }
            }

            if !found_config {
                println!("  \x1b[33m⚠\x1b[0m No config file found — using defaults\n");
                println!("  \x1b[90mRun `pledge init` to generate a config file\x1b[0m\n");
            }

            // Show current effective config
            println!("  \x1b[1mCurrent configuration:\x1b[0m\n");
            println!("    Entry:          {:?}", config.entry);
            println!("    Framework:      {:?}", config.framework);
            println!("    Out dir:        {:?}", config.out_dir);
            println!("    Dev server:     {}:{}", config.dev_server.host, config.dev_server.port);
            println!("    HMR:            {}", if config.dev_server.hmr { "enabled" } else { "disabled" });
            println!("    Source maps:    {}", if config.source_maps { "enabled" } else { "disabled" });
            println!("    Cache:          {} ({:?})", if config.cache.enabled { "enabled" } else { "disabled" }, config.cache.dir);
            println!("    Env prefix:     {:?}", config.env_prefix);
            println!("    Plugins:        {} configured", config.plugins.len());
            if let Some(ref edge) = config.edge_target {
                println!("    Edge target:    {}", edge);
            }
            println!();
        }

        Commands::Serve { port } => {
            let port = port.unwrap_or(4000);
            let out_dir = config.out_dir.clone();

            if !out_dir.exists() {
                println!("\n  \x1b[33mpledge serve\x1b[0m — .pledge/ not found. Run `pledge build` first.\n");
                return Ok(());
            }

            println!(
                "\n  \x1b[36mpledge serve\x1b[0m — serving {} on http://localhost:{}\n",
                out_dir.display(), port
            );

            let app = axum::Router::new()
                .fallback_service(tower_http::services::ServeDir::new(&out_dir));

            let addr = format!("127.0.0.1:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }

        Commands::Cache { action } => match action {
            CacheAction::Clear => {
                let cache_dir = config.cache.dir.clone();
                if cache_dir.exists() {
                    std::fs::remove_dir_all(&cache_dir)?;
                    println!("\n  \x1b[32m✓\x1b[0m Cache cleared: {:?}\n", cache_dir);
                } else {
                    println!("\n  \x1b[90mCache directory does not exist\x1b[0m\n");
                }
            }
            CacheAction::Stats => {
                let cache_dir = config.cache.dir.clone();
                if cache_dir.exists() {
                    let entries = std::fs::read_dir(&cache_dir)?
                        .filter_map(|e| e.ok())
                        .count();
                    println!("\n  Cache: {:?}\n  Entries: {}\n", cache_dir, entries);
                } else {
                    println!("\n  \x1b[90mNo cache directory\x1b[0m\n");
                }
            }
        },

        Commands::Bench { baseline, threshold } => {
            println!("\n  \x1b[36mpledge bench\x1b[0m — benchmarking build performance\n");

            config.mode = pledgepack_core::config::BuildMode::Production;

            let runs = 5;
            let mut times: Vec<u128> = Vec::new();

            for i in 0..runs {
                // Clear in-memory cache between runs by creating a new engine
                // (disk cache stays warm to measure incremental performance)
                let mut engine = BuildEngine::new(Arc::new(config.clone()));
                let result = engine.build().await?;

                let ms = result.duration_ms;
                times.push(ms);

                println!(
                    "  Run {}/{}: {} modules ({} cached) in {}ms",
                    i + 1, runs, result.modules_built, result.modules_cached, ms
                );
            }

            // Calculate statistics
            times.sort();
            let min = times.first().unwrap_or(&0);
            let max = times.last().unwrap_or(&0);
            let avg: u128 = times.iter().sum::<u128>() / times.len() as u128;
            let median = times[times.len() / 2];

            println!("\n  \x1b[32mBenchmark Results\x1b[0m ({} runs)\n", runs);
            println!("    Min:    {}ms", min);
            println!("    Max:    {}ms", max);
            println!("    Avg:    {}ms", avg);
            println!("    Median: {}ms\n", median);

            // Performance regression detection (#103)
            if let Some(ref baseline_ref) = baseline {
                let threshold = threshold.unwrap_or(10.0);
                let report = pledgepack_core::bench::compare_with_baseline(
                    &config.root,
                    baseline_ref,
                    median,
                    threshold,
                )?;

                if let Some(ref r) = report {
                    println!("{}", r.format());
                } else {
                    println!("  \x1b[32m✓ No regression\x1b[0m baseline={} median={}ms", baseline_ref, median);
                }
            }

            // Record benchmark result
            let git_ref = std::process::Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "current".to_string());

            let _ = pledgepack_core::bench::record_bench(
                &config.root,
                &git_ref,
                median,
                times.len(),
                0,
            );
        }

        Commands::Analyze { port, graph } => {
            println!("\n  \x1b[36mpledge analyze\x1b[0m — analyzing bundle...\n");

            config.mode = pledgepack_core::config::BuildMode::Production;

            let mut engine = BuildEngine::new(Arc::new(config.clone()));
            let _ = engine.build().await?;

            let analysis = pledgepack_core::analyzer::analyze_build(&engine)?;
            let html_output = if graph {
                // Generate interactive dependency graph (#104)
                let cycles = pledgepack_core::analyzer::detect_circular_deps(&analysis);
                if !cycles.is_empty() {
                    println!("  \x1b[33m⚠ {} circular dependency(s) detected:\x1b[0m", cycles.len());
                    for cycle in &cycles {
                        println!("    \x1b[31m{}\x1b[0m", cycle.join(" → "));
                    }
                    println!();
                }
                pledgepack_core::analyzer::generate_dependency_graph_html(&analysis)
            } else {
                pledgepack_core::analyzer::generate_analysis_html(&analysis)
            };

            let port = port.unwrap_or(4200);

            println!("  \x1b[32m✓\x1b[0m {} modules, {:.1}KB total", analysis.total_modules,
                analysis.total_transformed_size as f64 / 1024.0);
            println!("\n  \x1b[90mAnalysis server: http://localhost:{}\x1b[0m\n", port);

            let app = axum::Router::new()
                .route("/", axum::routing::get(move || async move {
                    axum::response::Html(html_output.clone())
                }));

            let addr = format!("127.0.0.1:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }

        Commands::Test { pattern, watch, ui } => {
            println!("\n  \x1b[36mpledge test\x1b[0m — running tests...\n");

            let pattern = pattern.unwrap_or_else(|| {
                if config.test.include.len() == 1 {
                    config.test.include[0].clone()
                } else {
                    "**/*.{test,spec}.{ts,tsx,js,jsx}".to_string()
                }
            });
            let test_dir = config.root.join("src");

            // Find test files
            let mut test_files = Vec::new();
            collect_test_files(&test_dir, &pattern, &mut test_files)?;

            if test_files.is_empty() {
                println!("  \x1b[33mNo test files found matching: {}\x1b[0m\n", pattern);
                return Ok(());
            }

            println!("  Found {} test file(s)\n", test_files.len());

            let mut all_summaries: Vec<(String, pledgepack_js_plugin_host::test_runner::TestSummary)> = Vec::new();
            let mut total_passed = 0;
            let mut total_failed = 0;
            let mut total_skipped = 0;

            for test_file in &test_files {
                let rel = test_file.strip_prefix(&config.root)
                    .unwrap_or(test_file)
                    .to_string_lossy()
                    .replace('\\', "/");

                let summary = match pledgepack_js_plugin_host::test_runner::run_test_file_with_config(
                    test_file,
                    &config.test,
                    &config.root,
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        println!("  \x1b[31m✗\x1b[0m {} — error: {}", rel, e);
                        total_failed += 1;
                        continue;
                    }
                };

                if summary.results.is_empty() {
                    println!("  \x1b[90m○ {} (no tests)\x1b[0m", rel);
                    total_skipped += 1;
                    continue;
                }

                // Print suite header
                if !summary.results.is_empty() {
                    println!("\n  \x1b[90m{}\x1b[0m", rel);
                }

                for result in &summary.results {
                    let suite_label = if result.suite.is_empty() {
                        String::new()
                    } else {
                        format!(" \x1b[90m{}\x1b[0m ›", result.suite)
                    };

                    match result.status {
                        pledgepack_js_plugin_host::test_runner::TestStatus::Passed => {
                            println!("    \x1b[32m✓\x1b[0m{} {} \x1b[90m({}ms)\x1b[0m",
                                suite_label, result.name, result.duration_ms);
                        }
                        pledgepack_js_plugin_host::test_runner::TestStatus::Failed => {
                            println!("    \x1b[31m✗\x1b[0m{} {}", suite_label, result.name);
                            if let Some(err) = &result.error {
                                println!("      \x1b[31m  {}\x1b[0m", err);
                            }
                        }
                        pledgepack_js_plugin_host::test_runner::TestStatus::Skipped => {
                            println!("    \x1b[90m○\x1b[0m{} {} \x1b[90m(skipped)\x1b[0m",
                                suite_label, result.name);
                        }
                    }
                }

                total_passed += summary.passed;
                total_failed += summary.failed;
                total_skipped += summary.skipped;
                all_summaries.push((rel, summary));
            }

            println!("\n  \x1b[32mTests:\x1b[0m  {} passed, {} skipped, {} failed\n",
                total_passed, total_skipped, total_failed);

            // Coverage report
            if config.test.coverage {
                println!("  \x1b[90mCoverage report ({}):\x1b[0m", config.test.coverage_reporter);
                println!("  \x1b[90m  Coverage data collected from {} file(s)\x1b[0m\n", all_summaries.len());
            }

            if watch {
                println!("  \x1b[90mWatch mode — press Ctrl+C to exit\x1b[0m\n");
                use notify::{Watcher, RecursiveMode, EventKind, recommended_watcher};
                use std::sync::mpsc::channel;
                use std::time::{Duration, Instant};

                let (tx, rx) = channel::<notify::Result<notify::Event>>();
                let mut watcher = recommended_watcher(move |res: notify::Result<notify::Event>| {
                    let _ = tx.send(res);
                })?;
                watcher.watch(&test_dir, RecursiveMode::Recursive)?;

                let debounce_ms = Duration::from_millis(300);
                let mut last_change: Option<Instant> = None;

                loop {
                    match rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(Ok(event)) => {
                            let is_relevant = matches!(
                                event.kind,
                                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                            );
                            if !is_relevant { continue; }
                            let has_src = event.paths.iter().any(|p| {
                                let s = p.to_string_lossy();
                                !s.contains("node_modules") && !s.contains(".pledge") && !s.contains("target")
                            });
                            if has_src {
                                last_change = Some(Instant::now());
                            }
                        }
                        Ok(Err(e)) => {
                            tracing::warn!("Watch error: {}", e);
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            if let Some(last) = last_change {
                                if last.elapsed() >= debounce_ms {
                                    println!("\n  \x1b[36mpledge test\x1b[0m — re-running tests...\n");
                                    total_passed = 0;
                                    total_failed = 0;
                                    total_skipped = 0;
                                    for test_file in &test_files {
                                        let rel = test_file.strip_prefix(&config.root)
                                            .unwrap_or(test_file)
                                            .to_string_lossy()
                                            .replace('\\', "/");
                                        let summary = match pledgepack_js_plugin_host::test_runner::run_test_file_with_config(
                                            test_file,
                                            &config.test,
                                            &config.root,
                                        ) {
                                            Ok(s) => s,
                                            Err(e) => {
                                                println!("  \x1b[31m✗\x1b[0m {} — error: {}", rel, e);
                                                total_failed += 1;
                                                continue;
                                            }
                                        };
                                        for result in &summary.results {
                                            match result.status {
                                                pledgepack_js_plugin_host::test_runner::TestStatus::Passed => {
                                                    println!("  \x1b[32m✓\x1b[0m {} › {}", result.suite, result.name);
                                                }
                                                pledgepack_js_plugin_host::test_runner::TestStatus::Failed => {
                                                    println!("  \x1b[31m✗\x1b[0m {} › {}", result.suite, result.name);
                                                    if let Some(err) = &result.error {
                                                        println!("    \x1b[31m{}\x1b[0m", err);
                                                    }
                                                }
                                                pledgepack_js_plugin_host::test_runner::TestStatus::Skipped => {
                                                    println!("  \x1b[90m○\x1b[0m {} › {} (skipped)", result.suite, result.name);
                                                }
                                            }
                                        }
                                        total_passed += summary.passed;
                                        total_failed += summary.failed;
                                        total_skipped += summary.skipped;
                                    }
                                    println!("\n  \x1b[32mTests:\x1b[0m  {} passed, {} skipped, {} failed\n",
                                        total_passed, total_skipped, total_failed);
                                    last_change = None;
                                }
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                            break;
                        }
                    }
                }
            }

            if ui {
                println!("  \x1b[36mUI mode\x1b[0m — generating test report...\n");

                let html = pledgepack_js_plugin_host::test_runner::generate_html_report(&all_summaries);
                let report_path = config.root.join(".pledge").join("test-report.html");
                std::fs::create_dir_all(report_path.parent().unwrap_or(&config.root))?;
                std::fs::write(&report_path, &html)?;

                println!("  \x1b[32m✓\x1b[0m Test report written to {}\n", report_path.display());

                // Serve the report on a local server
                let ui_port = 5174;
                let html_output = html.clone();
                let app = axum::Router::new()
                    .route("/", axum::routing::get(move || async move {
                        axum::response::Html(html_output.clone())
                    }));

                let addr = format!("127.0.0.1:{}", ui_port);
                println!("  \x1b[90mTest UI running at http://{}\x1b[0m\n", addr);

                // Auto-open browser
                #[cfg(target_os = "windows")]
                std::process::Command::new("cmd").args(["/C", "start", "", &format!("http://{}", addr)]).spawn().ok();

                let listener = tokio::net::TcpListener::bind(&addr).await?;
                axum::serve(listener, app).await?;
            }
        }

        Commands::Dashboard { port } => {
            let port = port.unwrap_or(4300);
            println!("\n  \x1b[36mpledge dashboard\x1b[0m — build telemetry\n");

            let history = pledgepack_core::telemetry::BuildHistory::load(&config.root)?;

            if history.builds.is_empty() {
                println!("  \x1b[33mNo build history found\x1b[0m — run `pledge build` first\n");
                return Ok(());
            }

            let recent = history.recent(10);
            println!("  \x1b[90m{} build(s) recorded\x1b[0m\n", history.builds.len());
            for r in recent.iter().rev() {
                let status = if r.success { "\x1b[32m✓\x1b[0m" } else { "\x1b[31m✗\x1b[0m" };
                println!("    {} {}ms — {} modules, {:.0}% cache hit",
                    status, r.duration_ms, r.modules_built + r.modules_cached,
                    r.cache_hit_rate * 100.0,
                );
            }

            let html_output = pledgepack_core::telemetry::generate_dashboard_html(&history);
            println!("\n  \x1b[90mDashboard: http://localhost:{}\x1b[0m\n", port);

            let app = axum::Router::new()
                .route("/", axum::routing::get(move || async move {
                    axum::response::Html(html_output.clone())
                }));

            let addr = format!("127.0.0.1:{}", port);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }

        Commands::GenerateEnvTypes => {
            println!("\n  \x1b[36mpledge generate-env-types\x1b[0m — generating .d.ts for import.meta.env\n");

            let env = EnvVars::load(&config.root, config.mode, &config.env_prefix);
            let dts = env.generate_dts(&config.env_prefix);

            let dts_path = config.root.join("pledge-env.d.ts");
            std::fs::write(&dts_path, &dts)?;

            println!("  \x1b[32m✓\x1b[0m Generated {}\n", dts_path.display());
        }

        Commands::Completions { shell } => {
            use clap_complete::Shell;
            use clap::CommandFactory;

            let shell_name = match shell {
                Some(s) => s,
                None => {
                    let options = vec!["bash", "zsh", "fish", "powershell", "elvish"];
                    let ans = inquire::Select::new("Select shell", options).prompt();
                    match ans {
                        Ok(s) => s.to_string(),
                        Err(_) => {
                            println!("\n  \x1b[33mCancelled.\x1b[0m\n");
                            return Ok(());
                        }
                    }
                }
            };

            let shell_enum = match shell_name.as_str() {
                "bash" => Shell::Bash,
                "zsh" => Shell::Zsh,
                "fish" => Shell::Fish,
                "powershell" | "pwsh" => Shell::PowerShell,
                "elvish" => Shell::Elvish,
                other => {
                    println!("\n  \x1b[31mError\x1b[0m Unknown shell: {}\n", other);
                    println!("  Supported shells: bash, zsh, fish, powershell, elvish\n");
                    return Ok(());
                }
            };

            let mut cmd = Cli::command();
            let bin_name = "pledge";
            println!("\n  \x1b[36mpledge completions\x1b[0m — generating {} completions\n", shell_name);
            clap_complete::generate(shell_enum, &mut cmd, bin_name, &mut std::io::stdout());
            println!("\n  \x1b[32m✓\x1b[0m Add the output to your shell's completion directory or source it in your rc file.\n");
        }
    }

    Ok(())
}

/// Recursively collect test files matching a pattern
fn collect_test_files(dir: &std::path::Path, _pattern: &str, files: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Skip node_modules and hidden dirs
            if path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == "node_modules" || n.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }
            collect_test_files(&path, _pattern, files)?;
        } else if path.is_file() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.contains(".test.") || name.contains(".spec.") {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "ts" | "tsx" | "js" | "jsx") {
                    files.push(path);
                }
            }
        }
    }

    Ok(())
}
