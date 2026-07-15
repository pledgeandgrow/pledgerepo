// File-based routing — Next.js/Expo-style app directory convention
//
// Scans an `app/` directory and builds a route table from the file system:
//   app/page.tsx              → /
//   app/about/page.tsx        → /about
//   app/blog/[slug]/page.tsx  → /blog/:slug
//   app/blog/[...slug]/page.tsx → /blog/*slug
//   app/layout.tsx            → shared layout wrapper
//   app/loading.tsx           → loading UI for route segment
//   app/error.tsx             → error boundary for route segment
//   app/not-found.tsx         → 404 page

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
    /// Loading component for this route if one exists
    pub loading: Option<String>,
    /// Error boundary for this route if one exists
    pub error_boundary: Option<String>,
    /// Depth in the route tree (0 = root)
    pub depth: usize,
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
}

impl RouteSegment {
    /// Convert segment to URL pattern string
    pub fn to_pattern(&self) -> String {
        match self {
            RouteSegment::Static(s) => s.clone(),
            RouteSegment::Param(p) => format!(":{}", p),
            RouteSegment::CatchAll(p) => format!("*{}", p),
        }
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
    /// The app directory path relative to root
    pub app_dir: String,
}

/// Page file names that define a route
const PAGE_FILES: &[&str] = &["page.tsx", "page.ts", "page.jsx", "page.js"];

/// Layout file names
const LAYOUT_FILES: &[&str] = &["layout.tsx", "layout.ts", "layout.jsx", "layout.js"];

/// Loading file names
const LOADING_FILES: &[&str] = &["loading.tsx", "loading.ts", "loading.jsx", "loading.js"];

/// Error boundary file names
const ERROR_FILES: &[&str] = &["error.tsx", "error.ts", "error.jsx", "error.js"];

/// Not-found file names
const NOT_FOUND_FILES: &[&str] = &["not-found.tsx", "not-found.ts", "not-found.jsx", "not-found.js"];

/// Scan the app directory and build a route table
pub fn scan_app_dir(root: &Path, app_dir: &str) -> anyhow::Result<RouteTable> {
    let app_path = root.join(app_dir);
    if !app_path.is_dir() {
        return Ok(RouteTable {
            routes: vec![],
            root_layout: None,
            not_found: None,
            app_dir: app_dir.to_string(),
        });
    }

    let mut routes: Vec<Route> = vec![];
    let mut root_layout: Option<String> = None;
    let mut not_found: Option<String> = None;

    // Check for root-level files
    root_layout = find_file(&app_path, LAYOUT_FILES)
        .map(|p| relative_path(root, &p));

    not_found = find_file(&app_path, NOT_FOUND_FILES)
        .map(|p| relative_path(root, &p));

    // Recursively scan for page files
    scan_dir_for_pages(root, &app_path, &mut routes, 0)?;

    // Sort routes: static segments before dynamic, shallower before deeper
    routes.sort_by(|a, b| {
        for (sa, sb) in a.segments.iter().zip(b.segments.iter()) {
            let rank_a = match sa { RouteSegment::Static(_) => 0, RouteSegment::Param(_) => 1, RouteSegment::CatchAll(_) => 2 };
            let rank_b = match sb { RouteSegment::Static(_) => 0, RouteSegment::Param(_) => 1, RouteSegment::CatchAll(_) => 2 };
            if rank_a != rank_b {
                return rank_a.cmp(&rank_b);
            }
        }
        a.segments.len().cmp(&b.segments.len())
    });

    // Attach layouts to routes
    for route in &mut routes {
        let route_file_path = root.join(&route.file);
        let route_dir = route_file_path.parent().unwrap_or(&app_path).to_path_buf();
        let mut current_dir = route_dir.as_path();
        let mut best_layout: Option<String> = None;

        // Walk up from the route's directory to find the nearest layout
        loop {
            if let Some(layout_path) = find_file(current_dir, LAYOUT_FILES) {
                let rel = relative_path(root, &layout_path);
                // The root layout is handled separately — only use it if no closer layout
                if best_layout.is_none() {
                    best_layout = Some(rel);
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

        // Find loading and error boundaries (nearest in the route's directory)
        route.loading = find_file(&route_dir, LOADING_FILES)
            .map(|p| relative_path(root, &p));
        route.error_boundary = find_file(&route_dir, ERROR_FILES)
            .map(|p| relative_path(root, &p));
    }

    Ok(RouteTable {
        routes,
        root_layout,
        not_found,
        app_dir: app_dir.to_string(),
    })
}

/// Recursively scan a directory for page files
fn scan_dir_for_pages(
    root: &Path,
    dir: &Path,
    routes: &mut Vec<Route>,
    depth: usize,
) -> anyhow::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    // Check if this directory has a page file
    if let Some(page_path) = find_file(dir, PAGE_FILES) {
        let segments = parse_route_segments(dir, root);
        let pattern = build_pattern(&segments);
        let file = relative_path(root, &page_path);

        routes.push(Route {
            pattern,
            file,
            segments,
            layout: None, // Filled in later
            loading: None,
            error_boundary: None,
            depth,
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
            scan_dir_for_pages(root, &path, routes, depth + 1)?;
        }
    }

    Ok(())
}

/// Parse route segments from a directory path relative to the app root
fn parse_route_segments(dir: &Path, root: &Path) -> Vec<RouteSegment> {
    let app_dir = root.join("app");
    let rel = dir.strip_prefix(&app_dir).unwrap_or(dir);

    let mut segments = vec![];
    for comp in rel.components() {
        let name = comp.as_os_str().to_string_lossy().to_string();
        if name.is_empty() || name == "." {
            continue;
        }

        if name.starts_with('[') && name.ends_with(']') {
            let inner = &name[1..name.len()-1];
            if inner.starts_with("...") {
                // Catch-all: [...slug]
                segments.push(RouteSegment::CatchAll(inner[3..].to_string()));
            } else {
                // Dynamic param: [slug]
                segments.push(RouteSegment::Param(inner.to_string()));
            }
        } else {
            segments.push(RouteSegment::Static(name));
        }
    }

    segments
}

/// Build a URL pattern from route segments
fn build_pattern(segments: &[RouteSegment]) -> String {
    if segments.is_empty() {
        return "/".to_string();
    }
    let parts: Vec<String> = segments.iter().map(|s| s.to_pattern()).collect();
    format!("/{}", parts.join("/"))
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
        let mut imports = String::new();
        let mut routes_js = String::new();

        for (i, route) in self.routes.iter().enumerate() {
            let import_name = format!("Page{}", i);
            imports.push_str(&format!(
                "import {} from \"/{}\";\n",
                import_name, route.file
            ));

            if let Some(layout) = &route.layout {
                let layout_name = format!("Layout{}", i);
                imports.push_str(&format!(
                    "import {} from \"/{}\";\n",
                    layout_name, layout
                ));
                routes_js.push_str(&format!(
                    "  {{ pattern: \"{}\", component: {}, layout: {} }},\n",
                    route.pattern, import_name, layout_name
                ));
            } else {
                routes_js.push_str(&format!(
                    "  {{ pattern: \"{}\", component: {}, layout: null }},\n",
                    route.pattern, import_name
                ));
            }
        }

        // Add not-found page
        let not_found_var = if let Some(nf) = &self.not_found {
            imports.push_str(&format!("import NotFound from \"/{}\";\n", nf));
            "NotFound"
        } else {
            "null"
        };

        format!(
            r#"// Auto-generated by Pledge — do not edit
// File-based routing from app/ directory

import React from "react";
{imports}

const routes = [
{routes_js}];

const notFound = {not_found_var};

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
      const element = React.createElement(route.component, params);
      if (route.layout) {{
        return React.createElement(route.layout, {{ children: element, params }});
      }}
      return element;
    }}
  }}
  if (notFound) return React.createElement(notFound);
  return null;
}}

export {{ routes }};
"#
        )
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
        // Check for catch-all
        if let Some(last) = pattern_parts.last() {
            if last.starts_with('*') {
                return path_parts.len() >= pattern_parts.len() - 1;
            }
        }
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
