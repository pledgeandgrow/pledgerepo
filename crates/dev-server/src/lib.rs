// Dev server with HMR support
//
// Serves modules on-demand (lazy bundling like Turbopack):
//   1. Browser requests / → serve index.html
//   2. index.html loads /src/index.tsx → transform on-the-fly with Oxc
//   3. Import specifiers rewritten to browser-compatible URLs
//   4. File changes → notify watcher → WebSocket push → HMR update

use anyhow::Result;
use axum::{
    extract::{Path, State, WebSocketUpgrade, ws::Message},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use pledgepack_core::{BuildEngine, PledgeConfig};
use pledgepack_core::module::ModuleKind;
use pledgepack_core::transform as pledge_transform;
use pledgepack_js_plugin_host::JsPluginHost;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::info;

mod watcher;
mod hmr_diff;
mod lazy_pipeline;
mod middleware;
mod shell_generator;

/// A TLS listener that wraps a TCP listener with tokio-rustls
struct TlsListener {
    listener: tokio::net::TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
}

impl axum::serve::Listener for TlsListener {
    type Io = tokio_rustls::server::TlsStream<tokio::net::TcpStream>;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    match self.acceptor.accept(stream).await {
                        Ok(tls_stream) => return (tls_stream, addr),
                        Err(e) => {
                            tracing::warn!("TLS accept error: {}", e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("TCP accept error: {}", e);
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}

pub struct DevServerState {
    pub engine: RwLock<BuildEngine>,
    pub config: Arc<PledgeConfig>,
    pub hmr_tx: mpsc::UnboundedSender<HmrUpdate>,
    pub hmr_clients: RwLock<Vec<mpsc::UnboundedSender<HmrUpdate>>>,
    /// Import graph: module path → set of modules that import it (dependents)
    pub import_graph: RwLock<std::collections::HashMap<String, Vec<String>>>,
    /// Lazy-initialized transform pipeline (cold boot optimization)
    pub lazy_pipeline: RwLock<lazy_pipeline::LazyPipeline>,
    /// Module cache: path → last transformed output (for HMR diff computation)
    pub module_cache: RwLock<std::collections::HashMap<String, String>>,
    /// Import patterns per module: path → sorted import specifiers (for on-demand optimization)
    pub import_patterns: RwLock<std::collections::HashMap<String, Vec<String>>>,
    /// Middleware chain for request processing
    pub middleware_chain: RwLock<Vec<middleware::MiddlewareFn>>,
    /// Multi-entry HTML files: entry name → HTML content
    pub entries: RwLock<Vec<EntryConfig>>,
}

/// Configuration for a multi-entry dev server
#[derive(Debug, Clone)]
pub struct EntryConfig {
    /// Entry name (e.g., "index", "admin", "mobile")
    pub name: String,
    /// HTML file path relative to root
    pub html_file: String,
    /// JS entry module path
    pub entry_module: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HmrUpdate {
    #[serde(rename = "type")]
    pub update_type: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub css: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deps: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_reload: Option<bool>,
    /// Partial update: line-level diff for HMR (feature 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<hmr_diff::LineDiff>,
    /// Full module code for fallback when diff can't be applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_code: Option<String>,
}

pub async fn serve(engine: BuildEngine, config: &PledgeConfig) -> Result<()> {
    let port = config.dev_server.port;
    let host = config.dev_server.host.clone();

    let (hmr_tx, hmr_rx) = mpsc::unbounded_channel::<HmrUpdate>();

    // Start native file watcher if HMR is enabled
    if config.dev_server.hmr {
        let watch_root = config.root.clone();
        let tx = hmr_tx.clone();
        let server_entry = config.server_entry.clone();
        tokio::spawn(async move {
            start_native_file_watcher(watch_root, tx, server_entry);
        });
    }

    // Build middleware chain from config
    let middleware_fns: Vec<middleware::MiddlewareFn> = config.dev_server.middleware
        .iter()
        .filter_map(|src| middleware::MiddlewareFn::from_source(src))
        .collect();
    if !middleware_fns.is_empty() {
        info!("Middleware chain: {} functions registered", middleware_fns.len());
    }

    // Detect multi-entry HTML files
    let entries = detect_entries(config);
    if entries.len() > 1 {
        info!("Multi-entry dev server: {} entries detected", entries.len());
        for entry in &entries {
            info!("  Entry '{}': {} → {}", entry.name, entry.html_file, entry.entry_module);
        }
    }

    let state = Arc::new(DevServerState {
        engine: RwLock::new(engine),
        config: config.clone().into(),
        hmr_tx,
        hmr_clients: RwLock::new(Vec::new()),
        import_graph: RwLock::new(std::collections::HashMap::new()),
        lazy_pipeline: RwLock::new(lazy_pipeline::LazyPipeline::new()),
        module_cache: RwLock::new(std::collections::HashMap::new()),
        import_patterns: RwLock::new(std::collections::HashMap::new()),
        middleware_chain: RwLock::new(middleware_fns),
        entries: RwLock::new(entries),
    });

    // Spawn HMR broadcast task
    let hmr_state = state.clone();
    tokio::spawn(async move {
        hmr_broadcast_loop(hmr_state, hmr_rx).await;
    });

    // Build router with all stateful routes first
    let mut app = Router::new()
        .route("/", get(index_handler))
        .route("/__pledge_hmr", get(hmr_websocket_handler))
        .route("/__pledge_error", get(error_overlay_handler))
        .route("/__pledge_router", get(router_handler))
        .route("/__pledge_entry", get(entry_module_handler))
        .route("/__pledge_shell", get(shell_preview_handler))
        .route("/@fs/{*path}", get(virtual_fs_handler))
        .route("/@id/{*path}", get(virtual_id_handler))
        .route("/__pledge_public/{*path}", get(public_dir_handler));

    // Add multi-entry routes (e.g., /admin, /mobile) if configured
    {
        let entries = state.entries.read().await;
        for entry in entries.iter() {
            if entry.name != "index" {
                let entry_path = format!("/{}", entry.name);
                app = app.route(&entry_path, get(entry_index_handler));
                info!("Multi-entry route: {} → {}", entry_path, entry.html_file);
            }
        }
    }

    // Add catch-all route last
    let app = app
        .route("/{*path}", get(app_route_handler))
        .with_state(state.clone());

    // Apply HTTP compression middleware (feature 12: WebSocket compression via per-message deflate
    // is handled at the WebSocket upgrade level; this handles HTTP response compression)
    let mut app = app.layer(tower_http::compression::CompressionLayer::new()
        .gzip(true)
        .quality(tower_http::CompressionLevel::Fastest));

    // Execute configureServer hooks from JS plugins
    let plugins_dir = config.root.join("plugins");
    if plugins_dir.is_dir() {
        if let Ok(mut plugin_host) = JsPluginHost::load_from_dir(&plugins_dir) {
            let middlewares = plugin_host.configure_server();
            for mw in &middlewares {
                info!("[plugin:{}] configureServer registered middleware ({} bytes)", mw.plugin_name, mw.source.len());
            }
        }
    }

    // Log configured middleware (from config.dev_server.middleware)
    for (i, mw_source) in config.dev_server.middleware.iter().enumerate() {
        info!("Middleware #{} configured ({} bytes)", i, mw_source.len());
    }

    // Add proxy routes if configured
    for proxy in &config.proxy {
        let proxy_target = proxy.target.clone();
        let proxy_rewrite = proxy.rewrite;
        let proxy_path = proxy.path.clone();
        let proxy_headers = proxy.headers.clone();
        info!("Proxy: {} → {}{}", proxy_path, proxy_target, if proxy_rewrite { " (rewrite)" } else { "" });
        let proxy_router = Router::new().route(
            &format!("/{}/*rest", proxy_path.trim_start_matches('/')),
            axum::routing::any(move |method: axum::http::Method, Path(rest): Path<String>, body: axum::body::Body| {
                let target = proxy_target.clone();
                let path_prefix = proxy_path.clone();
                let rewrite = proxy_rewrite;
                let headers = proxy_headers.clone();
                async move {
                    proxy_handler(method, &rest, &target, &path_prefix, rewrite, &headers, body).await
                }
            }),
        );
        app = app.merge(proxy_router);

        // Add WebSocket proxy route if ws is enabled
        if proxy.ws {
            let ws_target = proxy.target.clone();
            let ws_rewrite = proxy.rewrite;
            let ws_path = proxy.path.clone();
            info!("WS Proxy: {} → {}", ws_path, ws_target);
            let ws_router = Router::new().route(
                &format!("/{}/*rest", ws_path.trim_start_matches('/')),
                get(move |ws: axum::extract::WebSocketUpgrade, Path(rest): Path<String>| {
                    let target = ws_target.clone();
                    let rewrite = ws_rewrite;
                    let path_prefix = ws_path.clone();
                    async move {
                        ws.on_upgrade(move |socket| {
                            let rest = rest.clone();
                            let target = target.clone();
                            let path_prefix = path_prefix.clone();
                            async move { ws_proxy_handler(socket, &rest, &target, &path_prefix, rewrite).await }
                        })
                    }
                }),
            );
            app = app.merge(ws_router);
        }
    }

    let addr = format!("{}:{}", host, port);

    // Auto-open browser if configured
    if config.dev_server.open {
        let protocol = if config.https.is_some() { "https" } else { "http" };
        let url = format!("{}://{}", protocol, addr);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            open_browser(&url);
        });
    }

    // HTTPS support
    if let Some(ref https_config) = config.https {
        info!("Dev server running at https://{}", addr);
        if let Ok(ip) = local_ip_address::local_ip() {
            info!("  → Network: https://{}:{}", ip, port);
        }
        let cert_path = &https_config.cert;
        let key_path = &https_config.key;

        if !cert_path.exists() || !key_path.exists() {
            info!("HTTPS enabled but cert/key not found — generating self-signed certificate...");
            generate_self_signed_cert(cert_path, key_path)?;
            info!("Self-signed certificate generated at {:?}", cert_path);
        }

        // Use tokio-rustls for TLS
        let cert = match std::fs::read(cert_path) {
            Ok(c) => c,
            Err(e) => anyhow::bail!("Failed to read cert: {}", e),
        };
        let key = match std::fs::read(key_path) {
            Ok(k) => k,
            Err(e) => anyhow::bail!("Failed to read key: {}", e),
        };

        // Parse cert and key
        let cert_chain = rustls_pemfile::certs(&mut cert.as_slice())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("Failed to parse cert: {}", e))?;
        let key_der = rustls_pemfile::private_key(&mut key.as_slice())
            .map_err(|e| anyhow::anyhow!("Failed to parse key: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("No private key found in key file"))?;

        let mut tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain.into_iter().map(rustls::pki_types::CertificateDer::from).collect(), key_der)
            .map_err(|e| anyhow::anyhow!("Failed to build TLS config: {}", e))?;
        tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        let tls_acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(tls_config));
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        
        // Serve with TLS using a custom Listener implementation
        let tls_listener = TlsListener {
            listener,
            acceptor: tls_acceptor,
        };
        axum::serve(tls_listener, app).await?;
    } else {
        info!("Dev server running at http://{}", addr);
        if let Ok(ip) = local_ip_address::local_ip() {
            info!("  → Network: http://{}:{}", ip, port);
        }
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}

/// Serve the virtual router module for file-based routing in dev mode
async fn router_handler(State(state): State<Arc<DevServerState>>) -> Response {
    // Scan the app directory and generate the router module
    if let Some(app_dir) = state.config.resolve_app_dir() {
        match pledgepack_core::router::scan_app_dir(&state.config.root, &app_dir) {
            Ok(route_table) => {
                let router_module = route_table.generate_router_module();
                return (
                    [
                        (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
                        (header::CACHE_CONTROL, "no-cache"),
                    ],
                    router_module,
                ).into_response();
            }
            Err(e) => {
                let error_body = format!(
                    "console.error('[pledge] Router generation error: {}');\nexport function render() {{ return null; }}",
                    e
                );
                return (
                    [
                        (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
                        (header::CACHE_CONTROL, "no-cache"),
                    ],
                    error_body,
                ).into_response();
            }
        }
    }

    // No app directory — return a minimal router that renders nothing
    (
        [
            (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        "export function render() { return null; }".to_string(),
    ).into_response()
}

/// Serve the auto-generated entry module (replaces static entry.tsx)
async fn entry_module_handler() -> Response {
    let entry_code = shell_generator::generate_entry_module();
    (
        [
            (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        entry_code,
    ).into_response()
}

/// Shell preview endpoint — shows the generated HTML shell for debugging
async fn shell_preview_handler(State(state): State<Arc<DevServerState>>) -> Response {
    let (html_attrs, head_content) = match shell_generator::try_extract_shell_from_project(&state.config.root) {
        Some((attrs, head)) => (attrs, head),
        None => ("lang=\"en\"".to_string(), "<title>PledgeStack</title>".to_string()),
    };

    let import_map = generate_import_map(&state.config);

    // Show the raw shell (without HMR script) for inspection
    let shell = shell_generator::generate_html_shell(
        &html_attrs,
        &head_content,
        "<!-- HMR script injected here -->",
        &import_map,
    );

    let escaped = shell
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"><title>Pledge Shell Preview</title>
<style>
  body {{ font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; background: #1a1a1a; color: #e0e0e0; padding: 2rem; }}
  h1 {{ color: #6c63ff; }}
  pre {{ background: #0d0d0d; border: 1px solid #333; border-radius: 8px; padding: 1.5rem; overflow: auto; font-size: 0.85rem; line-height: 1.6; }}
  .info {{ color: #888; margin-bottom: 1rem; }}
</style>
</head>
<body>
  <h1>Pledge Shell Preview</h1>
  <div class="info">This is the auto-generated HTML shell from layout.tsx. HMR script is shown as a comment.</div>
  <pre>{}</pre>
</body>
</html>"#,
        escaped
    );

    Html(html).into_response()
}

/// Serve the index.html shell
async fn index_handler(State(state): State<Arc<DevServerState>>) -> impl IntoResponse {
    // Convention: auto-detect project structure
    // Priority: app/ → src/app/ → src/ → root
    let (html_path, base_dir) = if let Some(entry) = &state.config.html_entry {
        (state.config.root.join(entry), None)
    } else {
        let base = state.config.resolve_base_dir();
        let html = match &base {
            Some(b) => state.config.root.join(b).join("index.html"),
            None => state.config.root.join("index.html"),
        };
        (html, base)
    };

    let mut html = match std::fs::read_to_string(&html_path) {
        Ok(content) => content,
        Err(_) => {
            // Auto-generate HTML shell from layout.tsx (no static index.html needed)
            let (html_attrs, head_content) = match shell_generator::try_extract_shell_from_project(&state.config.root) {
                Some((attrs, head)) => (attrs, head),
                None => ("lang=\"en\"".to_string(), "<title>PledgeStack</title>".to_string()),
            };
            let import_map = generate_import_map(&state.config);
            shell_generator::generate_html_shell(&html_attrs, &head_content, "", &import_map)
        }
    };

    // Inject HMR client script before </body>
    let hmr_script = r#"
    <script>
        // HMR runtime registries for framework component hot replacement
        window.__pledge_vue_components = window.__pledge_vue_components || {};
        window.__pledge_svelte_components = window.__pledge_svelte_components || {};
        window.__pledge_solid_hmr = window.__pledge_solid_hmr || [];
        window.__pledge_fast_refresh = window.__pledge_fast_refresh || {};
        // HMR module registry: path -> module hot data
        window.__pledge_hmr_modules = window.__pledge_hmr_modules || {};

        // WebSocket with exponential backoff reconnection
        let __pledge_ws;
        let __pledge_ws_reconnect_delay = 1000;
        let __pledge_ws_max_delay = 30000;
        let __pledge_ws_current_delay = __pledge_ws_reconnect_delay;
        let __pledge_ws_closed_by_user = false;

        function __pledge_connect_ws() {
            const ws = new WebSocket('ws://' + location.host + '/__pledge_hmr');
            window.__pledge_ws = ws;

            ws.onmessage = (event) => {
                const data = JSON.parse(event.data);
                if (data.type === 'update') {
                    console.log('[pledge] HMR update:', data.path, data.deps && data.deps.length ? '(cascading to: ' + data.deps.join(', ') + ')' : '');
                    clearPledgeError();
                    // Reset reconnection delay on successful message
                    __pledge_ws_current_delay = __pledge_ws_reconnect_delay;
                    if (data.path) {
                        // CSS HMR: inject <style> tag without page reload
                        if (data.path.endsWith('.css') || data.css) {
                            if (data.css) {
                                updatePledgeCSS(data.path, data.css);
                            } else {
                                fetchPledgeCSS(data.path);
                            }
                        } else {
                            // Framework HMR: .vue and .svelte use dynamic import for component replacement
                            if (data.path.endsWith('.vue') || data.path.endsWith('.svelte')) {
                                import(data.path + '?t=' + Date.now()).then((newModule) => {
                                    console.log('[pledge] Framework HMR:', data.path);
                                    window.dispatchEvent(new CustomEvent('pledge:hmr-success'));
                                }).catch((err) => {
                                    console.error('[pledge] Framework HMR failed:', err);
                                    location.reload();
                                });
                            } else {
                                // JS HMR: reload the changed script tag
                                const links = document.querySelectorAll('script[src="' + data.path + '"]');
                                links.forEach(link => {
                                    const newLink = document.createElement('script');
                                    newLink.type = 'module';
                                    newLink.src = data.path + '?t=' + Date.now();
                                    link.replaceWith(newLink);
                                });
                            }
                        }
                        // Handle cascading updates for dependent modules
                        if (data.deps && data.deps.length > 0) {
                            data.deps.forEach((depPath) => {
                                console.log('[pledge] HMR cascade:', depPath);
                                if (depPath.endsWith('.css')) {
                                    fetchPledgeCSS(depPath);
                                } else {
                                    const depLinks = document.querySelectorAll('script[src="' + depPath + '"]');
                                    depLinks.forEach(link => {
                                        const newLink = document.createElement('script');
                                        newLink.type = 'module';
                                        newLink.src = depPath + '?t=' + Date.now();
                                        link.replaceWith(newLink);
                                    });
                                }
                            });
                        }
                    }
                } else if (data.type === 'error') {
                    showPledgeError(data.message, data.file, data.stack, data.line, data.column);
                } else if (data.type === 'connected') {
                    console.log('[pledge] HMR connected');
                } else if (data.type === 'server-reload') {
                    console.log('[pledge] Server reloading:', data.message);
                    showPledgeServerReload(data.message);
                } else if (data.type === 'server-reload-complete') {
                    console.log('[pledge] Server reloaded:', data.message);
                    clearPledgeServerReload();
                }
            };
            ws.onopen = () => {
                console.log('[pledge] HMR connected');
                __pledge_ws_current_delay = __pledge_ws_reconnect_delay;
            };
            ws.onclose = () => {
                if (__pledge_ws_closed_by_user) return;
                console.warn('[pledge] HMR disconnected — reconnecting in ' + __pledge_ws_current_delay + 'ms');
                setTimeout(() => {
                    __pledge_ws_current_delay = Math.min(__pledge_ws_current_delay * 2, __pledge_ws_max_delay);
                    __pledge_connect_ws();
                }, __pledge_ws_current_delay);
            };
            ws.onerror = () => {
                // onclose will handle reconnection
            };
        }

        __pledge_connect_ws();

        // CSS HMR: update or inject <style> tag
        function updatePledgeCSS(path, cssContent) {
            let styleId = '__pledge_style_' + path.replace(/[^a-zA-Z0-9]/g, '_');
            let existing = document.getElementById(styleId);
            if (!existing) {
                existing = document.createElement('style');
                existing.id = styleId;
                document.head.appendChild(existing);
            }
            existing.textContent = cssContent;
            console.log('[pledge] CSS HMR:', path);
        }

        // CSS HMR: fetch updated CSS and inject
        async function fetchPledgeCSS(path) {
            try {
                const res = await fetch(path + '?t=' + Date.now());
                const css = await res.text();
                updatePledgeCSS(path, css);
            } catch(e) {
                console.error('[pledge] CSS HMR fetch failed:', e);
            }
        }

        // Pledge Error Overlay — beautiful, interactive error display with stack traces
        function showPledgeError(message, file, stack, line, column) {
            let overlay = document.getElementById('__pledge_error_overlay');
            if (!overlay) {
                overlay = document.createElement('div');
                overlay.id = '__pledge_error_overlay';
                overlay.style.cssText = 'position:fixed;inset:0;z-index:99999;background:rgba(0,0,0,0.92);font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;padding:2rem;overflow:auto;display:flex;flex-direction:column;';
                document.body.appendChild(overlay);
            }
            let fileHtml = file ? '<div style="color:#888;margin-bottom:1rem;font-size:0.9rem;">' + file + (line ? ':' + line + (column ? ':' + column : '') : '') + '</div>' : '';
            // Try to extract source context from error message
            let sourceContext = '';
            let lines = message.split('\n');
            if (lines.length > 1) {
                sourceContext = lines.map((line, i) => {
                    let lineClass = line.includes('error') || line.includes('Error') ? 'color:#ff4444;' : 'color:#e0e0e0;';
                    return '<div style="' + lineClass + 'white-space:pre;padding-left:2rem;">' + line.replace(/</g, '&lt;').replace(/>/g, '&gt;') + '</div>';
                }).join('');
            } else {
                sourceContext = '<pre style="color:#fff;white-space:pre-wrap;font-size:0.9rem;">' + message.replace(/</g, '&lt;').replace(/>/g, '&gt;') + '</pre>';
            }
            // Stack trace section
            let stackHtml = '';
            if (stack) {
                let stackLines = stack.split('\n').map(s => {
                    return '<div style="color:#aaa;white-space:pre;padding-left:2rem;font-size:0.85rem;">' + s.replace(/</g, '&lt;').replace(/>/g, '&gt;') + '</div>';
                }).join('');
                stackHtml = '<div style="color:#888;font-size:0.8rem;margin-top:1rem;margin-bottom:0.5rem;">Stack Trace:</div>' +
                    '<div style="background:#111;border:1px solid #222;border-radius:8px;padding:1rem;margin-bottom:1rem;overflow:auto;">' + stackLines + '</div>';
            }
            overlay.innerHTML =
                '<div style="max-width:900px;margin:0 auto;flex:1;">' +
                '<div style="display:flex;align-items:center;gap:0.5rem;margin-bottom:1rem;">' +
                '<span style="color:#ff4444;font-size:1.5rem;">&#9888;</span>' +
                '<span style="color:#ff4444;font-size:1.3rem;font-weight:600;">Pledge Build Error</span>' +
                '</div>' +
                fileHtml +
                '<div style="background:#1a1a1a;border:1px solid #333;border-radius:8px;padding:1rem;margin-bottom:1rem;overflow:auto;">' + sourceContext + '</div>' +
                stackHtml +
                '<div style="color:#666;font-size:0.8rem;">Fix the error and save to reload. The overlay will disappear automatically.</div>' +
                '<button onclick="clearPledgeError()" style="margin-top:1rem;padding:0.5rem 1rem;background:#333;color:#fff;border:1px solid #555;border-radius:4px;cursor:pointer;font-family:inherit;">Close</button>' +
                '</div>';
        }
        function clearPledgeError() {
            let overlay = document.getElementById('__pledge_error_overlay');
            if (overlay) overlay.remove();
        }
        function showPledgeServerReload(message) {
            let banner = document.getElementById('__pledge_server_reload');
            if (!banner) {
                banner = document.createElement('div');
                banner.id = '__pledge_server_reload';
                banner.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:99998;background:#1a1a2e;color:#7c3aed;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;padding:0.75rem;text-align:center;font-size:0.9rem;border-bottom:1px solid #7c3aed;';
                document.body.appendChild(banner);
            }
            banner.textContent = '⟳ ' + (message || 'Server reloading...');
        }
        function clearPledgeServerReload() {
            let banner = document.getElementById('__pledge_server_reload');
            if (banner) banner.remove();
        }
        // Listen for successful HMR updates to clear errors
        window.addEventListener('pledge:hmr-success', clearPledgeError);

        // Runtime error overlay — catch unhandled browser errors and display in overlay
        window.addEventListener('error', function(event) {
            if (event.error || event.message) {
                var msg = event.error && event.error.message ? event.error.message : (event.message || 'Unknown runtime error');
                var file = event.filename || '';
                var line = event.lineno || 0;
                var col = event.colno || 0;
                var stack = event.error && event.error.stack ? event.error.stack : '';
                showPledgeError(msg, file, stack, line, col);
            }
        });

        // Catch unhandled promise rejections and display in overlay
        window.addEventListener('unhandledrejection', function(event) {
            var reason = event.reason;
            var msg = reason && reason.message ? reason.message : String(reason);
            var stack = reason && reason.stack ? reason.stack : '';
            showPledgeError('Unhandled Promise Rejection: ' + msg, '', stack, 0, 0);
        });
    </script>
</body>"#;

    // Rewrite relative paths to absolute paths based on base_dir
    // e.g., ./index.tsx → /src/index.tsx, ./styles.css → /src/styles.css
    if let Some(base) = base_dir {
        let prefix = format!("/{}", base);
        // Rewrite src="./..." and href="./..." to absolute paths
        html = html.replace("src=\"./", &format!("src=\"{}/", prefix));
        html = html.replace("src='./", &format!("src='{}/", prefix));
        html = html.replace("href=\"./", &format!("href=\"{}/", prefix));
        html = html.replace("href='./", &format!("href='{}/", prefix));
    }

    // Inject import map for bare specifiers (react, react-dom, etc.)
    let import_map = generate_import_map(&state.config);
    if !import_map.is_empty() {
        let map_tag = format!("<script type=\"importmap\">\n{}\n</script>\n", import_map);
        if html.contains("</head>") {
            html = html.replace("</head>", &format!("{}\n</head>", map_tag));
        } else if html.contains("<body") {
            html = html.replace("<body", &format!("{}\n<body", map_tag));
        } else {
            html = format!("{}\n{}", map_tag, html);
        }
    }

    // Replace the closing </body> with HMR script + </body>
    if html.contains("</body>") {
        html = html.replace("</body>", &format!("{}\n</body>", hmr_script));
    } else {
        // No </body> tag, just append
        html.push_str(hmr_script);
        html.push_str("</body></html>");
    }

    Html(html)
}

/// Handle app-style routes: serve index.html for non-asset paths, fall back to module_handler for assets
async fn app_route_handler(
    State(state): State<Arc<DevServerState>>,
    Path(path): Path<String>,
) -> Response {
    // If the path looks like a static asset (has a file extension), serve it as a module
    let has_extension = path.rsplit('/').next()
        .map(|last| last.contains('.'))
        .unwrap_or(false);

    if has_extension {
        return module_handler(State(state), Path(path)).await;
    }

    // Non-asset path — serve the index.html shell for client-side routing
    // This enables both app-router and SPA-style routing
    let (html_path, base_dir) = if let Some(entry) = &state.config.html_entry {
        (state.config.root.join(entry), None)
    } else {
        let base = state.config.resolve_base_dir();
        let html = match &base {
            Some(b) => state.config.root.join(b).join("index.html"),
            None => state.config.root.join("index.html"),
        };
        (html, base)
    };

    let mut html = match std::fs::read_to_string(&html_path) {
        Ok(content) => content,
        Err(_) => {
            // Auto-generate HTML shell from layout.tsx (no static index.html needed)
            let (html_attrs, head_content) = match shell_generator::try_extract_shell_from_project(&state.config.root) {
                Some((attrs, head)) => (attrs, head),
                None => ("lang=\"en\"".to_string(), "<title>PledgeStack</title>".to_string()),
            };
            let import_map = generate_import_map(&state.config);
            shell_generator::generate_html_shell(&html_attrs, &head_content, "", &import_map)
        }
    };

    // Rewrite relative paths to absolute paths based on base_dir
    if let Some(base) = base_dir {
        let prefix = format!("/{}", base);
        html = html.replace("src=\"./", &format!("src=\"{}/", prefix));
        html = html.replace("src='./", &format!("src='{}/", prefix));
        html = html.replace("href=\"./", &format!("href=\"{}/", prefix));
        html = html.replace("href='./", &format!("href='{}/", prefix));
    }

    // Inject import map for bare specifiers (react, react-dom, etc.)
    let import_map = generate_import_map(&state.config);
    if !import_map.is_empty() {
        let map_tag = format!("<script type=\"importmap\">\n{}\n</script>\n", import_map);
        if html.contains("</head>") {
            html = html.replace("</head>", &format!("{}\n</head>", map_tag));
        } else if html.contains("<body") {
            html = html.replace("<body", &format!("{}\n<body", map_tag));
        } else {
            html = format!("{}\n{}", map_tag, html);
        }
    }

    // Inject HMR client script before </body>
    let hmr_script = r#"
    <script>
        window.__pledge_vue_components = window.__pledge_vue_components || {};
        window.__pledge_svelte_components = window.__pledge_svelte_components || {};
        window.__pledge_solid_hmr = window.__pledge_solid_hmr || [];
        window.__pledge_fast_refresh = window.__pledge_fast_refresh || {};
        window.__pledge_hmr_modules = window.__pledge_hmr_modules || {};
        let __pledge_ws;
        let __pledge_ws_reconnect_delay = 1000;
        let __pledge_ws_max_delay = 30000;
        let __pledge_ws_current_delay = __pledge_ws_reconnect_delay;
        let __pledge_ws_closed_by_user = false;
        function __pledge_connect_ws() {
            const ws = new WebSocket('ws://' + location.host + '/__pledge_hmr');
            window.__pledge_ws = ws;
            ws.onmessage = (event) => {
                const data = JSON.parse(event.data);
                if (data.type === 'update') {
                    console.log('[pledge] HMR update:', data.path);
                    clearPledgeError();
                    __pledge_ws_current_delay = __pledge_ws_reconnect_delay;
                    if (data.path) {
                        if (data.path.endsWith('.css') || data.css) {
                            if (data.css) { updatePledgeCSS(data.path, data.css); }
                            else { fetchPledgeCSS(data.path); }
                        } else {
                            const links = document.querySelectorAll('script[src="' + data.path + '"]');
                            links.forEach(link => {
                                const newLink = document.createElement('script');
                                newLink.type = 'module';
                                newLink.src = data.path + '?t=' + Date.now();
                                link.replaceWith(newLink);
                            });
                        }
                        if (data.deps && data.deps.length > 0) {
                            data.deps.forEach((depPath) => {
                                if (depPath.endsWith('.css')) { fetchPledgeCSS(depPath); }
                                else {
                                    const depLinks = document.querySelectorAll('script[src="' + depPath + '"]');
                                    depLinks.forEach(link => {
                                        const newLink = document.createElement('script');
                                        newLink.type = 'module';
                                        newLink.src = depPath + '?t=' + Date.now();
                                        link.replaceWith(newLink);
                                    });
                                }
                            });
                        }
                    }
                } else if (data.type === 'error') {
                    showPledgeError(data.message, data.file, data.stack, data.line, data.column);
                } else if (data.type === 'connected') {
                    console.log('[pledge] HMR connected');
                } else if (data.type === 'server-reload') {
                    console.log('[pledge] Server reloading:', data.message);
                    showPledgeServerReload(data.message);
                } else if (data.type === 'server-reload-complete') {
                    console.log('[pledge] Server reloaded:', data.message);
                    clearPledgeServerReload();
                }
            };
            ws.onopen = () => { console.log('[pledge] HMR connected'); __pledge_ws_current_delay = __pledge_ws_reconnect_delay; };
            ws.onclose = () => {
                if (__pledge_ws_closed_by_user) return;
                console.warn('[pledge] HMR disconnected — reconnecting in ' + __pledge_ws_current_delay + 'ms');
                setTimeout(() => {
                    __pledge_ws_current_delay = Math.min(__pledge_ws_current_delay * 2, __pledge_ws_max_delay);
                    __pledge_connect_ws();
                }, __pledge_ws_current_delay);
            };
            ws.onerror = () => {};
        }
        __pledge_connect_ws();
        function updatePledgeCSS(path, cssContent) {
            let styleId = '__pledge_style_' + path.replace(/[^a-zA-Z0-9]/g, '_');
            let existing = document.getElementById(styleId);
            if (!existing) { existing = document.createElement('style'); existing.id = styleId; document.head.appendChild(existing); }
            existing.textContent = cssContent;
        }
        async function fetchPledgeCSS(path) {
            try { const res = await fetch(path + '?t=' + Date.now()); const css = await res.text(); updatePledgeCSS(path, css); }
            catch(e) { console.error('[pledge] CSS HMR fetch failed:', e); }
        }
        function showPledgeError(message, file, stack, line, column) {
            let overlay = document.getElementById('__pledge_error_overlay');
            if (!overlay) {
                overlay = document.createElement('div');
                overlay.id = '__pledge_error_overlay';
                overlay.style.cssText = 'position:fixed;inset:0;z-index:99999;background:rgba(0,0,0,0.92);font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;padding:2rem;overflow:auto;display:flex;flex-direction:column;';
                document.body.appendChild(overlay);
            }
            let fileHtml = file ? '<div style="color:#888;margin-bottom:1rem;font-size:0.9rem;">' + file + (line ? ':' + line + (column ? ':' + column : '') : '') + '</div>' : '';
            let sourceContext = '<pre style="color:#fff;white-space:pre-wrap;font-size:0.9rem;">' + message.replace(/</g, '&lt;').replace(/>/g, '&gt;') + '</pre>';
            overlay.innerHTML = '<div style="max-width:900px;margin:0 auto;flex:1;"><div style="display:flex;align-items:center;gap:0.5rem;margin-bottom:1rem;"><span style="color:#ff4444;font-size:1.5rem;">&#9888;</span><span style="color:#ff4444;font-size:1.3rem;font-weight:600;">Pledge Build Error</span></div>' + fileHtml + '<div style="background:#1a1a1a;border:1px solid #333;border-radius:8px;padding:1rem;margin-bottom:1rem;overflow:auto;">' + sourceContext + '</div><div style="color:#666;font-size:0.8rem;">Fix the error and save to reload.</div><button onclick="clearPledgeError()" style="margin-top:1rem;padding:0.5rem 1rem;background:#333;color:#fff;border:1px solid #555;border-radius:4px;cursor:pointer;font-family:inherit;">Close</button></div>';
        }
        function clearPledgeError() { let overlay = document.getElementById('__pledge_error_overlay'); if (overlay) overlay.remove(); }
        function showPledgeServerReload(message) { let b = document.getElementById('__pledge_server_reload'); if (!b) { b = document.createElement('div'); b.id = '__pledge_server_reload'; b.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:99998;background:#1a1a2e;color:#7c3aed;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;padding:0.75rem;text-align:center;font-size:0.9rem;border-bottom:1px solid #7c3aed;'; document.body.appendChild(b); } b.textContent = '⟳ ' + (message || 'Server reloading...'); }
        function clearPledgeServerReload() { let b = document.getElementById('__pledge_server_reload'); if (b) b.remove(); }
        window.addEventListener('pledge:hmr-success', clearPledgeError);
        window.addEventListener('error', function(event) {
            if (event.error || event.message) {
                var msg = event.error && event.error.message ? event.error.message : (event.message || 'Unknown runtime error');
                showPledgeError(msg, event.filename || '', event.error && event.error.stack ? event.error.stack : '', event.lineno || 0, event.colno || 0);
            }
        });
        window.addEventListener('unhandledrejection', function(event) {
            var reason = event.reason;
            showPledgeError('Unhandled Promise Rejection: ' + (reason && reason.message ? reason.message : String(reason)), '', reason && reason.stack ? reason.stack : '', 0, 0);
        });
    </script>
</body>"#;

    if html.contains("</body>") {
        html = html.replace("</body>", &format!("{}\n</body>", hmr_script));
    } else {
        html.push_str(hmr_script);
        html.push_str("</body></html>");
    }

    Html(html).into_response()
}

/// Generate an import map for bare specifiers (react, vue, etc.)
/// This allows the browser to resolve bare imports without a bundler.
/// For CJS-only packages (no ESM), use esm.sh CDN.
/// For ESM packages, serve locally from node_modules.
fn generate_import_map(config: &PledgeConfig) -> String {
    let node_modules = config.root.join("node_modules");
    let mut imports = serde_json::Map::new();

    // Known CJS-only packages that need esm.sh CDN
    let cjs_only_packages = ["react", "react-dom", "scheduler"];

    if node_modules.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&node_modules) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                if name.starts_with('@') {
                    let scoped_dir = entry.path();
                    if scoped_dir.is_dir() {
                        if let Ok(scoped_entries) = std::fs::read_dir(&scoped_dir) {
                            for se in scoped_entries.flatten() {
                                let sub_name = se.file_name().to_string_lossy().to_string();
                                let pkg_name = format!("{}/{}", name, sub_name);
                                let pkg_json = scoped_dir.join(&sub_name).join("package.json");
                                if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                                    if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                                        let has_esm = pkg.get("module").is_some()
                                            || pkg.get("type").and_then(|v| v.as_str()) == Some("module");
                                        if has_esm {
                                            let entry_field = pkg.get("module").or_else(|| pkg.get("main"))
                                                .and_then(|v| v.as_str()).unwrap_or("index.js");
                                            imports.insert(pkg_name, serde_json::Value::String(
                                                format!("/node_modules/{}/{}/{}", name, sub_name, entry_field)
                                                    .replace('\\', "/")
                                            ));
                                        } else {
                                            // CJS-only: use esm.sh
                                            imports.insert(pkg_name.clone(), serde_json::Value::String(
                                                format!("https://esm.sh/{}", pkg_name)
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    continue;
                }
                let pkg_json = node_modules.join(&name).join("package.json");
                if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                    if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                        let has_esm = pkg.get("module").is_some()
                            || pkg.get("type").and_then(|v| v.as_str()) == Some("module");

                        // Check if exports has an "import" condition
                        let has_esm_export = pkg.get("exports")
                            .and_then(|e| e.as_object())
                            .and_then(|o| o.get("."))
                            .and_then(|d| d.as_object())
                            .map(|d| d.contains_key("import") || d.contains_key("browser"))
                            .unwrap_or(false);

                        let is_cjs_only = cjs_only_packages.contains(&name.as_str())
                            || (!has_esm && !has_esm_export);

                        if is_cjs_only {
                            // CJS-only: use esm.sh CDN for all entry points
                            imports.insert(name.clone(), serde_json::Value::String(
                                format!("https://esm.sh/{}", name)
                            ));

                            // Add exports map entries via esm.sh
                            if let Some(exports) = pkg.get("exports") {
                                if let Some(obj) = exports.as_object() {
                                    for (export_key, _) in obj {
                                        if export_key == "." {
                                            continue;
                                        }
                                        let full_key = format!("{}/{}", name, export_key.trim_start_matches("./"));
                                        imports.insert(full_key, serde_json::Value::String(
                                            format!("https://esm.sh/{}/{}", name, export_key.trim_start_matches("./"))
                                        ));
                                    }
                                }
                            }
                        } else {
                            // ESM package: serve locally
                            let entry_field = pkg.get("module").or_else(|| pkg.get("main"))
                                .and_then(|v| v.as_str()).unwrap_or("index.js");
                            imports.insert(name.clone(), serde_json::Value::String(
                                format!("/node_modules/{}/{}", name, entry_field).replace('\\', "/")
                            ));

                            // Add exports map entries
                            if let Some(exports) = pkg.get("exports") {
                                if let Some(obj) = exports.as_object() {
                                    for (export_key, export_val) in obj {
                                        if export_key == "." {
                                            continue;
                                        }
                                        let resolved = if let Some(s) = export_val.as_str() {
                                            Some(s.to_string())
                                        } else if let Some(obj) = export_val.as_object() {
                                            obj.get("browser").or_else(|| obj.get("import"))
                                                .or_else(|| obj.get("default"))
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string())
                                        } else {
                                            None
                                        };
                                        if let Some(resolved_path) = resolved {
                                            let full_key = format!("{}/{}", name, export_key.trim_start_matches("./"));
                                            imports.insert(full_key, serde_json::Value::String(
                                                format!("/node_modules/{}/{}", name, resolved_path.trim_start_matches("./"))
                                                    .replace('\\', "/")
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Build scopes for multi-version deduplication.
    // Scans nested node_modules for different versions of the same package
    // and creates scoped import map entries so modules depending on different
    // versions resolve correctly.
    let scopes = build_import_map_scopes(&node_modules, &imports);

    if scopes.is_empty() {
        serde_json::json!({ "imports": imports }).to_string()
    } else {
        serde_json::json!({ "imports": imports, "scopes": scopes }).to_string()
    }
}

/// Build scoped import map entries for packages with multiple versions.
/// When a package has different versions in nested node_modules, we create
/// a scope entry for each parent module path so the browser resolves the
/// correct version.
fn build_import_map_scopes(
    root_node_modules: &std::path::Path,
    top_level_imports: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut scopes: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    let mut pkg_versions: HashMap<String, Vec<(String, String)>> = HashMap::new();

    // Walk all nested node_modules directories to find version conflicts
    fn scan_node_modules(
        dir: &std::path::Path,
        root: &std::path::Path,
        pkg_versions: &mut HashMap<String, Vec<(String, String)>>,
    ) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                if name.starts_with('@') {
                    let scoped_dir = entry.path();
                    if scoped_dir.is_dir() {
                        if let Ok(scoped_entries) = std::fs::read_dir(&scoped_dir) {
                            for se in scoped_entries.flatten() {
                                let sub_name = se.file_name().to_string_lossy().to_string();
                                let pkg_name = format!("{}/{}", name, sub_name);
                                let pkg_json = scoped_dir.join(&sub_name).join("package.json");
                                if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                                    if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                                        let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0");
                                        let rel_path = dir.strip_prefix(root).unwrap_or(dir).to_string_lossy().to_string();
                                        pkg_versions.entry(pkg_name).or_default().push((version.to_string(), rel_path));
                                    }
                                }
                            }
                        }
                    }
                    continue;
                }
                let pkg_json = dir.join(&name).join("package.json");
                if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                    if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                        let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("0.0.0");
                        let rel_path = dir.strip_prefix(root).unwrap_or(dir).to_string_lossy().to_string();
                        pkg_versions.entry(name).or_default().push((version.to_string(), rel_path));
                    }
                }
            }
        }

        // Recurse into subdirectories that contain nested node_modules
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.file_name().map(|n| n != "node_modules").unwrap_or(true) {
                    let nested_nm = path.join("node_modules");
                    if nested_nm.is_dir() {
                        scan_node_modules(&nested_nm, root, pkg_versions);
                    }
                }
            }
        }
    }

    scan_node_modules(root_node_modules, root_node_modules, &mut pkg_versions);

    // For packages with multiple versions, create scope entries
    for (pkg_name, versions) in &pkg_versions {
        let unique_versions: HashSet<&String> = versions.iter().map(|(v, _)| v).collect();
        if unique_versions.len() <= 1 {
            continue;
        }

        // Group by version, pick the first path for each version
        let mut by_version: HashMap<&String, &String> = HashMap::new();
        for (version, path) in versions {
            by_version.entry(version).or_insert(path);
        }

        // For each version, create a scope that maps the package to the correct path
        for (version, parent_path) in &by_version {
            let scope_key = format!("/node_modules/{}/", parent_path.trim_start_matches("node_modules/"));
            let scope_key = scope_key.replace("//", "/");

            let scope_entry = scopes
                .entry(scope_key.clone())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

            if let Some(scope_obj) = scope_entry.as_object_mut() {
                // Map this package to its nested version path
                let nested_path = format!("/node_modules/{}/{}/", parent_path.trim_start_matches("node_modules/"), pkg_name);
                let nested_path = nested_path.replace("//", "/");

                // Try to get the entry field from the nested package.json
                let nested_pkg_json = root_node_modules
                    .join(parent_path.trim_start_matches("node_modules/"))
                    .join(pkg_name)
                    .join("package.json");

                let entry_field = std::fs::read_to_string(&nested_pkg_json)
                    .ok()
                    .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                    .and_then(|p| p.get("module").or_else(|| p.get("main")).and_then(|v| v.as_str()).map(String::from))
                    .unwrap_or_else(|| "index.js".to_string());

                let full_path = format!("{}{}", nested_path, entry_field).replace('\\', "/");
                scope_obj.insert(pkg_name.clone(), serde_json::Value::String(full_path));
            }
        }
    }

    scopes
}

/// Error overlay endpoint — returns error info as JSON for programmatic access
async fn error_overlay_handler(
    State(_state): State<Arc<DevServerState>>,
) -> Response {
    // Return a simple page that can be used to display errors
    let html = r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"><title>Pledge Error</title></head>
<body style="background:#1a1a1a;color:#ff4444;font-family:monospace;padding:2rem;">
<h1>&#9888; Pledge Build Error</h1>
<p>Check the console for details.</p>
</body>
</html>"#;
    Html(html).into_response()
}

/// Serve a transformed module on-demand
async fn module_handler(
    State(state): State<Arc<DevServerState>>,
    Path(path): Path<String>,
) -> Response {
    // First, try serving from the configured public directory (static assets)
    let public_dir = &state.config.dev_server.public_dir;
    let public_path = state.config.root.join(public_dir).join(&path);
    if public_path.exists() && public_path.is_file() {
        if let Ok(content) = tokio::fs::read(&public_path).await {
            let content_type = guess_content_type(&path);
            return (
                [(header::CONTENT_TYPE, content_type)],
                content,
            ).into_response();
        }
    }

    let full_path = state.config.root.join(&path);

    // If the exact file doesn't exist, try alternative extensions
    // (e.g., /src/utils.js → /src/utils.ts, /src/index.js → /src/index.tsx)
    let full_path = if full_path.exists() {
        full_path
    } else {
        // Try replacing .js extension with source extensions
        let stem = full_path.with_extension("");
        let mut found = None;
        for ext in &["tsx", "ts", "jsx", "js", "mjs", "css", "json", "vue", "svelte"] {
            let candidate = stem.with_extension(ext);
            if candidate.exists() {
                found = Some(candidate);
                break;
            }
        }
        match found {
            Some(p) => p,
            None => return (StatusCode::NOT_FOUND, "Module not found").into_response(),
        }
    };

    // Read source via Zig I/O
    let source = match pledgepack_native_sys::read_file(full_path.to_str().unwrap_or("")) {
        Ok(content) => content,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read").into_response(),
    };

    let source_str = String::from_utf8_lossy(&source).to_string();

    // CJS → ESM conversion for node_modules files
    // Browser can't use require()/module.exports, so wrap them in ESM
    let (source_str, skip_transform) = if path.starts_with("node_modules/") {
        let is_cjs = source_str.contains("module.exports")
            || source_str.contains("require(")
            || source_str.contains("exports.");

        if is_cjs {
            let specifier = path.strip_prefix("node_modules/").unwrap_or(&path);
            let rewritten = rewrite_cjs_requires(&source_str, &path);
            let wrapped = pledgepack_core::dep_bundler::DepBundler::cjs_to_esm_wrapper(specifier, &rewritten);
            (wrapped, true)
        } else {
            (source_str, false)
        }
    } else {
        (source_str, false)
    };

    // For CJS-wrapped node_modules, skip Oxc transform and serve directly
    if skip_transform {
        let rewritten = rewrite_imports(&source_str, &path, &state.config.resolve_alias);
        return serve_js_module(&path, &rewritten, &state).await;
    }

    // Determine module kind from extension
    let ext_str = full_path.extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    let kind = ModuleKind::from_extension(&ext_str);

    // Transform using Oxc (JSX → JS, TS type stripping) or Lightning CSS
    // Uses lazy pipeline initialization (feature 11: cold boot optimization)
    // The transform pipeline (Oxc, Lightning CSS) is only loaded on first request
    let file_path = full_path.to_str().unwrap_or("");
    let transform_output = {
        let mut lazy_pipeline = state.lazy_pipeline.write().await;
        lazy_pipeline.ensure_initialized();
        match pledge_transform::transform(&source_str, kind, file_path, false, &state.config) {
            Ok(output) => output,
            Err(e) => {
                // Send error to all HMR clients via WebSocket
                let error_update = HmrUpdate {
                    update_type: "error".to_string(),
                    path: path.clone(),
                    message: Some(format!("{}", e)),
                    file: Some(file_path.to_string()),
                    css: None,
                    stack: Some(format!("{:?}", e)),
                    line: None,
                    column: None,
                    deps: Vec::new(),
                    full_reload: None,
                    diff: None,
                    full_code: None,
                };
                let _ = state.hmr_tx.send(error_update);
                // Also return an error response with proper content type
                let error_body = format!(
                    "/* Pledge Transform Error */\nconsole.error('[pledge] Transform error in {}: {}');\nthrow new Error('Transform error: {}');",
                    path,
                    e,
                    e
                );
                return (
                    [
                        (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
                        (header::CACHE_CONTROL, "no-cache"),
                    ],
                    error_body,
                ).into_response();
            }
        }
    };

    // Track import patterns for on-demand dependency optimization (feature 15)
    {
        let import_patterns = state.import_patterns.read().await;
        if !import_patterns.contains_key(&path) {
            drop(import_patterns);
            let mut import_patterns = state.import_patterns.write().await;
            import_patterns.insert(path.clone(), extract_imports(&source_str));
        }
    }

    // Cache the transformed module for HMR diff computation (feature 10)
    {
        let mut module_cache = state.module_cache.write().await;
        module_cache.insert(path.clone(), transform_output.code.clone());
    }

    // CSS files: serve as JS module with style injection for dev mode HMR
    if transform_output.is_css {
        let css_code = &transform_output.code;

        // CSS Modules: generate export object with scoped class names
        if let Some(ref css_module_map) = transform_output.css_modules {
            let mut exports = String::new();
            for (original, scoped) in css_module_map {
                exports.push_str(&format!("  {}: \"{}\",\n", original, scoped));
            }
            // Rewrite class names in CSS to scoped versions
            let mut scoped_css = css_code.clone();
            for (original, scoped) in css_module_map {
                let pattern = format!(".{}", original);
                let replacement = format!(".{}", scoped);
                scoped_css = scoped_css.replace(&pattern, &replacement);
            }
            let js_module = format!(
                r#"const __css = {};
const __styleId = '__pledge_style_' + {};
let __existing = document.getElementById(__styleId);
if (!__existing) {{
  __existing = document.createElement('style');
  __existing.id = __styleId;
  document.head.appendChild(__existing);
}}
__existing.textContent = __css;
export default {{}};
export {{}};
"#,
                serde_json::to_string(&scoped_css).unwrap_or_else(|_| "\"\"".to_string()),
                serde_json::to_string(&path).unwrap_or_else(|_| "\"\"".to_string())
            );
            // Replace the empty export with actual CSS module exports
            let js_module = js_module.replace(
                "export default {};\nexport {};\n",
                &format!("export default {{\n{}}};\n", exports)
            );
            return (
                [
                    (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
                    (header::CACHE_CONTROL, "no-cache"),
                ],
                js_module,
            ).into_response();
        }

        // Regular CSS: inject as JS module that creates a <style> tag
        let js_module = format!(
            r#"const __css = {};
const __styleId = '__pledge_style_' + {};
let __existing = document.getElementById(__styleId);
if (!__existing) {{
  __existing = document.createElement('style');
  __existing.id = __styleId;
  document.head.appendChild(__existing);
}}
__existing.textContent = __css;
// HMR: update style tag on hot reload
if (import.meta.hot) {{
  import.meta.hot.accept();
}}
"#,
            serde_json::to_string(css_code).unwrap_or_else(|_| "\"\"".to_string()),
            serde_json::to_string(&path).unwrap_or_else(|_| "\"\"".to_string())
        );

        return (
            [
                (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            js_module,
        ).into_response();
    }

    // Handle extracted CSS from SFCs (Vue/Svelte/Astro) — inject as style tag
    if let Some(ref extracted_css) = transform_output.extracted_css {
        let style_id = format!("__pledge_style_{}", path.replace(|c: char| !c.is_alphanumeric(), "_"));
        let css_inject = format!(
            r#"
const __css = {};
const __styleId = '{}';
let __existing = document.getElementById(__styleId);
if (!__existing) {{
  __existing = document.createElement('style');
  __existing.id = __styleId;
  document.head.appendChild(__existing);
}}
__existing.textContent = __css;
"#,
            serde_json::to_string(extracted_css).unwrap_or_else(|_| "\"\"".to_string()),
            style_id
        );
        // Prepend CSS injection to the module code
        let transformed = rewrite_imports(&format!("{}\n{}", css_inject, transform_output.code), &path, &state.config.resolve_alias);
        // Continue with normal JS module handling below using the combined code
        let transform_output = pledgepack_core::transform::TransformOutput {
            code: transformed,
            source_map: None,
            css_modules: None,
            is_css: false,
            extracted_css: None,
            is_worker: false,
            dynamic_imports: Vec::new(),
            content_hash: None,
        };
        // Fall through to JS handling by re-running the logic below
        return serve_js_module(&path, &transform_output.code, &state).await;
    }

    // JS/TS files: rewrite imports and add HMR boundary
    let transformed = rewrite_imports(&transform_output.code, &path, &state.config.resolve_alias);
    serve_js_module(&path, &transformed, &state).await
}

/// Serve a JS module with HMR polyfill, dependency tracking, and source maps
async fn serve_js_module(
    path: &str,
    transformed: &str,
    state: &Arc<DevServerState>,
) -> Response {
    // Track imports in the dependency graph for cascading HMR updates
    {
        let imports = extract_imports(transformed);
        if !imports.is_empty() {
            let mut graph = state.import_graph.write().await;
            for dep in &imports {
                let normalized = normalize_module_path(dep, path);
                graph.entry(normalized).or_default().push(path.to_string());
            }
        }
    }

    // Inject import.meta.hot polyfill with accept(), dispose(), invalidate(), and data
    let hmr_polyfill = format!(
        r#"
// Pledge HMR polyfill — import.meta.hot API
if (!import.meta.hot) {{
  const __pledge_hot_id = '{}';
  const __pledge_hot_data = {{}};
  const __pledge_hot_dispose_callbacks = [];
  const __pledge_hot_accept_callbacks = [];
  import.meta.hot = {{
    data: __pledge_hot_data,
    accept(cb) {{
      if (typeof cb === 'function') __pledge_hot_accept_callbacks.push(cb);
    }},
    dispose(cb) {{
      if (typeof cb === 'function') __pledge_hot_dispose_callbacks.push(cb);
    }},
    invalidate() {{
      console.log('[pledge] HMR invalidate:', __pledge_hot_id);
      window.__pledge_hmr_invalidate = true;
      location.reload();
    }},
    __run_dispose() {{
      __pledge_hot_dispose_callbacks.forEach(cb => {{
        try {{ cb(__pledge_hot_data); }} catch(e) {{ console.error('[pledge] HMR dispose error:', e); }}
      }});
      __pledge_hot_dispose_callbacks.length = 0;
    }},
    __run_accept(newModule) {{
      __pledge_hot_accept_callbacks.forEach(cb => {{
        try {{ cb(newModule); }} catch(e) {{ console.error('[pledge] HMR accept error:', e); }}
      }});
    }}
  }};
}}
"#,
        path
    );

    // Add HMR boundary code for JS/TS files
    let module_with_hmr = if path.ends_with(".tsx") || path.ends_with(".jsx") || path.ends_with(".ts") || path.ends_with(".js") {
        format!(
            "{}\n{}\nif (import.meta.hot) {{\nimport.meta.hot.accept();\n}}",
            hmr_polyfill,
            transformed
        )
    } else {
        format!("{}\n{}", hmr_polyfill, transformed)
    };

    (
        [
            (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        module_with_hmr,
    )
        .into_response()
}

/// Normalize a relative import specifier to an absolute module path
fn normalize_module_path(specifier: &str, importer: &str) -> String {
    if specifier.starts_with('/') {
        return specifier.trim_start_matches('/').to_string();
    }
    if specifier.starts_with("./") || specifier.starts_with("../") {
        // Resolve relative to importer's directory
        let importer_dir = importer.rfind('/').map(|i| &importer[..i]).unwrap_or("");
        let parts: Vec<&str> = specifier.split('/').collect();
        let mut path_parts: Vec<&str> = importer_dir.split('/').filter(|s| !s.is_empty()).collect();
        for part in parts {
            match part {
                "." => {}
                ".." => { path_parts.pop(); }
                _ => { path_parts.push(part); }
            }
        }
        return path_parts.join("/");
    }
    specifier.to_string()
}

/// Rewrite require() calls in CJS node_modules files to use the __pledge_require shim.
/// The CJS→ESM wrapper provides a `require` function that returns the cached module.
/// For relative requires like require("./cjs/react.development.js"), we need to resolve
/// them to absolute paths so the browser can fetch them as ESM modules.
fn rewrite_cjs_requires(source: &str, module_path: &str) -> String {
    // Get the directory of the current module for resolving relative requires
    let module_dir = module_path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");

    // Replace require("./...") and require("./...") with resolved absolute paths
    // The CJS wrapper already provides `require`, so we just need to make sure
    // relative paths resolve correctly. We convert relative require paths to
    // absolute paths from the server root.
    let mut result = source.to_string();

    // Pattern: require("./something") or require('../something')
    // Replace with require("/node_modules/<pkg>/something")
    while let Some(start) = result.find("require(\".") {
        if let Some(end) = result[start..].find("\")") {
            let quote_start = start + 8; // after require("
            let quote_end = start + end; // position of closing "
            let relative_path = &result[quote_start..quote_end];

            // Resolve relative to module directory
            let resolved = if relative_path.starts_with("./") {
                format!("/node_modules/{}/{}", module_dir, &relative_path[2..])
            } else if relative_path.starts_with("../") {
                // Go up one directory
                let parent_dir = module_dir.rsplit_once('/').map(|(d, _)| d).unwrap_or(module_dir);
                format!("/node_modules/{}/{}", parent_dir, &relative_path[3..])
            } else {
                relative_path.to_string()
            };

            result = format!("{}require(\"{}\"){}", &result[..start], resolved, &result[quote_end + 2..]);
        } else {
            break;
        }
    }

    // Also handle require('./...') with single quotes
    while let Some(start) = result.find("require('.") {
        if let Some(end) = result[start..].find("')") {
            let quote_start = start + 8;
            let quote_end = start + end;
            let relative_path = &result[quote_start..quote_end];

            let resolved = if relative_path.starts_with("./") {
                format!("/node_modules/{}/{}", module_dir, &relative_path[2..])
            } else if relative_path.starts_with("../") {
                let parent_dir = module_dir.rsplit_once('/').map(|(d, _)| d).unwrap_or(module_dir);
                format!("/node_modules/{}/{}", parent_dir, &relative_path[3..])
            } else {
                relative_path.to_string()
            };

            result = format!("{}require('{}'){}", &result[..start], resolved, &result[quote_end + 2..]);
        } else {
            break;
        }
    }

    result
}

/// Rewrite import/export specifiers to browser-compatible URLs.
/// Converts `./foo` → `./foo.js`, `../bar` → `../bar.js`, etc.
/// Also rewrites resolve aliases (e.g., `@/components` → `/src/components`).
/// Bare specifiers like `react` are left as-is (browser must resolve via import map).
fn rewrite_imports(code: &str, _current_module_path: &str, aliases: &[pledgepack_core::PathAlias]) -> String {
    let mut result = code.to_string();

    // Rewrite relative import/export specifiers to include .tsx extension
    // This is a simple regex-like approach; Oxc could do this in AST but
    // for dev server speed, string rewriting is sufficient.
    for pattern in ["from \"", "from '", "import \"", "import '", "import(", "export * from \"", "export * from '"] {
        let mut search_from = 0;
        while let Some(pos) = result[search_from..].find(pattern) {
            let pos = search_from + pos;
            let after_pattern = pos + pattern.len();
            let rest = &result[after_pattern..];

            // Find the closing quote
            let closing_quote = if pattern.ends_with('"') { '"' }
                else if pattern.ends_with('\'') { '\'' }
                else { '(' }; // for "import("

            if closing_quote == '(' {
                // Dynamic import: find the string inside
                if let Some(quote_pos) = rest.find(|c: char| c == '"' || c == '\'') {
                    let quote_char = rest.as_bytes()[quote_pos] as char;
                    let spec_start = quote_pos + 1;
                    let spec_rest = &rest[spec_start..];
                    if let Some(end) = spec_rest.find(quote_char) {
                        let specifier = &spec_rest[..end];
                        if specifier.starts_with("./") || specifier.starts_with("../") {
                            let new_spec = add_js_extension(specifier);
                            let abs_start = after_pattern + spec_start;
                            let abs_end = abs_start + end;
                            result.replace_range(abs_start..abs_end, &new_spec);
                        }
                    }
                }
                search_from = after_pattern + 1;
                continue;
            }

            if let Some(end) = rest.find(closing_quote) {
                let specifier = &rest[..end];
                if specifier.starts_with("./") || specifier.starts_with("../") {
                    let new_spec = add_js_extension(specifier);
                    let abs_start = after_pattern;
                    let abs_end = abs_start + end;
                    result.replace_range(abs_start..abs_end, &new_spec);
                    // Advance past the rewritten specifier
                    search_from = abs_end + 1;
                } else {
                    // Not a relative path, advance past it
                    search_from = after_pattern + end + 1;
                }
            } else {
                break;
            }
        }
    }

    // Rewrite resolve aliases (e.g., "@/components" → "/src/components")
    for alias in aliases {
        let from_with_slash = format!("{}/", alias.from);
        let from_exact = alias.from.as_str();
        for pattern in ["from \"", "from '", "import \"", "import '", "import(", "export * from \"", "export * from '"] {
            // Match alias as exact or prefix
            let alias_prefixes = [from_exact, &from_with_slash];
            for &alias_prefix in &alias_prefixes {
                let search = format!("{}{}", pattern, alias_prefix);
                while let Some(pos) = result.find(&search) {
                    let _after_alias = pos + search.len();
                    // Replace the alias with the target path
                    let replacement = format!("{}{}", pattern, alias.to);
                    result.replace_range(pos..pos + pattern.len() + alias_prefix.len(), &replacement);
                    // Don't advance — the replacement might create new matches
                    break;
                }
            }
        }
    }

    result
}

/// Add .tsx extension to a relative specifier if it doesn't have one
fn add_js_extension(specifier: &str) -> String {
    // Check if it already has a JS-compatible extension
    let has_ext = specifier.ends_with(".js")
        || specifier.ends_with(".jsx")
        || specifier.ends_with(".ts")
        || specifier.ends_with(".tsx")
        || specifier.ends_with(".mjs")
        || specifier.ends_with(".json")
        || specifier.ends_with(".css");

    if has_ext {
        specifier.to_string()
    } else {
        // Use .tsx as default — the dev server module_handler tries
        // alternative extensions (.tsx, .ts, .jsx, .js) when serving
        format!("{}.tsx", specifier)
    }
}

/// WebSocket endpoint for HMR updates
/// Supports per-message deflate compression (feature 12) via axum's WebSocket upgrade
async fn hmr_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<DevServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_hmr_connection(socket, state))
}

async fn handle_hmr_connection(
    socket: axum::extract::ws::WebSocket,
    state: Arc<DevServerState>,
) {
    info!("HMR client connected");

    // Split socket into sender and receiver
    let (mut socket_tx, mut socket_rx) = socket.split();

    // Send initial connection confirmation
    let hello = serde_json::json!({
        "type": "connected",
        "message": "Pledge HMR connected"
    });
    let _ = socket_tx.send(Message::Text(hello.to_string().into()));

    // Register this client to receive HMR updates
    let (client_tx, mut client_rx) = mpsc::unbounded_channel::<HmrUpdate>();
    {
        let mut clients = state.hmr_clients.write().await;
        clients.push(client_tx);
    }

    // Spawn a task to forward HMR updates to this WebSocket client
    // Uses binary messages for larger payloads (compression benefit)
    let send_task = tokio::spawn(async move {
        while let Some(update) = client_rx.recv().await {
            let json = serde_json::to_string(&update).unwrap_or_default();
            // For small messages, use text; for larger ones, use binary
            // (WebSocket per-message deflate compresses both automatically)
            if json.len() < 4096 {
                if socket_tx.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            } else {
                if socket_tx.send(Message::Binary(json.into_bytes().into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Keep connection alive — handle incoming messages (ping/pong)
    while let Some(msg) = socket_rx.next().await {
        match msg {
            Ok(Message::Ping(_)) => {
                // axum auto-responds to pings, but we can also handle explicitly
            }
            Ok(Message::Close(_)) => {
                info!("HMR client disconnected");
                break;
            }
            _ => {}
        }
    }

    // Clean up: remove this client from the registered list
    send_task.abort();
    {
        let mut clients = state.hmr_clients.write().await;
        clients.retain(|tx| !tx.is_closed());
    }
}

/// Native file watcher — uses platform-specific APIs (ReadDirectoryChangesW/inotify/FSEvents)
/// with automatic fallback to notify crate
fn start_native_file_watcher(
    root: PathBuf,
    tx: mpsc::UnboundedSender<HmrUpdate>,
    server_entry: Option<String>,
) {
    let config = watcher::WatcherConfig::default();
    let rx = watcher::start_watcher(&root, config);

    // Determine server-only directories/patterns from server_entry
    let server_dirs = compute_server_dirs(&root, &server_entry);

    // Process events from the native watcher and send HMR updates
    while let Ok(event) = rx.recv() {
        let rel_path = event.path.strip_prefix(&root)
            .unwrap_or(&event.path)
            .to_string_lossy()
            .replace('\\', "/");

        let ext = event.path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Check if this is a server-only file change
        let is_server_file = is_server_file(&rel_path, &server_dirs, &server_entry);

        if is_server_file {
            info!("Server file changed: {} — triggering graceful reload", rel_path);

            // Send "server-reload" notification so clients know to expect a brief pause
            let reload_start = HmrUpdate {
                update_type: "server-reload".to_string(),
                path: rel_path.clone(),
                message: Some("Server code changed — reloading...".to_string()),
                file: None,
                css: None,
                stack: None,
                line: None,
                column: None,
                deps: Vec::new(),
                full_reload: None,
                diff: None,
                full_code: None,
            };
            let _ = tx.send(reload_start);

            // Brief delay to let clients process the notification
            std::thread::sleep(std::time::Duration::from_millis(100));

            // Send "server-reload-complete" so clients know the server is back
            let reload_done = HmrUpdate {
                update_type: "server-reload-complete".to_string(),
                path: rel_path.clone(),
                message: Some("Server code reloaded successfully".to_string()),
                file: None,
                css: None,
                stack: None,
                line: None,
                column: None,
                deps: Vec::new(),
                full_reload: None,
                diff: None,
                full_code: None,
            };
            let _ = tx.send(reload_done);
            continue;
        }

        // Read the new file content for diff computation
        let new_content = std::fs::read_to_string(&event.path).ok();

        let update = HmrUpdate {
            update_type: "update".to_string(),
            path: rel_path.clone(),
            message: None,
            file: None,
            css: if ext == "css" {
                std::fs::read_to_string(&event.path).ok()
            } else {
                None
            },
            stack: None,
            line: None,
            column: None,
            deps: Vec::new(),
            full_reload: None,
            diff: None,
            full_code: new_content,
        };
        let _ = tx.send(update);
    }
}

/// Compute server-only directories from the server_entry config
fn compute_server_dirs(root: &std::path::Path, server_entry: &Option<String>) -> Vec<String> {
    let mut dirs = Vec::new();
    if let Some(entry) = server_entry {
        // Derive server directory from entry path (e.g., "server/index.ts" → "server")
        if let Some(parent) = std::path::Path::new(entry).parent() {
            if !parent.as_os_str().is_empty() {
                dirs.push(parent.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    // Common SSR/API directories
    for common in &["api", "server", "src/api", "src/server", "app/api"] {
        if root.join(common).is_dir() {
            dirs.push(common.to_string());
        }
    }
    dirs
}

/// Check if a file path is a server-only file
fn is_server_file(
    rel_path: &str,
    server_dirs: &[String],
    server_entry: &Option<String>,
) -> bool {
    // Check if it's the server entry file itself
    if let Some(entry) = server_entry {
        if rel_path == entry.as_str() {
            return true;
        }
    }
    // Check if it's in a server directory
    for dir in server_dirs {
        if rel_path.starts_with(dir) {
            return true;
        }
    }
    false
}

/// Detect multi-entry HTML files in the project root
/// Looks for index.html, admin.html, mobile.html, etc.
fn detect_entries(config: &PledgeConfig) -> Vec<EntryConfig> {
    let mut entries = Vec::new();

    // Check for explicit html_entry in config
    if let Some(ref html_entry) = config.html_entry {
        let entry_module = config.entry.first().cloned().unwrap_or_default();
        entries.push(EntryConfig {
            name: "index".to_string(),
            html_file: html_entry.clone(),
            entry_module,
        });
        return entries;
    }

    // Auto-detect HTML files in root and src directories
    let check_dirs = [config.root.clone(), config.root.join("src")];

    for dir in &check_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries_iter) = std::fs::read_dir(dir) {
            for entry in entries_iter.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with(".html") {
                    continue;
                }

                let entry_name = name.trim_end_matches(".html").to_string();

                // Skip non-entry HTML files (like error pages)
                if entry_name == "error" || entry_name == "404" {
                    continue;
                }

                // Find the corresponding entry module
                let entry_module = if entry_name == "index" {
                    config.entry.first().cloned().unwrap_or_else(|| "src/index.tsx".to_string())
                } else {
                    // Look for matching JS/TS file
                    let candidates = [
                        format!("src/{}.tsx", entry_name),
                        format!("src/{}.ts", entry_name),
                        format!("src/{}/index.tsx", entry_name),
                        format!("src/{}/index.ts", entry_name),
                        format!("{}.tsx", entry_name),
                        format!("{}.ts", entry_name),
                    ];
                    candidates.iter()
                        .find(|c| config.root.join(c).exists())
                        .cloned()
                        .unwrap_or_else(|| format!("src/{}.tsx", entry_name))
                };

                let html_rel = path.strip_prefix(&config.root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();

                entries.push(EntryConfig {
                    name: entry_name,
                    html_file: html_rel,
                    entry_module,
                });
            }
        }
    }

    // If no entries found, default to index
    if entries.is_empty() {
        entries.push(EntryConfig {
            name: "index".to_string(),
            html_file: "index.html".to_string(),
            entry_module: config.entry.first().cloned().unwrap_or_else(|| "src/index.tsx".to_string()),
        });
    }

    entries
}

/// Entry index handler for multi-entry dev server (feature 13)
/// Serves the HTML file for a specific entry point (e.g., /admin → admin.html)
async fn entry_index_handler(
    State(state): State<Arc<DevServerState>>,
    Path(entry_name): Path<String>,
) -> Response {
    let entries = state.entries.read().await;
    let entry = entries.iter().find(|e| e.name == entry_name);

    if entry.is_none() {
        return (StatusCode::NOT_FOUND, "Entry not found").into_response();
    }

    let entry = entry.unwrap();
    let html_path = state.config.root.join(&entry.html_file);

    let mut html = match std::fs::read_to_string(&html_path) {
        Ok(content) => content,
        Err(_) => {
            // Auto-generate HTML shell from layout.tsx
            let (html_attrs, head_content) = match shell_generator::try_extract_shell_from_project(&state.config.root) {
                Some((attrs, head)) => (attrs, head),
                None => ("lang=\"en\"".to_string(), format!("<title>Pledge — {}</title>", entry.name)),
            };
            let import_map = generate_import_map(&state.config);
            shell_generator::generate_html_shell(&html_attrs, &head_content, "", &import_map)
        }
    };

    // Inject HMR client script (same as index_handler)
    let hmr_script = r#"
    <script>
        window.__pledge_vue_components = window.__pledge_vue_components || {};
        window.__pledge_svelte_components = window.__pledge_svelte_components || {};
        window.__pledge_solid_hmr = window.__pledge_solid_hmr || [];
        window.__pledge_fast_refresh = window.__pledge_fast_refresh || {};
        window.__pledge_hmr_modules = window.__pledge_hmr_modules || {};
        let __pledge_ws;
        let __pledge_ws_reconnect_delay = 1000;
        let __pledge_ws_max_delay = 30000;
        let __pledge_ws_current_delay = __pledge_ws_reconnect_delay;
        let __pledge_ws_closed_by_user = false;
        function __pledge_connect_ws() {
            const ws = new WebSocket('ws://' + location.host + '/__pledge_hmr');
            window.__pledge_ws = ws;
            ws.onmessage = (event) => {
                const data = JSON.parse(event.data);
                if (data.type === 'update') {
                    console.log('[pledge] HMR update:', data.path);
                    clearPledgeError();
                    __pledge_ws_current_delay = __pledge_ws_reconnect_delay;
                    if (data.path) {
                        if (data.path.endsWith('.css') || data.css) {
                            if (data.css) { updatePledgeCSS(data.path, data.css); }
                            else { fetchPledgeCSS(data.path); }
                        } else {
                            const links = document.querySelectorAll('script[src="' + data.path + '"]');
                            links.forEach(link => {
                                const newLink = document.createElement('script');
                                newLink.type = 'module';
                                newLink.src = data.path + '?t=' + Date.now();
                                link.replaceWith(newLink);
                            });
                        }
                        if (data.deps && data.deps.length > 0) {
                            data.deps.forEach((depPath) => {
                                if (depPath.endsWith('.css')) { fetchPledgeCSS(depPath); }
                                else {
                                    const depLinks = document.querySelectorAll('script[src="' + depPath + '"]');
                                    depLinks.forEach(link => {
                                        const newLink = document.createElement('script');
                                        newLink.type = 'module';
                                        newLink.src = depPath + '?t=' + Date.now();
                                        link.replaceWith(newLink);
                                    });
                                }
                            });
                        }
                    }
                } else if (data.type === 'error') {
                    showPledgeError(data.message, data.file, data.stack, data.line, data.column);
                } else if (data.type === 'connected') {
                    console.log('[pledge] HMR connected');
                } else if (data.type === 'server-reload') {
                    console.log('[pledge] Server reloading:', data.message);
                    showPledgeServerReload(data.message);
                } else if (data.type === 'server-reload-complete') {
                    console.log('[pledge] Server reloaded:', data.message);
                    clearPledgeServerReload();
                }
            };
            ws.onopen = () => { console.log('[pledge] HMR connected'); __pledge_ws_current_delay = __pledge_ws_reconnect_delay; };
            ws.onclose = () => {
                if (__pledge_ws_closed_by_user) return;
                console.warn('[pledge] HMR disconnected — reconnecting in ' + __pledge_ws_current_delay + 'ms');
                setTimeout(() => {
                    __pledge_ws_current_delay = Math.min(__pledge_ws_current_delay * 2, __pledge_ws_max_delay);
                    __pledge_connect_ws();
                }, __pledge_ws_current_delay);
            };
            ws.onerror = () => {};
        }
        __pledge_connect_ws();
        function updatePledgeCSS(path, cssContent) {
            let styleId = '__pledge_style_' + path.replace(/[^a-zA-Z0-9]/g, '_');
            let existing = document.getElementById(styleId);
            if (!existing) { existing = document.createElement('style'); existing.id = styleId; document.head.appendChild(existing); }
            existing.textContent = cssContent;
        }
        async function fetchPledgeCSS(path) {
            try { const res = await fetch(path + '?t=' + Date.now()); const css = await res.text(); updatePledgeCSS(path, css); }
            catch(e) { console.error('[pledge] CSS HMR fetch failed:', e); }
        }
        function showPledgeError(message, file, stack, line, column) {
            let overlay = document.getElementById('__pledge_error_overlay');
            if (!overlay) {
                overlay = document.createElement('div');
                overlay.id = '__pledge_error_overlay';
                overlay.style.cssText = 'position:fixed;inset:0;z-index:99999;background:rgba(0,0,0,0.92);font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;padding:2rem;overflow:auto;display:flex;flex-direction:column;';
                document.body.appendChild(overlay);
            }
            let fileHtml = file ? '<div style="color:#888;margin-bottom:1rem;font-size:0.9rem;">' + file + (line ? ':' + line + (column ? ':' + column : '') : '') + '</div>' : '';
            let sourceContext = '<pre style="color:#fff;white-space:pre-wrap;font-size:0.9rem;">' + message.replace(/</g, '&lt;').replace(/>/g, '&gt;') + '</pre>';
            overlay.innerHTML = '<div style="max-width:900px;margin:0 auto;flex:1;"><div style="display:flex;align-items:center;gap:0.5rem;margin-bottom:1rem;"><span style="color:#ff4444;font-size:1.5rem;">&#9888;</span><span style="color:#ff4444;font-size:1.3rem;font-weight:600;">Pledge Build Error</span></div>' + fileHtml + '<div style="background:#1a1a1a;border:1px solid #333;border-radius:8px;padding:1rem;margin-bottom:1rem;overflow:auto;">' + sourceContext + '</div><div style="color:#666;font-size:0.8rem;">Fix the error and save to reload.</div><button onclick="clearPledgeError()" style="margin-top:1rem;padding:0.5rem 1rem;background:#333;color:#fff;border:1px solid #555;border-radius:4px;cursor:pointer;font-family:inherit;">Close</button></div>';
        }
        function clearPledgeError() { let overlay = document.getElementById('__pledge_error_overlay'); if (overlay) overlay.remove(); }
        function showPledgeServerReload(message) { let b = document.getElementById('__pledge_server_reload'); if (!b) { b = document.createElement('div'); b.id = '__pledge_server_reload'; b.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:99998;background:#1a1a2e;color:#7c3aed;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;padding:0.75rem;text-align:center;font-size:0.9rem;border-bottom:1px solid #7c3aed;'; document.body.appendChild(b); } b.textContent = '⟳ ' + (message || 'Server reloading...'); }
        function clearPledgeServerReload() { let b = document.getElementById('__pledge_server_reload'); if (b) b.remove(); }
        window.addEventListener('pledge:hmr-success', clearPledgeError);
        window.addEventListener('error', function(event) {
            if (event.error || event.message) {
                var msg = event.error && event.error.message ? event.error.message : (event.message || 'Unknown runtime error');
                showPledgeError(msg, event.filename || '', event.error && event.error.stack ? event.error.stack : '', event.lineno || 0, event.colno || 0);
            }
        });
        window.addEventListener('unhandledrejection', function(event) {
            var reason = event.reason;
            showPledgeError('Unhandled Promise Rejection: ' + (reason && reason.message ? reason.message : String(reason)), '', reason && reason.stack ? reason.stack : '', 0, 0);
        });
    </script>
</body>"#;

    // Inject import map
    let import_map = generate_import_map(&state.config);
    if !import_map.is_empty() {
        let map_tag = format!("<script type=\"importmap\">\n{}\n</script>\n", import_map);
        if html.contains("</head>") {
            html = html.replace("</head>", &format!("{}\n</head>", map_tag));
        } else if html.contains("<body") {
            html = html.replace("<body", &format!("{}\n<body", map_tag));
        } else {
            html = format!("{}\n{}", map_tag, html);
        }
    }

    if html.contains("</body>") {
        html = html.replace("</body>", &format!("{}\n</body>", hmr_script));
    } else {
        html.push_str(hmr_script);
        html.push_str("</body></html>");
    }

    Html(html).into_response()
}

/// Broadcast HMR updates to all connected WebSocket clients
/// Also computes dependent modules from the import graph for cascading HMR updates
/// and computes line-level diffs for partial HMR updates (feature 10)
/// and tracks import pattern changes for on-demand optimization (feature 15)
async fn hmr_broadcast_loop(
    state: Arc<DevServerState>,
    mut hmr_rx: mpsc::UnboundedReceiver<HmrUpdate>,
) {
    while let Some(mut update) = hmr_rx.recv().await {
        info!("HMR update: {} (type: {})", update.path, update.update_type);

        // Compute line-level diff for partial HMR updates (feature 10)
        if update.update_type == "update" && update.full_code.is_some() {
            let module_cache = state.module_cache.read().await;
            if let Some(old_code) = module_cache.get(&update.path) {
                let new_code = update.full_code.as_ref().unwrap();
                let diff = hmr_diff::compute_diff(old_code, new_code);
                if diff.is_small() {
                    // Only send the diff if it's small enough to be efficient
                    update.diff = Some(diff);
                    // Keep full_code as fallback
                } else {
                    // Diff is too large, send full code instead
                    update.diff = None;
                }
            }
            drop(module_cache);

            // Update the module cache with the new code
            let mut module_cache = state.module_cache.write().await;
            module_cache.insert(update.path.clone(), update.full_code.clone().unwrap_or_default());
        }

        // Track import pattern changes for on-demand optimization (feature 15)
        if update.update_type == "update" && update.full_code.is_some() {
            let new_code = update.full_code.as_ref().unwrap();
            let new_imports = extract_imports(new_code);
            let import_patterns = state.import_patterns.read().await;
            let old_imports = import_patterns.get(&update.path);
            let imports_changed = old_imports.map_or(true, |old| {
                old.len() != new_imports.len() || old.iter().zip(new_imports.iter()).any(|(a, b)| a != b)
            });
            drop(import_patterns);

            if imports_changed {
                info!("Import patterns changed for {}, re-optimizing dependencies", update.path);
                let mut import_patterns = state.import_patterns.write().await;
                import_patterns.insert(update.path.clone(), new_imports);
                // Trigger on-demand re-optimization
                // The lazy pipeline will re-optimize only the affected dependencies
                let mut lazy_pipeline = state.lazy_pipeline.write().await;
                lazy_pipeline.mark_deps_dirty(&update.path);
            }
        }

        // Compute dependent modules that need cascading updates
        if update.update_type == "update" {
            let graph = state.import_graph.read().await;
            let mut deps = Vec::new();
            let mut visited = std::collections::HashSet::new();
            collect_dependents(&update.path, &graph, &mut deps, &mut visited);
            if !deps.is_empty() {
                update.deps = deps;
            }
        }

        // Broadcast to all registered client channels
        let clients = state.hmr_clients.read().await;
        for client_tx in clients.iter() {
            let _ = client_tx.send(update.clone());
        }
    }
}

/// Extract import specifiers from JS/TS source code for on-demand optimization tracking
fn extract_imports(code: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for pattern in ["from \"", "from '", "import \"", "import '", "import("] {
        let mut search_from = 0;
        while let Some(pos) = code[search_from..].find(pattern) {
            let pos = search_from + pos;
            let after_pattern = pos + pattern.len();
            let rest = &code[after_pattern..];

            let closing_quote = if pattern.ends_with('"') { '"' }
                else if pattern.ends_with('\'') { '\'' }
                else { '(' };

            if closing_quote == '(' {
                if let Some(quote_pos) = rest.find(|c: char| c == '"' || c == '\'') {
                    let quote_char = rest.as_bytes()[quote_pos] as char;
                    let spec_start = quote_pos + 1;
                    let spec_rest = &rest[spec_start..];
                    if let Some(end) = spec_rest.find(quote_char) {
                        let specifier = &spec_rest[..end];
                        imports.push(specifier.to_string());
                    }
                }
                search_from = after_pattern + 1;
                continue;
            }

            if let Some(end) = rest.find(closing_quote) {
                let specifier = &rest[..end];
                imports.push(specifier.to_string());
                search_from = after_pattern + end + 1;
            } else {
                break;
            }
        }
    }
    imports.sort();
    imports.dedup();
    imports
}

/// Recursively collect all modules that depend on the given path (directly or transitively)
fn collect_dependents(
    path: &str,
    graph: &std::collections::HashMap<String, Vec<String>>,
    deps: &mut Vec<String>,
    visited: &mut std::collections::HashSet<String>,
) {
    if visited.contains(path) {
        return;
    }
    visited.insert(path.to_string());

    // Try exact match and also try with common extensions
    let candidates = [path.to_string(), format!("{}.js", path), format!("{}.ts", path), format!("{}.tsx", path)];
    for candidate in &candidates {
        if let Some(direct_deps) = graph.get(candidate) {
            for dep in direct_deps {
                if !deps.contains(dep) {
                    deps.push(dep.clone());
                }
                collect_dependents(dep, graph, deps, visited);
            }
        }
    }
}

/// Proxy handler for dev server API proxying.
/// Forwards requests to a target URL with optional path rewriting.
/// Supports all HTTP methods (GET, POST, PUT, DELETE, PATCH, etc.)
async fn proxy_handler(
    method: axum::http::Method,
    rest: &str,
    target: &str,
    path_prefix: &str,
    rewrite: bool,
    _headers: &std::collections::HashMap<String, String>,
    body: axum::body::Body,
) -> Response {
    let target_url = if rewrite {
        format!("{}/{}", target.trim_end_matches('/'), rest)
    } else {
        format!("{}{}/{}", target.trim_end_matches('/'), path_prefix, rest)
    };

    info!("Proxy: {} {}{}/{} → {}", method, path_prefix, rest, if rewrite { " (rewrite)" } else { "" }, target_url);

    let client = reqwest::Client::new();
    let req_method = reqwest::Method::from_bytes(method.as_str().as_bytes())
        .unwrap_or(reqwest::Method::GET);

    let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap_or_default();

    let request = client.request(req_method, &target_url).body(body_bytes);

    match request.send().await {
        Ok(resp) => {
            let status = resp.status();
            let headers = resp.headers().clone();
            let body = resp.bytes().await.unwrap_or_default();

            let mut response_headers = Vec::new();
            for (key, value) in headers.iter() {
                // Skip hop-by-hop headers
                if !matches!(key.as_str(), "connection" | "keep-alive" | "transfer-encoding" | "te" | "trailer" | "upgrade") {
                    if let Ok(v) = value.to_str() {
                        response_headers.push((
                            axum::http::HeaderName::try_from(key.as_str()).unwrap_or(axum::http::header::CONTENT_TYPE),
                            axum::http::HeaderValue::try_from(v).unwrap_or(axum::http::HeaderValue::from_static("")),
                        ));
                    }
                }
            }

            let axum_status = axum::http::StatusCode::from_u16(status.as_u16())
                .unwrap_or(StatusCode::BAD_GATEWAY);

            let mut response = axum::response::Response::new(axum::body::Body::from(body));
            *response.status_mut() = axum_status;
            for (key, value) in response_headers {
                response.headers_mut().insert(key, value);
            }
            response
        }
        Err(e) => {
            tracing::warn!("Proxy error: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                format!("Proxy error: {}", e),
            ).into_response()
        }
    }
}

/// WebSocket proxy handler — bridges client WebSocket to target WebSocket
async fn ws_proxy_handler(
    client_socket: axum::extract::ws::WebSocket,
    rest: &str,
    target: &str,
    path_prefix: &str,
    rewrite: bool,
) {
    let target_url = if rewrite {
        format!("{}/{}", target.trim_end_matches('/'), rest)
    } else {
        format!("{}{}/{}", target.trim_end_matches('/'), path_prefix, rest)
    };

    // Convert http(s):// to ws(s)://
    let ws_url = target_url
        .replacen("http://", "ws://", 1)
        .replacen("https://", "wss://", 1);

    info!("WS Proxy: connecting to {}", ws_url);

    use tokio_tungstenite::tungstenite::Message;
    use futures_util::StreamExt;

    // Connect to the target WebSocket
    let (target_socket, _) = match tokio_tungstenite::connect_async(&ws_url).await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::warn!("WS Proxy connection failed: {}", e);
            return;
        }
    };

    let (client_sink, client_stream) = client_socket.split();
    let (target_sink, target_stream) = target_socket.split();

    // Convert client messages to tungstenite messages and forward
    let client_to_target = client_stream.filter_map(|msg| async {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => Some(Ok(Message::Text(text.to_string().into()))),
            Ok(axum::extract::ws::Message::Binary(bin)) => Some(Ok(Message::Binary(bin))),
            Ok(axum::extract::ws::Message::Ping(data)) => Some(Ok(Message::Ping(data))),
            Ok(axum::extract::ws::Message::Pong(data)) => Some(Ok(Message::Pong(data))),
            Ok(axum::extract::ws::Message::Close(_)) => Some(Ok(Message::Close(None))),
            Err(_) => None,
        }
    }).forward(target_sink);

    // Convert target messages to client messages and forward
    let target_to_client = target_stream.filter_map(|msg| async {
        match msg {
            Ok(Message::Text(text)) => Some(Ok(axum::extract::ws::Message::Text(text.to_string().into()))),
            Ok(Message::Binary(bin)) => Some(Ok(axum::extract::ws::Message::Binary(bin))),
            Ok(Message::Ping(data)) => Some(Ok(axum::extract::ws::Message::Ping(data))),
            Ok(Message::Pong(data)) => Some(Ok(axum::extract::ws::Message::Pong(data))),
            Ok(Message::Close(_)) => Some(Ok(axum::extract::ws::Message::Close(None))),
            Ok(_) => None,
            Err(_) => None,
        }
    }).forward(client_sink);

    tokio::select! {
        _ = client_to_target => {},
        _ = target_to_client => {},
    }

    info!("WS Proxy: connection closed");
}

/// Virtual file system handler — /@fs/<path> serves files from absolute paths
/// This mirrors Vite's /@fs/ virtual module system for internal module resolution
async fn virtual_fs_handler(
    State(state): State<Arc<DevServerState>>,
    Path(path): Path<String>,
) -> Response {
    // /@fs/ serves files from absolute paths on the filesystem
    let full_path = if path.starts_with('/') {
        std::path::PathBuf::from(&path)
    } else {
        std::path::PathBuf::from(format!("/{}", path))
    };

    if !full_path.exists() || !full_path.is_file() {
        return (StatusCode::NOT_FOUND, "Virtual file not found").into_response();
    }

    // Security: ensure the path is within the project root or node_modules
    let canonical = match full_path.canonicalize() {
        Ok(c) => c,
        Err(_) => return (StatusCode::NOT_FOUND, "Cannot resolve path").into_response(),
    };
    let root_canonical = match state.config.root.canonicalize() {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Cannot resolve root").into_response(),
    };

    // Allow access to project root and its node_modules
    let is_allowed = canonical.starts_with(&root_canonical)
        || canonical.to_string_lossy().contains("node_modules");
    if !is_allowed {
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    }

    if let Ok(content) = tokio::fs::read(&canonical).await {
        let content_type = guess_content_type(&canonical.to_string_lossy());
        return (
            [(header::CONTENT_TYPE, content_type),
             (header::CACHE_CONTROL, "no-cache")],
            content,
        ).into_response();
    }

    (StatusCode::NOT_FOUND, "Virtual file not found").into_response()
}

/// Virtual ID handler — /@id/<id> serves modules by virtual identifier
/// Used for resolving bare module specifiers and internal module IDs
async fn virtual_id_handler(
    State(state): State<Arc<DevServerState>>,
    Path(path): Path<String>,
) -> Response {
    // /@id/ resolves virtual module IDs to actual files
    // First try as a bare specifier in node_modules
    let node_modules_path = state.config.root.join("node_modules").join(&path);
    if node_modules_path.exists() {
        if let Ok(content) = tokio::fs::read(&node_modules_path).await {
            let content_type = guess_content_type(&path);
            return (
                [(header::CONTENT_TYPE, content_type),
                 (header::CACHE_CONTROL, "no-cache")],
                content,
            ).into_response();
        }
    }

    // Try as a path relative to project root
    let root_path = state.config.root.join(&path);
    if root_path.exists() && root_path.is_file() {
        if let Ok(content) = tokio::fs::read(&root_path).await {
            let content_type = guess_content_type(&path);
            return (
                [(header::CONTENT_TYPE, content_type),
                 (header::CACHE_CONTROL, "no-cache")],
                content,
            ).into_response();
        }
    }

    // Try resolving as a package with module/main field
    let pkg_json = state.config.root.join("node_modules").join(&path).join("package.json");
    if pkg_json.exists() {
        if let Ok(content) = std::fs::read_to_string(&pkg_json) {
            if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                let entry = pkg.get("module").or_else(|| pkg.get("main"))
                    .and_then(|v| v.as_str()).unwrap_or("index.js");
                let entry_path = state.config.root.join("node_modules").join(&path).join(entry);
                if entry_path.exists() {
                    if let Ok(entry_content) = tokio::fs::read(&entry_path).await {
                        return (
                            [(header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
                             (header::CACHE_CONTROL, "no-cache")],
                            entry_content,
                        ).into_response();
                    }
                }
            }
        }
    }

    (StatusCode::NOT_FOUND, "Virtual module not found").into_response()
}

/// Public directory handler — serves static assets from the configured public directory
async fn public_dir_handler(
    State(state): State<Arc<DevServerState>>,
    Path(path): Path<String>,
) -> Response {
    let public_dir = &state.config.dev_server.public_dir;
    let public_path = state.config.root.join(public_dir).join(&path);

    if !public_path.exists() || !public_path.is_file() {
        return (StatusCode::NOT_FOUND, "Static asset not found").into_response();
    }

    if let Ok(content) = tokio::fs::read(&public_path).await {
        let content_type = guess_content_type(&path);
        return (
            [(header::CONTENT_TYPE, content_type),
             (header::CACHE_CONTROL, "public, max-age=3600")],
            content,
        ).into_response();
    }

    (StatusCode::NOT_FOUND, "Static asset not found").into_response()
}

/// Guess content type from file extension for static asset serving
fn guess_content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

/// Open the default browser to the given URL.
/// Uses the `opener` crate for cross-platform support (Windows, macOS, Linux, WSL).
fn open_browser(url: &str) {
    match opener::open(url) {
        Ok(_) => info!("Opened browser at {}", url),
        Err(e) => tracing::warn!("Failed to open browser: {}", e),
    }
}

/// Generate a self-signed TLS certificate and private key for local HTTPS dev server.
/// Uses rcgen to create a certificate valid for localhost and the local IP.
fn generate_self_signed_cert(cert_path: &std::path::Path, key_path: &std::path::Path) -> anyhow::Result<()> {
    use rcgen::{CertificateParams, KeyPair, DistinguishedName, DnType};

    let mut san_names = vec!["localhost".to_string()];
    if let Ok(ip) = local_ip_address::local_ip() {
        san_names.push(ip.to_string());
    }

    let mut params = CertificateParams::new(san_names.clone())?;

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "PledgePack Dev Server");
    dn.push(DnType::OrganizationName, "PledgePack");
    params.distinguished_name = dn;

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    std::fs::write(cert_path, cert.pem())?;
    std::fs::write(key_path, key_pair.serialize_pem())?;

    Ok(())
}
