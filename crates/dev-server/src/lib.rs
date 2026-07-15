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
use pledgepack_core::{BuildEngine, PledgeConfig};
use pledgepack_core::module::ModuleKind;
use pledgepack_core::transform as pledge_transform;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::info;

pub struct DevServerState {
    pub engine: RwLock<BuildEngine>,
    pub config: Arc<PledgeConfig>,
    pub hmr_tx: mpsc::UnboundedSender<HmrUpdate>,
    pub hmr_clients: RwLock<Vec<mpsc::UnboundedSender<HmrUpdate>>>,
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
}

pub async fn serve(engine: BuildEngine, config: &PledgeConfig) -> Result<()> {
    let port = config.dev_server.port;
    let host = config.dev_server.host.clone();

    let (hmr_tx, hmr_rx) = mpsc::unbounded_channel::<HmrUpdate>();

    // Start file watcher if HMR is enabled
    if config.dev_server.hmr {
        let watch_root = config.root.clone();
        let tx = hmr_tx.clone();
        tokio::spawn(async move {
            start_file_watcher(watch_root, tx);
        });
    }

    let state = Arc::new(DevServerState {
        engine: RwLock::new(engine),
        config: config.clone().into(),
        hmr_tx,
        hmr_clients: RwLock::new(Vec::new()),
    });

    // Spawn HMR broadcast task
    let hmr_state = state.clone();
    tokio::spawn(async move {
        hmr_broadcast_loop(hmr_state, hmr_rx).await;
    });

    let mut app = Router::new()
        .route("/", get(index_handler))
        .route("/__pledge_hmr", get(hmr_websocket_handler))
        .route("/__pledge_error", get(error_overlay_handler))
        .route("/{*path}", get(module_handler))
        .with_state(state.clone());

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
                        ws.on_upgrade(move |socket| ws_proxy_handler(socket, &rest, &target, &path_prefix, rewrite))
                    }
                }),
            );
            app = app.merge(ws_router);
        }
    }

    let addr = format!("{}:{}", host, port);

    // HTTPS support
    if let Some(ref https_config) = config.https {
        info!("Dev server running at https://{}", addr);
        let cert_path = &https_config.cert;
        let key_path = &https_config.key;

        if !cert_path.exists() || !key_path.exists() {
            anyhow::bail!("HTTPS cert or key file not found: {:?}, {:?}", cert_path, key_path);
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
        
        // Serve with TLS
        loop {
            let (stream, _addr) = listener.accept().await?;
            let acceptor = tls_acceptor.clone();
            let app_clone = app.clone();
            tokio::spawn(async move {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        let _ = axum::serve(axum::serve::IncomingStream::new(tls_stream), app_clone).await;
                    }
                    Err(e) => {
                        tracing::warn!("TLS accept error: {}", e);
                    }
                }
            });
        }
    } else {
        info!("Dev server running at http://{}", addr);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}

/// Serve the index.html shell
async fn index_handler(State(state): State<Arc<DevServerState>>) -> impl IntoResponse {
    let entry = &state.config.entry[0];
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Pledge Dev</title>
    <style>* {{ margin: 0; padding: 0; box-sizing: border-box; }} body {{ background: #0a0a0a; }}</style>
</head>
<body>
    <div id="root"></div>
    <script>
        // Minimal React shim for classic JSX runtime (React.createElement)
        window.React = {{
            createElement: function(type, props) {{
                var children = [];
                for (var i = 2; i < arguments.length; i++) {{
                    var arg = arguments[i];
                    if (Array.isArray(arg)) {{
                        children = children.concat(arg);
                    }} else if (arg !== null && arg !== undefined && arg !== false) {{
                        children.push(arg);
                    }}
                }}
                var el = document.createElement(typeof type === 'string' ? type : 'div');
                if (props) {{
                    for (var key in props) {{
                        if (key === 'children') {{
                            if (typeof props.children === 'string' || typeof props.children === 'number') {{
                                children = [props.children];
                            }}
                        }} else if (key === 'className') {{
                            el.className = props[key];
                        }} else if (key === 'style' && typeof props[key] === 'object') {{
                            Object.assign(el.style, props[key]);
                        }} else if (key.startsWith('on') && typeof props[key] === 'function') {{
                            el.addEventListener(key.slice(2).toLowerCase(), props[key]);
                        }}
                    }}
                }}
                children.forEach(function(child) {{
                    if (typeof child === 'string' || typeof child === 'number') {{
                        el.appendChild(document.createTextNode(String(child)));
                    }} else if (child instanceof Node) {{
                        el.appendChild(child);
                    }}
                }});
                return el;
            }},
            Fragment: 'div'
        }};
    </script>
    <script type="module" src="/"></script>
    <script>
        const ws = new WebSocket('ws://' + location.host + '/__pledge_hmr');
        ws.onmessage = (event) => {{
            const data = JSON.parse(event.data);
            if (data.type === 'update') {{
                console.log('[pledge] HMR update:', data.path);
                clearPledgeError();
                if (data.path) {{
                    // CSS HMR: inject <style> tag without page reload
                    if (data.path.endsWith('.css') || data.css) {{
                        if (data.css) {{
                            updatePledgeCSS(data.path, data.css);
                        }} else {{
                            // Reload CSS by fetching the updated file
                            fetchPledgeCSS(data.path);
                        }}
                    }} else {{
                        // JS HMR: reload the changed script tag
                        const links = document.querySelectorAll('script[src="' + data.path + '"]');
                        links.forEach(link => {{
                            const newLink = document.createElement('script');
                            newLink.type = 'module';
                            newLink.src = data.path + '?t=' + Date.now();
                            link.replaceWith(newLink);
                        }});
                    }}
                }}
            }} else if (data.type === 'error') {{
                showPledgeError(data.message, data.file);
            }} else if (data.type === 'connected') {{
                console.log('[pledge] HMR connected');
            }}
        }};
        ws.onopen = () => console.log('[pledge] HMR connected');
        ws.onclose = () => {{
            console.warn('[pledge] HMR disconnected — reloading...');
            setTimeout(() => location.reload(), 1000);
        }};

        // CSS HMR: update or inject <style> tag
        function updatePledgeCSS(path, cssContent) {{
            let styleId = '__pledge_style_' + path.replace(/[^a-zA-Z0-9]/g, '_');
            let existing = document.getElementById(styleId);
            if (!existing) {{
                existing = document.createElement('style');
                existing.id = styleId;
                document.head.appendChild(existing);
            }}
            existing.textContent = cssContent;
            console.log('[pledge] CSS HMR:', path);
        }}

        // CSS HMR: fetch updated CSS and inject
        async function fetchPledgeCSS(path) {{
            try {{
                const res = await fetch(path + '?t=' + Date.now());
                const css = await res.text();
                updatePledgeCSS(path, css);
            }} catch(e) {{
                console.error('[pledge] CSS HMR fetch failed:', e);
            }}
        }}

        // Pledge Error Overlay — beautiful, interactive error display
        function showPledgeError(message, file) {{
            let overlay = document.getElementById('__pledge_error_overlay');
            if (!overlay) {{
                overlay = document.createElement('div');
                overlay.id = '__pledge_error_overlay';
                overlay.style.cssText = 'position:fixed;inset:0;z-index:99999;background:rgba(0,0,0,0.92);font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;padding:2rem;overflow:auto;display:flex;flex-direction:column;';
                document.body.appendChild(overlay);
            }}
            let fileHtml = file ? '<div style="color:#888;margin-bottom:1rem;font-size:0.9rem;">' + file + '</div>' : '';
            // Try to extract source context from error message
            let sourceContext = '';
            let lines = message.split('\n');
            if (lines.length > 1) {{
                sourceContext = lines.map((line, i) => {{
                    let lineClass = line.includes('error') || line.includes('Error') ? 'color:#ff4444;' : 'color:#e0e0e0;';
                    return '<div style="' + lineClass + 'white-space:pre;padding-left:2rem;">' + line.replace(/</g, '&lt;').replace(/>/g, '&gt;') + '</div>';
                }}).join('');
            }} else {{
                sourceContext = '<pre style="color:#fff;white-space:pre-wrap;font-size:0.9rem;">' + message.replace(/</g, '&lt;').replace(/>/g, '&gt;') + '</pre>';
            }}
            overlay.innerHTML =
                '<div style="max-width:900px;margin:0 auto;flex:1;">' +
                '<div style="display:flex;align-items:center;gap:0.5rem;margin-bottom:1rem;">' +
                '<span style="color:#ff4444;font-size:1.5rem;">&#9888;</span>' +
                '<span style="color:#ff4444;font-size:1.3rem;font-weight:600;">Pledge Build Error</span>' +
                '</div>' +
                fileHtml +
                '<div style="background:#1a1a1a;border:1px solid #333;border-radius:8px;padding:1rem;margin-bottom:1rem;overflow:auto;">' + sourceContext + '</div>' +
                '<div style="color:#666;font-size:0.8rem;">Fix the error and save to reload. The overlay will disappear automatically.</div>' +
                '<button onclick="clearPledgeError()" style="margin-top:1rem;padding:0.5rem 1rem;background:#333;color:#fff;border:1px solid #555;border-radius:4px;cursor:pointer;font-family:inherit;">Close</button>' +
                '</div>';
        }}
        function clearPledgeError() {{
            let overlay = document.getElementById('__pledge_error_overlay');
            if (overlay) overlay.remove();
        }}
        // Listen for successful HMR updates to clear errors
        window.addEventListener('pledge:hmr-success', clearPledgeError);
    </script>
</body>
</html>"#,
        entry
    );

    Html(html)
}

/// Generate an import map for bare specifiers (react, vue, etc.)
/// This allows the browser to resolve bare imports without a bundler
fn generate_import_map(config: &PledgeConfig) -> String {
    let node_modules = config.root.join("node_modules");
    let mut imports = serde_json::Map::new();

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
                                        let entry_field = pkg.get("module").or_else(|| pkg.get("main"))
                                            .and_then(|v| v.as_str()).unwrap_or("index.js");
                                        imports.insert(pkg_name, serde_json::Value::String(
                                            format!("/node_modules/{}/{}/{}", name, sub_name, entry_field)
                                                .replace('\\', "/")
                                        ));
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
                        let entry_field = pkg.get("module").or_else(|| pkg.get("main"))
                            .and_then(|v| v.as_str()).unwrap_or("index.js");
                        imports.insert(name, serde_json::Value::String(
                            format!("/node_modules/{}/{}", name, entry_field).replace('\\', "/")
                        ));
                    }
                }
            }
        }
    }

    serde_json::json!({ "imports": imports }).to_string()
}

/// Error overlay endpoint — returns error info as JSON for programmatic access
async fn error_overlay_handler(
    State(state): State<Arc<DevServerState>>,
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
    // First, try serving from public/ directory (static assets)
    let public_path = state.config.root.join("public").join(&path);
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
        for ext in &["tsx", "ts", "jsx", "js", "mjs", "css", "json"] {
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

    // Determine module kind from extension
    let ext_str = full_path.extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    let kind = ModuleKind::from_extension(&ext_str);

    // Transform using Oxc (JSX → JS, TS type stripping) or Lightning CSS
    let file_path = full_path.to_str().unwrap_or("");
    let transform_output = match pledge_transform::transform(&source_str, kind, file_path, false, &state.config) {
        Ok(output) => output,
        Err(e) => {
            // Send error to all HMR clients via WebSocket
            let error_update = HmrUpdate {
                update_type: "error".to_string(),
                path: path.clone(),
                message: Some(format!("{}", e)),
                file: Some(file_path.to_string()),
                css: None,
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
    };

    // CSS files: serve as text/css with HMR support
    if transform_output.is_css {
        return (
            [
                (header::CONTENT_TYPE, "text/css; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            transform_output.code,
        ).into_response();
    }

    // JS/TS files: rewrite imports and add HMR boundary
    let transformed = rewrite_imports(&transform_output.code, &path, &state.config.resolve_alias);

    // Add HMR boundary code for JSX/TSX/JS files
    let module_with_hmr = if path.ends_with(".tsx") || path.ends_with(".jsx") || path.ends_with(".ts") || path.ends_with(".js") {
        format!(
            "{}\n// HMR boundary\nif (import.meta.hot) {{\nimport.meta.hot.accept();\n}}",
            transformed
        )
    } else {
        transformed
    };

    // Add source map comment if available
    let final_output = if let Some(ref _source_map) = transform_output.source_map {
        format!("{}\n//# sourceMappingURL=pledge://{}", module_with_hmr, path)
    } else {
        module_with_hmr
    };

    (
        [
            (header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        final_output,
    )
        .into_response()
}

/// Rewrite import/export specifiers to browser-compatible URLs.
/// Converts `./foo` → `./foo.js`, `../bar` → `../bar.js`, etc.
/// Also rewrites resolve aliases (e.g., `@/components` → `/src/components`).
/// Bare specifiers like `react` are left as-is (browser must resolve via import map).
fn rewrite_imports(code: &str, _current_module_path: &str, aliases: &[pledgepack_core::PathAlias]) -> String {
    let mut result = code.to_string();

    // Rewrite relative import/export specifiers to include .js extension
    // This is a simple regex-like approach; Oxc could do this in AST but
    // for dev server speed, string rewriting is sufficient.
    for pattern in ["from \"", "from '", "import \"", "import '", "import(", "export * from \"", "export * from '"] {
        while let Some(pos) = result.find(pattern) {
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
                continue;
            }

            if let Some(end) = rest.find(closing_quote) {
                let specifier = &rest[..end];
                if specifier.starts_with("./") || specifier.starts_with("../") {
                    let new_spec = add_js_extension(specifier);
                    let abs_start = after_pattern;
                    let abs_end = abs_start + end;
                    result.replace_range(abs_start..abs_end, &new_spec);
                }
            }

            // Move past this occurrence to avoid infinite loop
            // We need to break if no more occurrences
            break;
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
                    let after_alias = pos + search.len();
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

/// Add .js extension to a relative specifier if it doesn't have one
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
        format!("{}.js", specifier)
    }
}

/// WebSocket endpoint for HMR updates
async fn hmr_websocket_handler(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<DevServerState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_hmr_connection(socket))
}

async fn handle_hmr_connection(
    mut socket: axum::extract::ws::WebSocket,
) {
    info!("HMR client connected");

    // Send initial connection confirmation
    let hello = serde_json::json!({
        "type": "connected",
        "message": "Pledge HMR connected"
    });
    let _ = socket.send(Message::Text(hello.to_string().into()));

    // Keep connection alive — file watcher pushes via hmr_tx
    while let Some(msg) = socket.recv().await {
        match msg {
            Ok(Message::Ping(data)) => {
                let _ = socket.send(Message::Pong(data));
            }
            Ok(Message::Close(_)) => {
                info!("HMR client disconnected");
                break;
            }
            _ => {}
        }
    }
}

/// File watcher using notify crate
fn start_file_watcher(root: PathBuf, tx: mpsc::UnboundedSender<HmrUpdate>) {
    use notify::{Watcher, RecursiveMode, Event, EventKind};
    use std::time::Duration;

    let (notify_tx, notify_rx) = std::sync::mpsc::channel::<notify::Result<Event>>();

    let mut watcher = match notify::recommended_watcher(notify_tx) {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("Failed to create file watcher: {}", e);
            return;
        }
    };

    // Watch the root directory recursively
    if let Err(e) = watcher.watch(&root, RecursiveMode::Recursive) {
        tracing::error!("Failed to watch {:?}: {}", root, e);
        return;
    }

    info!("File watcher started on {}", root.display());

    // Debounce: collect events for 200ms before sending
    let mut last_event_time: Option<std::time::Instant> = None;
    let mut pending_path: Option<PathBuf> = None;
    let mut pending_ext: Option<String> = None;

    loop {
        match notify_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Ok(event)) => {
                if let EventKind::Modify(_) | EventKind::Create(_) = event.kind {
                    for path in &event.paths {
                        // Only watch source files
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "css" | "json") {
                            pending_path = Some(path.clone());
                            pending_ext = Some(ext.to_string());
                            last_event_time = Some(std::time::Instant::now());
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("File watcher error: {}", e);
            }
            Err(_) => {
                // Timeout — check if we have a pending event to send
                if let (Some(path), Some(time)) = (&pending_path, last_event_time) {
                    if time.elapsed() > Duration::from_millis(150) {
                        // Debounce period passed, send the update
                        let rel_path = path.strip_prefix(&root)
                            .unwrap_or(path)
                            .to_string_lossy()
                            .replace('\\', "/");
                        let update = HmrUpdate {
                            update_type: "update".to_string(),
                            path: rel_path.clone(),
                            message: None,
                            file: None,
                            css: if pending_ext.as_deref() == Some("css") {
                                // Read the CSS file content for inline HMR
                                std::fs::read_to_string(path).ok()
                            } else {
                                None
                            },
                        };
                        let _ = tx.send(update);
                        pending_path = None;
                        pending_ext = None;
                        last_event_time = None;
                    }
                }
            }
        }
    }
}

/// Broadcast HMR updates to all connected WebSocket clients
async fn hmr_broadcast_loop(
    state: Arc<DevServerState>,
    mut hmr_rx: mpsc::UnboundedReceiver<HmrUpdate>,
) {
    // Track connected WebSocket clients
    // In this architecture, the broadcast loop receives updates from hmr_tx
    // and needs to forward them to all connected clients.
    // Since we can't share WebSocket connections across tasks easily,
    // we use a broadcast pattern with client channels.
    while let Some(update) = hmr_rx.recv().await {
        info!("HMR update: {} (type: {})", update.path, update.update_type);
        // Broadcast to all registered client channels
        let clients = state.hmr_clients.read().await;
        for client_tx in clients.iter() {
            let _ = client_tx.send(update.clone());
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
    use futures_util::{SinkExt, StreamExt};

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
            Ok(axum::extract::ws::Message::Text(text)) => Some(Ok(Message::Text(text.into()))),
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
            Ok(Message::Text(text)) => Some(Ok(axum::extract::ws::Message::Text(text.into()))),
            Ok(Message::Binary(bin)) => Some(Ok(axum::extract::ws::Message::Binary(bin))),
            Ok(Message::Ping(data)) => Some(Ok(axum::extract::ws::Message::Ping(data))),
            Ok(Message::Pong(data)) => Some(Ok(axum::extract::ws::Message::Pong(data))),
            Ok(Message::Close(_)) => Some(Ok(axum::extract::ws::Message::Close(None))),
            Err(_) => None,
        }
    }).forward(client_sink);

    tokio::select! {
        _ = client_to_target => {},
        _ = target_to_client => {},
    }

    info!("WS Proxy: connection closed");
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
