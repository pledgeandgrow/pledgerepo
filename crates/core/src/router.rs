// File-based routing — PledgeStack/Next.js-style app directory convention
//
// Scans an `app/` directory and builds a route table from the file system:
//   app/page.tsx              → /
//   app/about/page.tsx        → /about
//   app/blog/[slug]/page.tsx  → /blog/:slug
//   app/blog/[...slug]/page.tsx → /blog/*slug
//   app/blog/[[...slug]]/page.tsx → /blog/*slug (optional catch-all)
//   app/(marketing)/page.tsx → / (route group — skipped from URL)
//   app/@analytics/page.tsx  → parallel route (skipped from URL)
//   app/layout.tsx            → shared layout wrapper
//   app/template.tsx          → like layout but re-renders on navigation
//   app/loading.tsx           → loading UI for route segment
//   app/error.tsx             → error boundary for route segment
//   app/global-error.tsx      → root-level error boundary
//   app/not-found.tsx         → 404 page
//   app/head.tsx              → per-segment head metadata
//   app/route.ts              → API route handler (GET, POST, etc.)
//   app/api/hello/route.ts    → /api/hello (API route handler)
//   app/middleware.ts         → middleware (applies to all routes)
//
// PledgeStack extensions:
//   .psx files are treated like .tsx (PledgeStack's Rust+JSX extension)

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// A single route discovered from the app directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// URL path pattern (e.g., "/", "/about", "/blog/:slug")
    pub pattern: String,
    /// Relative file path from project root (e.g., "app/about/page.tsx")
    pub file: String,
    /// Route segments parsed from the path (e.g., ["blog", ":slug"])
    pub segments: Vec<RouteSegment>,
    /// Layout file for this route if one exists (nearest parent layout)
    pub layout: Option<String>,
    /// Template file for this route if one exists (re-renders on navigation)
    pub template: Option<String>,
    /// Loading component for this route if one exists
    pub loading: Option<String>,
    /// Error boundary for this route if one exists
    pub error_boundary: Option<String>,
    /// Head component for this route if one exists
    pub head: Option<String>,
    /// Depth in the route tree (0 = root)
    pub depth: usize,
    /// Route type (page, api route handler, or page with API)
    pub route_type: RouteType,
}

/// Type of route discovered
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RouteType {
    /// React page component (page.tsx)
    Page,
    /// API route handler (route.ts/route.tsx)
    ApiRoute,
}

/// A parsed route segment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RouteSegment {
    /// Static segment (e.g., "about")
    Static(String),
    /// Dynamic param (e.g., [slug] → :slug)
    Param(String),
    /// Catch-all param (e.g., [...slug] → *slug)
    CatchAll(String),
    /// Optional catch-all (e.g., [[...slug]] → *slug, matches with or without)
    OptionalCatchAll(String),
    /// Route group (e.g., (marketing) — skipped from URL pattern)
    Group(String),
    /// Parallel route slot (e.g., @analytics — skipped from URL pattern)
    Slot(String),
}

impl RouteSegment {
    /// Convert segment to URL pattern string
    pub fn to_pattern(&self) -> String {
        match self {
            RouteSegment::Static(s) => s.clone(),
            RouteSegment::Param(p) => format!(":{}", p),
            RouteSegment::CatchAll(p) => format!("*{}", p),
            RouteSegment::OptionalCatchAll(p) => format!("*{}", p),
            RouteSegment::Group(_) => String::new(), // Skipped from URL
            RouteSegment::Slot(_) => String::new(),  // Skipped from URL
        }
    }

    /// Whether this segment contributes to the URL pattern
    pub fn is_url_segment(&self) -> bool {
        !matches!(self, RouteSegment::Group(_) | RouteSegment::Slot(_))
    }
}

/// The full route table discovered from the app directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteTable {
    /// All page routes sorted by specificity (static before dynamic)
    pub routes: Vec<Route>,
    /// Root layout if one exists (app/layout.tsx)
    pub root_layout: Option<String>,
    /// Global not-found page (app/not-found.tsx)
    pub not_found: Option<String>,
    /// Global error boundary (app/global-error.tsx)
    pub global_error: Option<String>,
    /// Root middleware (app/middleware.ts)
    pub middleware: Option<String>,
    /// The app directory path relative to root
    pub app_dir: String,
}

/// Page file names that define a route
const PAGE_FILES: &[&str] = &[
    "page.tsx", "page.ts", "page.jsx", "page.js", "page.psx",
];

/// Route handler file names (API endpoints)
const ROUTE_FILES: &[&str] = &[
    "route.tsx", "route.ts", "route.jsx", "route.js", "route.psx",
];

/// Layout file names
const LAYOUT_FILES: &[&str] = &[
    "layout.tsx", "layout.ts", "layout.jsx", "layout.js", "layout.psx",
];

/// Template file names (like layout but re-renders on navigation)
const TEMPLATE_FILES: &[&str] = &[
    "template.tsx", "template.ts", "template.jsx", "template.js", "template.psx",
];

/// Loading file names
const LOADING_FILES: &[&str] = &[
    "loading.tsx", "loading.ts", "loading.jsx", "loading.js", "loading.psx",
];

/// Error boundary file names
const ERROR_FILES: &[&str] = &[
    "error.tsx", "error.ts", "error.jsx", "error.js", "error.psx",
];

/// Not-found file names
const NOT_FOUND_FILES: &[&str] = &[
    "not-found.tsx", "not-found.ts", "not-found.jsx", "not-found.js", "not-found.psx",
];

/// Global error boundary file names
const GLOBAL_ERROR_FILES: &[&str] = &[
    "global-error.tsx", "global-error.ts", "global-error.jsx", "global-error.js", "global-error.psx",
];

/// Head file names (per-segment head metadata)
const HEAD_FILES: &[&str] = &[
    "head.tsx", "head.ts", "head.jsx", "head.js", "head.psx",
];

/// Middleware file names
const MIDDLEWARE_FILES: &[&str] = &[
    "middleware.ts", "middleware.tsx", "middleware.js", "middleware.jsx", "middleware.psx",
];

/// Scan the app directory and build a route table
pub fn scan_app_dir(root: &Path, app_dir: &str) -> anyhow::Result<RouteTable> {
    let app_path = root.join(app_dir);
    if !app_path.is_dir() {
        return Ok(RouteTable {
            routes: vec![],
            root_layout: None,
            not_found: None,
            global_error: None,
            middleware: None,
            app_dir: app_dir.to_string(),
        });
    }

    let mut routes: Vec<Route> = vec![];
    let mut root_layout: Option<String> = None;
    let mut not_found: Option<String> = None;
    let mut global_error: Option<String> = None;
    let mut middleware: Option<String> = None;

    // Check for root-level files
    root_layout = find_file(&app_path, LAYOUT_FILES)
        .map(|p| relative_path(root, &p));

    not_found = find_file(&app_path, NOT_FOUND_FILES)
        .map(|p| relative_path(root, &p));

    global_error = find_file(&app_path, GLOBAL_ERROR_FILES)
        .map(|p| relative_path(root, &p));

    middleware = find_file(&app_path, MIDDLEWARE_FILES)
        .map(|p| relative_path(root, &p));

    // Recursively scan for page and route handler files
    scan_dir_for_routes(root, &app_path, &app_path, &mut routes, 0)?;

    // Sort routes: static segments before dynamic, shallower before deeper
    routes.sort_by(|a, b| {
        for (sa, sb) in a.segments.iter().zip(b.segments.iter()) {
            let rank_a = match sa {
                RouteSegment::Static(_) => 0,
                RouteSegment::Param(_) => 1,
                RouteSegment::CatchAll(_) => 2,
                RouteSegment::OptionalCatchAll(_) => 3,
                RouteSegment::Group(_) => 0,
                RouteSegment::Slot(_) => 0,
            };
            let rank_b = match sb {
                RouteSegment::Static(_) => 0,
                RouteSegment::Param(_) => 1,
                RouteSegment::CatchAll(_) => 2,
                RouteSegment::OptionalCatchAll(_) => 3,
                RouteSegment::Group(_) => 0,
                RouteSegment::Slot(_) => 0,
            };
            if rank_a != rank_b {
                return rank_a.cmp(&rank_b);
            }
        }
        a.segments.len().cmp(&b.segments.len())
    });

    // Attach layouts, templates, loading, error, and head to routes
    for route in &mut routes {
        let route_file_path = root.join(&route.file);
        let route_dir = route_file_path.parent().unwrap_or(&app_path).to_path_buf();
        let mut current_dir = route_dir.as_path();
        let mut best_layout: Option<String> = None;
        let mut best_template: Option<String> = None;

        // Walk up from the route's directory to find the nearest layout and template
        loop {
            if best_layout.is_none() {
                if let Some(layout_path) = find_file(current_dir, LAYOUT_FILES) {
                    best_layout = Some(relative_path(root, &layout_path));
                }
            }
            if best_template.is_none() {
                if let Some(template_path) = find_file(current_dir, TEMPLATE_FILES) {
                    best_template = Some(relative_path(root, &template_path));
                }
            }
            if current_dir == app_path.as_path() {
                break;
            }
            current_dir = match current_dir.parent() {
                Some(p) if p >= app_path.as_path() => p,
                _ => break,
            };
        }

        route.layout = best_layout.or_else(|| root_layout.clone());
        route.template = best_template;

        // Find loading, error, and head (nearest in the route's directory)
        route.loading = find_file(&route_dir, LOADING_FILES)
            .map(|p| relative_path(root, &p));
        route.error_boundary = find_file(&route_dir, ERROR_FILES)
            .map(|p| relative_path(root, &p));
        route.head = find_file(&route_dir, HEAD_FILES)
            .map(|p| relative_path(root, &p));
    }

    Ok(RouteTable {
        routes,
        root_layout,
        not_found,
        global_error,
        middleware,
        app_dir: app_dir.to_string(),
    })
}

/// Recursively scan a directory for page and route handler files
fn scan_dir_for_routes(
    root: &Path,
    app_path: &Path,
    dir: &Path,
    routes: &mut Vec<Route>,
    depth: usize,
) -> anyhow::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    // Check if this directory has a page file
    if let Some(page_path) = find_file(dir, PAGE_FILES) {
        let segments = parse_route_segments(dir, app_path);
        let pattern = build_pattern(&segments);
        let file = relative_path(root, &page_path);

        routes.push(Route {
            pattern,
            file,
            segments,
            layout: None,
            template: None,
            loading: None,
            error_boundary: None,
            head: None,
            depth,
            route_type: RouteType::Page,
        });
    }

    // Check if this directory has a route handler file (API endpoint)
    if let Some(route_path) = find_file(dir, ROUTE_FILES) {
        let segments = parse_route_segments(dir, app_path);
        let pattern = build_pattern(&segments);
        let file = relative_path(root, &route_path);

        routes.push(Route {
            pattern,
            file,
            segments,
            layout: None,
            template: None,
            loading: None,
            error_boundary: None,
            head: None,
            depth,
            route_type: RouteType::ApiRoute,
        });
    }

    // Recurse into subdirectories
    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Skip private folders (starting with _)
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('_') || name.starts_with('.') {
                    continue;
                }
            }
            scan_dir_for_routes(root, app_path, &path, routes, depth + 1)?;
        }
    }

    Ok(())
}

/// Parse route segments from a directory path relative to the app root
fn parse_route_segments(dir: &Path, app_path: &Path) -> Vec<RouteSegment> {
    let rel = dir.strip_prefix(app_path).unwrap_or(dir);

    let mut segments = vec![];
    for comp in rel.components() {
        let name = comp.as_os_str().to_string_lossy().to_string();
        if name.is_empty() || name == "." {
            continue;
        }

        if name.starts_with("[[") && name.ends_with("]]") {
            // Optional catch-all: [[...slug]]
            let inner = &name[2..name.len()-2];
            if inner.starts_with("...") {
                segments.push(RouteSegment::OptionalCatchAll(inner[3..].to_string()));
            }
        } else if name.starts_with('[') && name.ends_with(']') {
            let inner = &name[1..name.len()-1];
            if inner.starts_with("...") {
                // Catch-all: [...slug]
                segments.push(RouteSegment::CatchAll(inner[3..].to_string()));
            } else {
                // Dynamic param: [slug]
                segments.push(RouteSegment::Param(inner.to_string()));
            }
        } else if name.starts_with('(') && name.ends_with(')') {
            // Route group: (marketing) — skipped from URL
            segments.push(RouteSegment::Group(name[1..name.len()-1].to_string()));
        } else if name.starts_with('@') {
            // Parallel route slot: @analytics — skipped from URL
            segments.push(RouteSegment::Slot(name[1..].to_string()));
        } else {
            segments.push(RouteSegment::Static(name));
        }
    }

    segments
}

/// Build a URL pattern from route segments (skips groups and slots)
fn build_pattern(segments: &[RouteSegment]) -> String {
    let url_segments: Vec<String> = segments
        .iter()
        .filter(|s| s.is_url_segment())
        .map(|s| s.to_pattern())
        .collect();
    
    if url_segments.is_empty() {
        return "/".to_string();
    }
    format!("/{}", url_segments.join("/"))
}

/// Find the first matching file from a list of candidate names in a directory
fn find_file(dir: &Path, candidates: &[&str]) -> Option<PathBuf> {
    for name in candidates {
        let path = dir.join(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Get a path relative to the root, with forward slashes
fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

impl RouteTable {
    /// Generate a virtual router module that imports all pages and exports a match function
    pub fn generate_router_module(&self) -> String {
        let imports = self.generate_imports("/");
        let routes_js = self.generate_routes_js();
        let not_found_var = self.generate_not_found_var("/");
        let global_error_var = self.generate_global_error_var("/");
        let middleware_var = self.generate_middleware_var("/");

        format!(
            r#"// Auto-generated by Pledge — do not edit
// File-based routing from app/ directory

import React from "react";
{imports}

const routes = [
{routes_js}];

const notFound = {not_found_var};
const globalError = {global_error_var};
const middleware = {middleware_var};

// Simple path matcher: converts "/blog/:slug" to a regex
function matchRoute(pattern, pathname) {{
  if (pattern === "/" && pathname === "/") return {{}};
  const paramNames = [];
  const regexStr = pattern
    .replace(/:[^/]+/g, (m) => {{ paramNames.push(m.slice(1)); return "[^/]+"; }})
    .replace(/\*[^/]+/g, (m) => {{ paramNames.push(m.slice(1)); return ".*"; }});
  const regex = new RegExp("^" + regexStr + "$");
  const match = pathname.match(regex);
  if (!match) return null;
  const params = {{}};
  paramNames.forEach((name, i) => {{ params[name] = match[i + 1]; }});
  return params;
}}

export function render(pathname) {{
  for (const route of routes) {{
    const params = matchRoute(route.pattern, pathname);
    if (params) {{
      let element = React.createElement(route.component, params);
      if (route.loading) {{
        element = React.createElement(route.loading, {{ children: element }});
      }}
      if (route.template) {{
        element = React.createElement(route.template, {{ children: element, params }});
      }}
      if (route.layout) {{
        element = React.createElement(route.layout, {{ children: element, params }});
      }}
      return element;
    }}
  }}
  if (notFound) return React.createElement(notFound);
  return null;
}}

export {{ routes, middleware }};
"#
        )
    }

    /// Generate a virtual router module with relative imports for production build.
    /// `relative_prefix` is the path from the generated module's directory to the project root
    /// (e.g., "../../" if the module is in .pledge/gen/).
    pub fn generate_router_module_build(&self, relative_prefix: &str) -> String {
        let imports = self.generate_imports(relative_prefix);
        let routes_js = self.generate_routes_js();
        let not_found_var = self.generate_not_found_var(relative_prefix);
        let global_error_var = self.generate_global_error_var(relative_prefix);
        let middleware_var = self.generate_middleware_var(relative_prefix);

        format!(
            r#"// Auto-generated by Pledge build — do not edit
// File-based routing from app/ directory

import React from "react";
{imports}

const routes = [
{routes_js}];

const notFound = {not_found_var};
const globalError = {global_error_var};
const middleware = {middleware_var};

// Simple path matcher: converts "/blog/:slug" to a regex
function matchRoute(pattern, pathname) {{
  if (pattern === "/" && pathname === "/") return {{}};
  const paramNames = [];
  const regexStr = pattern
    .replace(/:[^/]+/g, (m) => {{ paramNames.push(m.slice(1)); return "[^/]+"; }})
    .replace(/\*[^/]+/g, (m) => {{ paramNames.push(m.slice(1)); return ".*"; }});
  const regex = new RegExp("^" + regexStr + "$");
  const match = pathname.match(regex);
  if (!match) return null;
  const params = {{}};
  paramNames.forEach((name, i) => {{ params[name] = match[i + 1]; }});
  return params;
}}

export function render(pathname) {{
  for (const route of routes) {{
    const params = matchRoute(route.pattern, pathname);
    if (params) {{
      let element = React.createElement(route.component, params);
      if (route.loading) {{
        element = React.createElement(route.loading, {{ children: element }});
      }}
      if (route.template) {{
        element = React.createElement(route.template, {{ children: element, params }});
      }}
      if (route.layout) {{
        element = React.createElement(route.layout, {{ children: element, params }});
      }}
      return element;
    }}
  }}
  if (notFound) return React.createElement(notFound);
  return null;
}}

export {{ routes, middleware }};
"#
        )
    }

    /// Generate import statements for all routes
    fn generate_imports(&self, prefix: &str) -> String {
        let mut imports = String::new();
        for (i, route) in self.routes.iter().enumerate() {
            let import_name = format!("Page{}", i);
            imports.push_str(&format!(
                "import {} from \"{}{}\";\n",
                import_name, prefix, route.file
            ));

            if let Some(layout) = &route.layout {
                let layout_name = format!("Layout{}", i);
                imports.push_str(&format!(
                    "import {} from \"{}{}\";\n",
                    layout_name, prefix, layout
                ));
            }
            if let Some(template) = &route.template {
                let template_name = format!("Template{}", i);
                imports.push_str(&format!(
                    "import {} from \"{}{}\";\n",
                    template_name, prefix, template
                ));
            }
            if let Some(loading) = &route.loading {
                let loading_name = format!("Loading{}", i);
                imports.push_str(&format!(
                    "import {} from \"{}{}\";\n",
                    loading_name, prefix, loading
                ));
            }
            if let Some(error) = &route.error_boundary {
                let error_name = format!("Error{}", i);
                imports.push_str(&format!(
                    "import {} from \"{}{}\";\n",
                    error_name, prefix, error
                ));
            }
            if let Some(head) = &route.head {
                let head_name = format!("Head{}", i);
                imports.push_str(&format!(
                    "import {} from \"{}{}\";\n",
                    head_name, prefix, head
                ));
            }
        }
        // Import not-found component
        if let Some(nf) = &self.not_found {
            imports.push_str(&format!(
                "import NotFound from \"{}{}\";\n",
                prefix, nf
            ));
        }
        // Import global error boundary
        if let Some(ge) = &self.global_error {
            imports.push_str(&format!(
                "import GlobalError from \"{}{}\";\n",
                prefix, ge
            ));
        }
        // Import middleware
        if let Some(mw) = &self.middleware {
            imports.push_str(&format!(
                "import Middleware from \"{}{}\";\n",
                prefix, mw
            ));
        }
        imports
    }

    /// Generate the routes JavaScript array
    fn generate_routes_js(&self) -> String {
        let mut routes_js = String::new();
        for (i, route) in self.routes.iter().enumerate() {
            let import_name = format!("Page{}", i);
            let layout_str = route.layout.as_ref()
                .map(|_| format!("Layout{}", i))
                .map(|n| n.to_string())
                .unwrap_or_else(|| "null".to_string());
            let template_str = route.template.as_ref()
                .map(|_| format!("Template{}", i))
                .map(|n| n.to_string())
                .unwrap_or_else(|| "null".to_string());
            let loading_str = route.loading.as_ref()
                .map(|_| format!("Loading{}", i))
                .map(|n| n.to_string())
                .unwrap_or_else(|| "null".to_string());
            let error_str = route.error_boundary.as_ref()
                .map(|_| format!("Error{}", i))
                .map(|n| n.to_string())
                .unwrap_or_else(|| "null".to_string());
            let head_str = route.head.as_ref()
                .map(|_| format!("Head{}", i))
                .map(|n| n.to_string())
                .unwrap_or_else(|| "null".to_string());
            let type_str = match route.route_type {
                RouteType::Page => "\"page\"",
                RouteType::ApiRoute => "\"api\"",
            };

            routes_js.push_str(&format!(
                "  {{ pattern: \"{}\", component: {}, layout: {}, template: {}, loading: {}, error: {}, head: {}, type: {} }},\n",
                route.pattern, import_name, layout_str, template_str, loading_str, error_str, head_str, type_str
            ));
        }
        routes_js
    }

    /// Generate the not-found variable reference
    fn generate_not_found_var(&self, _prefix: &str) -> String {
        if self.not_found.is_some() {
            "NotFound".to_string()
        } else {
            "null".to_string()
        }
    }

    /// Generate the global error variable reference
    fn generate_global_error_var(&self, _prefix: &str) -> String {
        if self.global_error.is_some() {
            "GlobalError".to_string()
        } else {
            "null".to_string()
        }
    }

    /// Generate the middleware variable reference
    fn generate_middleware_var(&self, _prefix: &str) -> String {
        if self.middleware.is_some() {
            "Middleware".to_string()
        } else {
            "null".to_string()
        }
    }

    /// Check if a pathname matches any route
    pub fn match_path(&self, pathname: &str) -> Option<&Route> {
        for route in &self.routes {
            if path_matches_pattern(&route.pattern, pathname) {
                return Some(route);
            }
        }
        None
    }
}

/// Check if a pathname matches a pattern (simple version)
fn path_matches_pattern(pattern: &str, pathname: &str) -> bool {
    if pattern == "/" {
        return pathname == "/";
    }

    let pattern_parts: Vec<&str> = pattern.trim_start_matches('/').split('/').collect();
    let path_parts: Vec<&str> = pathname.trim_start_matches('/').split('/').collect();

    if pattern_parts.len() != path_parts.len() {
        // Check for catch-all (required or optional)
        if let Some(last) = pattern_parts.last() {
            if last.starts_with('*') {
                // Catch-all matches if path has at least pattern_parts - 1 segments
                return path_parts.len() >= pattern_parts.len() - 1;
            }
        }
        // Check for optional catch-all at the end — matches with or without the segment
        // Optional catch-all is represented as *param but should also match when the segment is absent
        return false;
    }

    for (pp, xp) in pattern_parts.iter().zip(path_parts.iter()) {
        if pp.starts_with(':') || pp.starts_with('*') {
            continue;
        }
        if pp != xp {
            return false;
        }
    }
    true
}
