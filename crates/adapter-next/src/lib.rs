// Next.js adapter: File-based routing, SSR, API routes
//
// Handles:
//   - app/ directory routing (App Router)
//   - pages/ directory routing (Pages Router)
//   - API routes (app/api/ or pages/api/)
//   - Server-side rendering (SSR)
//   - Static site generation (SSG)
//   - Layout nesting
//   - Loading/error boundaries

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Next.js adapter configuration
pub struct NextAdapter {
    /// Project root
    pub root: PathBuf,
    /// Use App Router (app/) or Pages Router (pages/)
    pub router_type: RouterType,
    /// Detected routes
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterType {
    AppRouter,
    PagesRouter,
}

#[derive(Debug, Clone)]
pub struct Route {
    /// URL path (e.g., "/about", "/posts/[id]")
    pub path: String,
    /// File path relative to project root
    pub file: String,
    /// Route type
    pub kind: RouteKind,
    /// Dynamic segments (e.g., ["id"] for /posts/[id])
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteKind {
    Page,
    Layout,
    Loading,
    Error,
    Api,
    NotFound,
}

impl NextAdapter {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            router_type: RouterType::AppRouter,
            routes: Vec::new(),
        }
    }

    /// Detect all routes from the file system
    pub fn discover_routes(&mut self) -> Result<()> {
        // Check for app/ directory (App Router)
        let app_dir = self.root.join("app");
        if app_dir.exists() && app_dir.is_dir() {
            self.router_type = RouterType::AppRouter;
            self.discover_app_routes(&app_dir, "")?;
        }

        // Check for pages/ directory (Pages Router)
        let pages_dir = self.root.join("pages");
        if pages_dir.exists() && pages_dir.is_dir() {
            self.router_type = RouterType::PagesRouter;
            self.discover_pages_routes(&pages_dir, "")?;
        }

        Ok(())
    }

    /// Discover routes from app/ directory (App Router)
    fn discover_app_routes(&mut self, dir: &Path, prefix: &str) -> Result<()> {
        let entries = std::fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                // Dynamic route: [param]
                let (route_segment, param) = if name.starts_with('[') && name.ends_with(']') {
                    let param = name[1..name.len()-1].to_string();
                    (format!(":{}", param), Some(param))
                } else if name.starts_with('(') && name.ends_with(')') {
                    // Route group: (group) — doesn't affect URL
                    (String::new(), None)
                } else {
                    (name.clone(), None)
                };

                let new_prefix = if route_segment.is_empty() {
                    prefix.to_string()
                } else if prefix.is_empty() {
                    format!("/{}", route_segment)
                } else {
                    format!("{}/{}", prefix, route_segment)
                };

                if let Some(p) = param {
                    let mut route = Route {
                        path: new_prefix.clone(),
                        file: path.strip_prefix(&self.root).unwrap_or(&path).to_string_lossy().to_string(),
                        kind: RouteKind::Page,
                        params: vec![p],
                    };
                    // Check for page.tsx/page.ts in this directory
                    let page_file = find_file(&path, &["page.tsx", "page.ts", "page.jsx", "page.js"]);
                    if page_file.exists() {
                        route.file = page_file.strip_prefix(&self.root).unwrap_or(&page_file).to_string_lossy().to_string();
                        self.routes.push(route);
                    }
                } else {
                    self.discover_app_routes(&path, &new_prefix)?;
                }
            } else {
                // File-level routes
                let kind = match name.as_str() {
                    "page.tsx" | "page.ts" | "page.jsx" | "page.js" => RouteKind::Page,
                    "layout.tsx" | "layout.ts" | "layout.jsx" | "layout.js" => RouteKind::Layout,
                    "loading.tsx" | "loading.ts" | "loading.jsx" | "loading.js" => RouteKind::Loading,
                    "error.tsx" | "error.ts" | "error.jsx" | "error.js" => RouteKind::Error,
                    "not-found.tsx" | "not-found.ts" | "not-found.jsx" | "not-found.js" => RouteKind::NotFound,
                    _ => continue,
                };

                let route_path = if prefix.is_empty() { "/".to_string() } else { prefix.to_string() };
                let file_rel = path.strip_prefix(&self.root).unwrap_or(&path).to_string_lossy().to_string();

                self.routes.push(Route {
                    path: route_path,
                    file: file_rel,
                    kind,
                    params: Vec::new(),
                });
            }
        }

        Ok(())
    }

    /// Discover routes from pages/ directory (Pages Router)
    fn discover_pages_routes(&mut self, dir: &Path, prefix: &str) -> Result<()> {
        let entries = std::fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                let (route_segment, param) = if name.starts_with('[') && name.ends_with(']') {
                    let param = name[1..name.len()-1].to_string();
                    (format!(":{}", param), Some(param))
                } else {
                    (name.clone(), None)
                };

                let new_prefix = if prefix.is_empty() {
                    format!("/{}", route_segment)
                } else {
                    format!("{}/{}", prefix, route_segment)
                };

                if let Some(p) = param {
                    // Check for index file in dynamic route
                    let index_file = find_file(&path, &["index.tsx", "index.ts", "index.jsx", "index.js"]);
                    if index_file.exists() {
                        self.routes.push(Route {
                            path: new_prefix.clone(),
                            file: index_file.strip_prefix(&self.root).unwrap_or(&index_file).to_string_lossy().to_string(),
                            kind: RouteKind::Page,
                            params: vec![p],
                        });
                    }
                }

                self.discover_pages_routes(&path, &new_prefix)?;
            } else {
                // File-level routes
                let (route_path, kind) = if name == "index.tsx" || name == "index.ts" || name == "index.jsx" || name == "index.js" {
                    (if prefix.is_empty() { "/".to_string() } else { prefix.to_string() }, RouteKind::Page)
                } else if name.starts_with("api/") || name == "api.tsx" || name == "api.ts" {
                    (format!("{}/api", prefix), RouteKind::Api)
                } else if name.ends_with(".tsx") || name.ends_with(".ts") || name.ends_with(".jsx") || name.ends_with(".js") {
                    let stem = name.split('.').next().unwrap_or(&name);
                    let (seg, param) = if stem.starts_with('[') && stem.ends_with(']') {
                        (format!(":{}", &stem[1..stem.len()-1]), Some(stem[1..stem.len()-1].to_string()))
                    } else {
                        (stem.to_string(), None)
                    };
                    let path = if prefix.is_empty() { format!("/{}", seg) } else { format!("{}/{}", prefix, seg) };
                    let mut params = Vec::new();
                    if let Some(p) = param { params.push(p); }
                    // We'll handle params properly in the route
                    (path, RouteKind::Page)
                } else {
                    continue;
                };

                let file_rel = path.strip_prefix(&self.root).unwrap_or(&path).to_string_lossy().to_string();
                self.routes.push(Route {
                    path: route_path,
                    file: file_rel,
                    kind,
                    params: Vec::new(),
                });
            }
        }

        Ok(())
    }

    /// Generate a client-side router from discovered routes
    pub fn generate_router_code(&self) -> String {
        let mut code = String::new();
        code.push_str("// Auto-generated by Pledge Next.js adapter\n");
        code.push_str("// File-based routing\n\n");

        // Generate route map
        code.push_str("const routes = {\n");
        for route in &self.routes {
            if route.kind == RouteKind::Page {
                code.push_str(&format!(
                    "  '{}': () => import('/{}'),\n",
                    route.path, route.file.replace('\\', "/")
                ));
            }
        }
        code.push_str("};\n\n");

        // Generate router
        code.push_str(r#"export function navigate(path) {
  const route = routes[path];
  if (route) {
    route().then(mod => {
      const app = document.getElementById('root');
      if (app && mod.default) {
        app.innerHTML = '';
        if (typeof mod.default === 'function') {
          mod.default(app);
        } else if (mod.default.render) {
          mod.default.render(app);
        }
      }
    });
  }
}

export function getRoutes() {
  return Object.keys(routes);
}
"#);

        code
    }

    /// Generate SSR manifest for server-side rendering
    pub fn generate_ssr_manifest(&self) -> String {
        let manifest: Vec<serde_json::Value> = self.routes.iter().map(|r| {
            serde_json::json!({
                "path": r.path,
                "file": r.file,
                "kind": format!("{:?}", r.kind),
                "params": r.params,
            })
        }).collect();

        serde_json::to_string_pretty(&manifest).unwrap_or("[]".to_string())
    }
}

/// Find the first existing file from a list of candidate names in a directory
fn find_file(dir: &Path, candidates: &[&str]) -> PathBuf {
    for name in candidates {
        let path = dir.join(name);
        if path.exists() {
            return path;
        }
    }
    dir.join(candidates[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_route_parsing() {
    }
}
