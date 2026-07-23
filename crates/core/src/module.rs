use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique identifier for a module in the graph
pub type ModuleId = u32;

/// Type of a resolved module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleKind {
    JavaScript,
    TypeScript,
    Jsx,
    Tsx,
    Css,
    Json,
    Asset,
    Wasm,
    Vue,
    Svelte,
    Astro,
    Worker,
    SharedWorker,
    WebComponent,
    Mdx,
    Graphql,
    Yaml,
    Csv,
    Tsv,
    Sass,
    Toml,
    Shader,
    Unknown,
}

impl ModuleKind {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            ".tsx" => Self::Tsx,
            ".ts" => Self::TypeScript,
            ".jsx" => Self::Jsx,
            ".js" | ".mjs" | ".cjs" => Self::JavaScript,
            ".css" => Self::Css,
            ".json" => Self::Json,
            ".wasm" => Self::Wasm,
            ".vue" => Self::Vue,
            ".svelte" => Self::Svelte,
            ".astro" => Self::Astro,
            ".worker.js" | ".worker.ts" => Self::Worker,
            ".wc.tsx" | ".wc.jsx" => Self::WebComponent,
            ".mdx" => Self::Mdx,
            ".graphql" | ".gql" => Self::Graphql,
            ".yaml" | ".yml" => Self::Yaml,
            ".csv" => Self::Csv,
            ".tsv" => Self::Tsv,
            ".scss" | ".sass" => Self::Sass,
            ".toml" => Self::Toml,
            ".glsl" | ".frag" | ".vert" | ".comp" | ".wgsl" => Self::Shader,
            ".png" | ".jpg" | ".jpeg" | ".gif" | ".svg" | ".webp" | ".ico" |
            ".woff" | ".woff2" | ".ttf" | ".otf" | ".eot" |
            ".mp4" | ".webm" | ".mp3" | ".wav" | ".pdf" => Self::Asset,
            _ => Self::Unknown,
        }
    }

    pub fn is_typescript(&self) -> bool {
        matches!(self, Self::TypeScript | Self::Tsx)
    }

    pub fn is_jsx(&self) -> bool {
        matches!(self, Self::Jsx | Self::Tsx)
    }
}

/// A fully resolved module — ready to be parsed and transformed
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub id: ModuleId,
    pub path: PathBuf,
    pub kind: ModuleKind,
    /// Raw source content (read from filesystem)
    pub source: Vec<u8>,
    /// Content hash for cache invalidation
    pub content_hash: u64,
}

impl ResolvedModule {
    pub fn extension(&self) -> String {
        self.path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default()
    }
}
