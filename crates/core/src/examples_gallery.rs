// G12.33: Examples gallery — 50+ example projects across 10 categories
//
// `pledge examples` CLI command browses examples by category.
// `pledge examples <category>` lists examples in a category.
// `pledge examples <name> --create` scaffolds an example project.

use serde::{Deserialize, Serialize};

/// Example project category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExampleCategory {
    React,
    Vue,
    Svelte,
    Solid,
    Fullstack,
    Css,
    Plugins,
    Vanilla,
    Tanstack,
    Performance,
}

impl ExampleCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::React => "react",
            Self::Vue => "vue",
            Self::Svelte => "svelte",
            Self::Solid => "solid",
            Self::Fullstack => "fullstack",
            Self::Css => "css",
            Self::Plugins => "plugins",
            Self::Vanilla => "vanilla",
            Self::Tanstack => "tanstack",
            Self::Performance => "performance",
        }
    }

    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "react" => Some(Self::React),
            "vue" => Some(Self::Vue),
            "svelte" => Some(Self::Svelte),
            "solid" => Some(Self::Solid),
            "fullstack" => Some(Self::Fullstack),
            "css" => Some(Self::Css),
            "plugins" => Some(Self::Plugins),
            "vanilla" => Some(Self::Vanilla),
            "tanstack" => Some(Self::Tanstack),
            "performance" => Some(Self::Performance),
            _ => None,
        }
    }

    pub fn all() -> &'static [ExampleCategory] {
        &[
            Self::React,
            Self::Vue,
            Self::Svelte,
            Self::Solid,
            Self::Fullstack,
            Self::Css,
            Self::Plugins,
            Self::Vanilla,
            Self::Tanstack,
            Self::Performance,
        ]
    }
}

/// An example project in the gallery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleProject {
    /// Unique identifier (e.g., "react-counter")
    pub id: String,
    /// Display name
    pub name: String,
    /// Category
    pub category: ExampleCategory,
    /// Short description
    pub description: String,
    /// Difficulty level: beginner, intermediate, advanced
    pub difficulty: &'static str,
    /// Tags for search
    pub tags: Vec<&'static str>,
    /// Template to use for scaffolding
    pub template: &'static str,
}

/// Get all example projects (50+)
pub fn all_examples() -> Vec<ExampleProject> {
    vec![
        // ─── React (8) ───
        ExampleProject {
            id: "react-counter".into(),
            name: "React Counter".into(),
            category: ExampleCategory::React,
            description: "Simple counter with useState hook".into(),
            difficulty: "beginner",
            tags: vec!["react", "hooks", "state"],
            template: "react",
        },
        ExampleProject {
            id: "react-todo".into(),
            name: "React Todo App".into(),
            category: ExampleCategory::React,
            description: "Todo list with add, complete, delete, filter".into(),
            difficulty: "beginner",
            tags: vec!["react", "hooks", "crud"],
            template: "react",
        },
        ExampleProject {
            id: "react-router".into(),
            name: "React Router".into(),
            category: ExampleCategory::React,
            description: "Multi-page app with client-side routing".into(),
            difficulty: "intermediate",
            tags: vec!["react", "router", "spa"],
            template: "react",
        },
        ExampleProject {
            id: "react-fetch-data".into(),
            name: "React Fetch Data".into(),
            category: ExampleCategory::React,
            description: "Data fetching with useEffect and loading states".into(),
            difficulty: "intermediate",
            tags: vec!["react", "fetch", "api"],
            template: "react",
        },
        ExampleProject {
            id: "react-form-validation".into(),
            name: "React Form Validation".into(),
            category: ExampleCategory::React,
            description: "Form with real-time validation and error messages".into(),
            difficulty: "intermediate",
            tags: vec!["react", "forms", "validation"],
            template: "react",
        },
        ExampleProject {
            id: "react-context-theme".into(),
            name: "React Context Theme".into(),
            category: ExampleCategory::React,
            description: "Dark/light theme toggle using Context API".into(),
            difficulty: "intermediate",
            tags: vec!["react", "context", "theme"],
            template: "react",
        },
        ExampleProject {
            id: "react-use-reducer".into(),
            name: "React useReducer".into(),
            category: ExampleCategory::React,
            description: "Complex state management with useReducer".into(),
            difficulty: "intermediate",
            tags: vec!["react", "reducer", "state"],
            template: "react",
        },
        ExampleProject {
            id: "react-portal-modal".into(),
            name: "React Portal Modal".into(),
            category: ExampleCategory::React,
            description: "Accessible modal dialog using React Portal".into(),
            difficulty: "advanced",
            tags: vec!["react", "portal", "modal"],
            template: "react",
        },
        // ─── Vue (5) ───
        ExampleProject {
            id: "vue-counter".into(),
            name: "Vue Counter".into(),
            category: ExampleCategory::Vue,
            description: "Simple counter with ref()".into(),
            difficulty: "beginner",
            tags: vec!["vue", "composition", "state"],
            template: "vue",
        },
        ExampleProject {
            id: "vue-todo".into(),
            name: "Vue Todo App".into(),
            category: ExampleCategory::Vue,
            description: "Todo list with v-model and computed".into(),
            difficulty: "beginner",
            tags: vec!["vue", "crud", "computed"],
            template: "vue",
        },
        ExampleProject {
            id: "vue-router".into(),
            name: "Vue Router".into(),
            category: ExampleCategory::Vue,
            description: "Multi-page app with vue-router".into(),
            difficulty: "intermediate",
            tags: vec!["vue", "router", "spa"],
            template: "vue",
        },
        ExampleProject {
            id: "vue-pinia-store".into(),
            name: "Vue Pinia Store".into(),
            category: ExampleCategory::Vue,
            description: "State management with Pinia stores".into(),
            difficulty: "intermediate",
            tags: vec!["vue", "pinia", "state"],
            template: "vue",
        },
        ExampleProject {
            id: "vue-transition".into(),
            name: "Vue Transitions".into(),
            category: ExampleCategory::Vue,
            description: "Animated transitions with Transition component".into(),
            difficulty: "intermediate",
            tags: vec!["vue", "animation", "transition"],
            template: "vue",
        },
        // ─── Svelte (4) ───
        ExampleProject {
            id: "svelte-counter".into(),
            name: "Svelte Counter".into(),
            category: ExampleCategory::Svelte,
            description: "Simple counter with reactive declarations".into(),
            difficulty: "beginner",
            tags: vec!["svelte", "reactive", "state"],
            template: "svelte",
        },
        ExampleProject {
            id: "svelte-todo".into(),
            name: "Svelte Todo".into(),
            category: ExampleCategory::Svelte,
            description: "Todo app with stores and transitions".into(),
            difficulty: "beginner",
            tags: vec!["svelte", "stores", "crud"],
            template: "svelte",
        },
        ExampleProject {
            id: "svelte-store".into(),
            name: "Svelte Store".into(),
            category: ExampleCategory::Svelte,
            description: "Writable/readable stores for shared state".into(),
            difficulty: "intermediate",
            tags: vec!["svelte", "store", "state"],
            template: "svelte",
        },
        ExampleProject {
            id: "svelte-motion".into(),
            name: "Svelte Motion".into(),
            category: ExampleCategory::Svelte,
            description: "Spring and tweened animations".into(),
            difficulty: "intermediate",
            tags: vec!["svelte", "animation", "motion"],
            template: "svelte",
        },
        // ─── Solid (4) ───
        ExampleProject {
            id: "solid-counter".into(),
            name: "Solid Counter".into(),
            category: ExampleCategory::Solid,
            description: "Simple counter with createSignal".into(),
            difficulty: "beginner",
            tags: vec!["solid", "signal", "state"],
            template: "solid",
        },
        ExampleProject {
            id: "solid-todo".into(),
            name: "Solid Todo".into(),
            category: ExampleCategory::Solid,
            description: "Todo list with createStore".into(),
            difficulty: "beginner",
            tags: vec!["solid", "store", "crud"],
            template: "solid",
        },
        ExampleProject {
            id: "solid-resource".into(),
            name: "Solid Resource".into(),
            category: ExampleCategory::Solid,
            description: "Async data loading with createResource".into(),
            difficulty: "intermediate",
            tags: vec!["solid", "resource", "async"],
            template: "solid",
        },
        ExampleProject {
            id: "solid-portal".into(),
            name: "Solid Portal".into(),
            category: ExampleCategory::Solid,
            description: "Portal rendering for overlays".into(),
            difficulty: "advanced",
            tags: vec!["solid", "portal", "overlay"],
            template: "solid",
        },
        // ─── Fullstack (5) ───
        ExampleProject {
            id: "fullstack-pledgestack-blog".into(),
            name: "PledgeStack Blog".into(),
            category: ExampleCategory::Fullstack,
            description: "Full-stack blog with SSR and file-based routing".into(),
            difficulty: "advanced",
            tags: vec!["fullstack", "ssr", "blog", "pledgestack"],
            template: "pledgestack",
        },
        ExampleProject {
            id: "fullstack-next-ssr".into(),
            name: "Next.js SSR".into(),
            category: ExampleCategory::Fullstack,
            description: "Server-side rendering with Next.js adapter".into(),
            difficulty: "advanced",
            tags: vec!["fullstack", "ssr", "next"],
            template: "next",
        },
        ExampleProject {
            id: "fullstack-api-routes".into(),
            name: "API Routes".into(),
            category: ExampleCategory::Fullstack,
            description: "Serverless API routes with JSON responses".into(),
            difficulty: "intermediate",
            tags: vec!["fullstack", "api", "serverless"],
            template: "pledgestack",
        },
        ExampleProject {
            id: "fullstack-auth".into(),
            name: "Auth Flow".into(),
            category: ExampleCategory::Fullstack,
            description: "Login/logout with session management".into(),
            difficulty: "advanced",
            tags: vec!["fullstack", "auth", "session"],
            template: "pledgestack",
        },
        ExampleProject {
            id: "fullstack-graphql".into(),
            name: "GraphQL Server".into(),
            category: ExampleCategory::Fullstack,
            description: "GraphQL API with codegen and React hooks".into(),
            difficulty: "advanced",
            tags: vec!["fullstack", "graphql", "api"],
            template: "pledgestack",
        },
        // ─── CSS (5) ───
        ExampleProject {
            id: "css-tailwind".into(),
            name: "Tailwind CSS".into(),
            category: ExampleCategory::Css,
            description: "Tailwind CSS integration with JIT compilation".into(),
            difficulty: "beginner",
            tags: vec!["css", "tailwind", "utility"],
            template: "react",
        },
        ExampleProject {
            id: "css-unocss".into(),
            name: "UnoCSS".into(),
            category: ExampleCategory::Css,
            description: "UnoCSS with atomic CSS classes".into(),
            difficulty: "beginner",
            tags: vec!["css", "unocss", "atomic"],
            template: "react",
        },
        ExampleProject {
            id: "css-modules".into(),
            name: "CSS Modules".into(),
            category: ExampleCategory::Css,
            description: "Scoped CSS with module conventions".into(),
            difficulty: "beginner",
            tags: vec!["css", "modules", "scoped"],
            template: "react",
        },
        ExampleProject {
            id: "css-sass".into(),
            name: "Sass/SCSS".into(),
            category: ExampleCategory::Css,
            description: "Sass preprocessing with variables and mixins".into(),
            difficulty: "intermediate",
            tags: vec!["css", "sass", "scss"],
            template: "react",
        },
        ExampleProject {
            id: "css-postcss".into(),
            name: "PostCSS".into(),
            category: ExampleCategory::Css,
            description: "PostCSS pipeline with autoprefixer".into(),
            difficulty: "intermediate",
            tags: vec!["css", "postcss", "autoprefixer"],
            template: "react",
        },
        // ─── Plugins (5) ───
        ExampleProject {
            id: "plugin-wasm-minify".into(),
            name: "WASM Minify Plugin".into(),
            category: ExampleCategory::Plugins,
            description: "WASM-powered JS minification plugin".into(),
            difficulty: "advanced",
            tags: vec!["plugin", "wasm", "minify"],
            template: "vanilla",
        },
        ExampleProject {
            id: "plugin-js-resolve".into(),
            name: "JS Resolve Plugin".into(),
            category: ExampleCategory::Plugins,
            description: "Custom module resolution plugin".into(),
            difficulty: "intermediate",
            tags: vec!["plugin", "resolve", "alias"],
            template: "vanilla",
        },
        ExampleProject {
            id: "plugin-env-replace".into(),
            name: "Env Replace Plugin".into(),
            category: ExampleCategory::Plugins,
            description: "Replace environment variables in code".into(),
            difficulty: "beginner",
            tags: vec!["plugin", "env", "replace"],
            template: "vanilla",
        },
        ExampleProject {
            id: "plugin-image-optimize".into(),
            name: "Image Optimize Plugin".into(),
            category: ExampleCategory::Plugins,
            description: "Automatic image optimization and WebP conversion".into(),
            difficulty: "intermediate",
            tags: vec!["plugin", "image", "webp"],
            template: "vanilla",
        },
        ExampleProject {
            id: "plugin-i18n".into(),
            name: "i18n Plugin".into(),
            category: ExampleCategory::Plugins,
            description: "Internationalization with translation extraction".into(),
            difficulty: "advanced",
            tags: vec!["plugin", "i18n", "translation"],
            template: "vanilla",
        },
        // ─── Vanilla (6) ───
        ExampleProject {
            id: "vanilla-ts-starter".into(),
            name: "TypeScript Starter".into(),
            category: ExampleCategory::Vanilla,
            description: "Minimal TypeScript project setup".into(),
            difficulty: "beginner",
            tags: vec!["vanilla", "typescript", "starter"],
            template: "vanilla",
        },
        ExampleProject {
            id: "vanilla-canvas-animation".into(),
            name: "Canvas Animation".into(),
            category: ExampleCategory::Vanilla,
            description: "HTML5 Canvas particle animation".into(),
            difficulty: "intermediate",
            tags: vec!["vanilla", "canvas", "animation"],
            template: "vanilla",
        },
        ExampleProject {
            id: "vanilla-web-components".into(),
            name: "Web Components".into(),
            category: ExampleCategory::Vanilla,
            description: "Custom elements with Shadow DOM".into(),
            difficulty: "intermediate",
            tags: vec!["vanilla", "web-components", "shadow-dom"],
            template: "vanilla",
        },
        ExampleProject {
            id: "vanilla-webgl".into(),
            name: "WebGL Demo".into(),
            category: ExampleCategory::Vanilla,
            description: "WebGL rendering with shaders".into(),
            difficulty: "advanced",
            tags: vec!["vanilla", "webgl", "shader"],
            template: "vanilla",
        },
        ExampleProject {
            id: "vanilla-web-worker".into(),
            name: "Web Worker".into(),
            category: ExampleCategory::Vanilla,
            description: "Offload computation to a web worker".into(),
            difficulty: "intermediate",
            tags: vec!["vanilla", "worker", "threads"],
            template: "vanilla",
        },
        ExampleProject {
            id: "vanilla-service-worker".into(),
            name: "Service Worker PWA".into(),
            category: ExampleCategory::Vanilla,
            description: "Offline-first PWA with service worker".into(),
            difficulty: "advanced",
            tags: vec!["vanilla", "pwa", "service-worker"],
            template: "vanilla",
        },
        // ─── TanStack (4) ───
        ExampleProject {
            id: "tanstack-router-basic".into(),
            name: "TanStack Router Basic".into(),
            category: ExampleCategory::Tanstack,
            description: "File-based routing with TanStack Router".into(),
            difficulty: "intermediate",
            tags: vec!["tanstack", "router", "file-based"],
            template: "tanstack",
        },
        ExampleProject {
            id: "tanstack-query".into(),
            name: "TanStack Query".into(),
            category: ExampleCategory::Tanstack,
            description: "Server state management with TanStack Query".into(),
            difficulty: "intermediate",
            tags: vec!["tanstack", "query", "data"],
            template: "tanstack",
        },
        ExampleProject {
            id: "tanstack-form".into(),
            name: "TanStack Form".into(),
            category: ExampleCategory::Tanstack,
            description: "Type-safe form management with TanStack Form".into(),
            difficulty: "intermediate",
            tags: vec!["tanstack", "form", "validation"],
            template: "tanstack",
        },
        ExampleProject {
            id: "tanstack-table".into(),
            name: "TanStack Table".into(),
            category: ExampleCategory::Tanstack,
            description: "Headless table with sorting and filtering".into(),
            difficulty: "advanced",
            tags: vec!["tanstack", "table", "sort"],
            template: "tanstack",
        },
        // ─── Performance (5) ───
        ExampleProject {
            id: "perf-code-splitting".into(),
            name: "Code Splitting".into(),
            category: ExampleCategory::Performance,
            description: "Dynamic imports and route-level code splitting".into(),
            difficulty: "intermediate",
            tags: vec!["performance", "splitting", "lazy"],
            template: "react",
        },
        ExampleProject {
            id: "perf-tree-shaking".into(),
            name: "Tree Shaking".into(),
            category: ExampleCategory::Performance,
            description: "Dead code elimination demonstration".into(),
            difficulty: "intermediate",
            tags: vec!["performance", "tree-shaking", "dead-code"],
            template: "vanilla",
        },
        ExampleProject {
            id: "perf-bundle-analysis".into(),
            name: "Bundle Analysis".into(),
            category: ExampleCategory::Performance,
            description: "Bundle size analysis and budget enforcement".into(),
            difficulty: "intermediate",
            tags: vec!["performance", "bundle", "budget"],
            template: "react",
        },
        ExampleProject {
            id: "perf-lazy-load".into(),
            name: "Lazy Loading".into(),
            category: ExampleCategory::Performance,
            description: "Lazy load images and components with IntersectionObserver".into(),
            difficulty: "intermediate",
            tags: vec!["performance", "lazy", "intersection"],
            template: "react",
        },
        ExampleProject {
            id: "perf-preload-prefetch".into(),
            name: "Preload & Prefetch".into(),
            category: ExampleCategory::Performance,
            description: "Resource hints for faster navigation".into(),
            difficulty: "advanced",
            tags: vec!["performance", "preload", "prefetch"],
            template: "react",
        },
    ]
}

/// Get examples filtered by category
pub fn examples_by_category(category: &ExampleCategory) -> Vec<ExampleProject> {
    all_examples()
        .into_iter()
        .filter(|e| &e.category == category)
        .collect()
}

/// Find an example by ID
pub fn find_example(id: &str) -> Option<ExampleProject> {
    all_examples().into_iter().find(|e| e.id == id)
}

/// Search examples by query string (matches name, description, tags)
pub fn search_examples(query: &str) -> Vec<ExampleProject> {
    let q = query.to_lowercase();
    all_examples()
        .into_iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&q)
                || e.description.to_lowercase().contains(&q)
                || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
                || e.id.contains(&q)
        })
        .collect()
}

/// Get example count per category
pub fn category_counts() -> Vec<(ExampleCategory, usize)> {
    ExampleCategory::all()
        .iter()
        .map(|cat| {
            let count = all_examples().iter().filter(|e| &e.category == cat).count();
            (cat.clone(), count)
        })
        .collect()
}

/// Format the examples gallery for terminal output
pub fn format_gallery() -> String {
    let examples = all_examples();
    let counts = category_counts();
    let total = examples.len();

    let mut out = String::new();
    out.push_str(&format!(
        "  \x1b[1mPledgePack Examples Gallery\x1b[0m ({} examples)\n",
        total
    ));
    out.push_str("  ───────────────────────────────────────\n\n");

    for (cat, count) in &counts {
        out.push_str(&format!("  \x1b[35m{}\x1b[0m ({})\n", cat.label(), count));
        for ex in examples.iter().filter(|e| &e.category == cat) {
            out.push_str(&format!(
                "    \x1b[36m{}\x1b[0m — {}\n      \x1b[90m[{}] tags: {}\x1b[0m\n",
                ex.id,
                ex.description,
                ex.difficulty,
                ex.tags.join(", ")
            ));
        }
        out.push('\n');
    }

    out.push_str("  \x1b[90mUsage: pledge examples <category> — list by category\x1b[0m\n");
    out.push_str("  \x1b[90m       pledge examples <id> --create — scaffold an example\x1b[0m\n");

    out
}

/// Format examples in a specific category
pub fn format_category(category: &ExampleCategory) -> String {
    let examples = examples_by_category(category);

    let mut out = String::new();
    out.push_str(&format!(
        "  \x1b[1m{} Examples\x1b[0m ({})\n",
        category.label(),
        examples.len()
    ));
    out.push_str("  ───────────────────────────────────────\n\n");

    for ex in &examples {
        out.push_str(&format!(
            "  \x1b[36m{}\x1b[0m — {}\n    \x1b[90m[{}] {}\x1b[0m\n\n",
            ex.id,
            ex.description,
            ex.difficulty,
            ex.tags.join(", ")
        ));
    }

    out.push_str(&format!(
        "  \x1b[90mScaffold: pledge examples {} <id> --create\x1b[0m\n",
        category.label()
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_examples_count_50_plus() {
        let examples = all_examples();
        assert!(
            examples.len() >= 50,
            "Expected 50+ examples, got {}",
            examples.len()
        );
    }

    #[test]
    fn test_unique_ids() {
        let examples = all_examples();
        let mut ids: Vec<&str> = examples.iter().map(|e| e.id.as_str()).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "Duplicate example IDs found");
    }

    #[test]
    fn test_category_counts() {
        let counts = category_counts();
        assert_eq!(counts.len(), 10);
        let total: usize = counts.iter().map(|(_, c)| c).sum();
        assert_eq!(total, all_examples().len());
    }

    #[test]
    fn test_react_has_8_examples() {
        let react = examples_by_category(&ExampleCategory::React);
        assert_eq!(react.len(), 8);
    }

    #[test]
    fn test_vue_has_5_examples() {
        let vue = examples_by_category(&ExampleCategory::Vue);
        assert_eq!(vue.len(), 5);
    }

    #[test]
    fn test_svelte_has_4_examples() {
        let svelte = examples_by_category(&ExampleCategory::Svelte);
        assert_eq!(svelte.len(), 4);
    }

    #[test]
    fn test_solid_has_4_examples() {
        let solid = examples_by_category(&ExampleCategory::Solid);
        assert_eq!(solid.len(), 4);
    }

    #[test]
    fn test_fullstack_has_5_examples() {
        let fs = examples_by_category(&ExampleCategory::Fullstack);
        assert_eq!(fs.len(), 5);
    }

    #[test]
    fn test_css_has_5_examples() {
        let css = examples_by_category(&ExampleCategory::Css);
        assert_eq!(css.len(), 5);
    }

    #[test]
    fn test_plugins_has_5_examples() {
        let plugins = examples_by_category(&ExampleCategory::Plugins);
        assert_eq!(plugins.len(), 5);
    }

    #[test]
    fn test_vanilla_has_6_examples() {
        let vanilla = examples_by_category(&ExampleCategory::Vanilla);
        assert_eq!(vanilla.len(), 6);
    }

    #[test]
    fn test_tanstack_has_4_examples() {
        let ts = examples_by_category(&ExampleCategory::Tanstack);
        assert_eq!(ts.len(), 4);
    }

    #[test]
    fn test_performance_has_5_examples() {
        let perf = examples_by_category(&ExampleCategory::Performance);
        assert_eq!(perf.len(), 5);
    }

    #[test]
    fn test_find_example() {
        let ex = find_example("react-counter").unwrap();
        assert_eq!(ex.name, "React Counter");
        assert_eq!(ex.category, ExampleCategory::React);
    }

    #[test]
    fn test_find_example_not_found() {
        assert!(find_example("nonexistent").is_none());
    }

    #[test]
    fn test_search_examples() {
        let results = search_examples("todo");
        assert!(results.len() >= 3); // react-todo, vue-todo, svelte-todo
        assert!(results.iter().any(|e| e.id == "react-todo"));
        assert!(results.iter().any(|e| e.id == "vue-todo"));
        assert!(results.iter().any(|e| e.id == "svelte-todo"));
    }

    #[test]
    fn test_search_by_tag() {
        let results = search_examples("wasm");
        assert!(results.iter().any(|e| e.id == "plugin-wasm-minify"));
    }

    #[test]
    fn test_format_gallery() {
        let gallery = format_gallery();
        assert!(gallery.contains("PledgePack Examples Gallery"));
        assert!(gallery.contains("react"));
        assert!(gallery.contains("vue"));
        assert!(gallery.contains("performance"));
    }

    #[test]
    fn test_format_category() {
        let output = format_category(&ExampleCategory::React);
        assert!(output.contains("react Examples"));
        assert!(output.contains("react-counter"));
        assert!(output.contains("react-todo"));
    }

    #[test]
    fn test_category_from_label() {
        assert_eq!(
            ExampleCategory::from_label("react"),
            Some(ExampleCategory::React)
        );
        assert_eq!(
            ExampleCategory::from_label("performance"),
            Some(ExampleCategory::Performance)
        );
        assert_eq!(ExampleCategory::from_label("nonexistent"), None);
    }
}
