// Service Worker generation — built-in SW + manifest for PWA.
//
// Features:
//   - Service worker generation with caching strategies
//   - Web App Manifest generation
//   - Offline support
//   - Cache strategies: cache-first, network-first, stale-while-revalidate
//   - Background sync support

/// Cache strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStrategy {
    CacheFirst,
    NetworkFirst,
    StaleWhileRevalidate,
    NetworkOnly,
    CacheOnly,
}

impl CacheStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CacheFirst => "cache-first",
            Self::NetworkFirst => "network-first",
            Self::StaleWhileRevalidate => "stale-while-revalidate",
            Self::NetworkOnly => "network-only",
            Self::CacheOnly => "cache-only",
        }
    }
}

/// Service worker configuration
#[derive(Debug, Clone)]
pub struct ServiceWorkerConfig {
    /// Cache name prefix
    pub cache_name: String,
    /// Resources to precache
    pub precache: Vec<String>,
    /// Runtime caching rules
    pub runtime_caching: Vec<RuntimeCacheRule>,
    /// Offline fallback page
    pub offline_fallback: Option<String>,
    /// Skip waiting on update
    pub skip_waiting: bool,
    /// Claim clients immediately
    pub clients_claim: bool,
}

impl Default for ServiceWorkerConfig {
    fn default() -> Self {
        Self {
            cache_name: "pledgepack-sw".to_string(),
            precache: vec![],
            runtime_caching: vec![
                RuntimeCacheRule {
                    pattern: r"\.(?:js|ts|jsx|tsx|css|woff2?)$".to_string(),
                    strategy: CacheStrategy::StaleWhileRevalidate,
                },
                RuntimeCacheRule {
                    pattern: r"\.(?:png|jpg|jpeg|gif|webp|avif|svg)$".to_string(),
                    strategy: CacheStrategy::CacheFirst,
                },
            ],
            offline_fallback: Some("/offline.html".to_string()),
            skip_waiting: true,
            clients_claim: true,
        }
    }
}

/// Runtime cache rule
#[derive(Debug, Clone)]
pub struct RuntimeCacheRule {
    pub pattern: String,
    pub strategy: CacheStrategy,
}

/// Web App Manifest
#[derive(Debug, Clone)]
pub struct WebAppManifest {
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub start_url: String,
    pub display: ManifestDisplay,
    pub background_color: String,
    pub theme_color: String,
    pub icons: Vec<ManifestIcon>,
}

impl Default for WebAppManifest {
    fn default() -> Self {
        Self {
            name: "Pledgepack App".to_string(),
            short_name: "App".to_string(),
            description: "Built with Pledgepack".to_string(),
            start_url: "/".to_string(),
            display: ManifestDisplay::Standalone,
            background_color: "#ffffff".to_string(),
            theme_color: "#000000".to_string(),
            icons: vec![
                ManifestIcon { src: "/icons/icon-192.png".to_string(), sizes: "192x192".to_string(), type_: "image/png".to_string(), purpose: "any maskable".to_string() },
                ManifestIcon { src: "/icons/icon-512.png".to_string(), sizes: "512x512".to_string(), type_: "image/png".to_string(), purpose: "any maskable".to_string() },
            ],
        }
    }
}

/// Display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestDisplay {
    Fullscreen,
    Standalone,
    MinimalUI,
    Browser,
}

impl ManifestDisplay {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fullscreen => "fullscreen",
            Self::Standalone => "standalone",
            Self::MinimalUI => "minimal-ui",
            Self::Browser => "browser",
        }
    }
}

/// Manifest icon
#[derive(Debug, Clone)]
pub struct ManifestIcon {
    pub src: String,
    pub sizes: String,
    pub type_: String,
    pub purpose: String,
}

/// Generate a service worker JavaScript file
pub fn generate_service_worker(config: &ServiceWorkerConfig) -> String {
    let precache_array = config
        .precache
        .iter()
        .map(|p| format!("'{}'", p))
        .collect::<Vec<_>>()
        .join(", ");

    let runtime_rules: Vec<String> = config
        .runtime_caching
        .iter()
        .map(|rule| {
            format!(
                r#"  {{ pattern: /{pattern}/, strategy: '{strategy}' }}"#,
                pattern = rule.pattern,
                strategy = rule.strategy.as_str()
            )
        })
        .collect();

    let offline_block = if let Some(ref fallback) = config.offline_fallback {
        format!(
            r#"
    if (event.request.mode === 'navigate') {{
      try {{
        const response = await fetch(event.request);
        return response;
      }} catch (err) {{
        const cache = await caches.open(CACHE_NAME);
        const fallback = await cache.match('{fallback}');
        return fallback || Response.error();
      }}
    }}"#,
            fallback = fallback
        )
    } else {
        String::new()
    };

    format!(
        r#"// Generated by Pledgepack — do not edit manually
const CACHE_NAME = '{cache_name}';
const PRECACHE_URLS = [{precache}];
const RUNTIME_RULES = [
{runtime}
];

self.addEventListener('install', (event) => {{
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(PRECACHE_URLS))
  );{skip_waiting}
}});

self.addEventListener('activate', (event) => {{
  event.waitUntil(
    caches.keys().then((keys) => {{
      return Promise.all(
        keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key))
      );
    }})
  );{clients_claim}
}});

self.addEventListener('fetch', (event) => {{
  const {{ request }} = event;
{offline}

  // Find matching runtime rule
  const rule = RUNTIME_RULES.find((r) => r.pattern.test(request.url));
  if (!rule) return;

  event.respondWith((async () => {{
    const cache = await caches.open(CACHE_NAME);

    switch (rule.strategy) {{
      case 'cache-first': {{
        const cached = await cache.match(request);
        if (cached) return cached;
        const response = await fetch(request);
        cache.put(request, response.clone());
        return response;
      }}
      case 'network-first': {{
        try {{
          const response = await fetch(request);
          cache.put(request, response.clone());
          return response;
        }} catch (err) {{
          const cached = await cache.match(request);
          if (cached) return cached;
          throw err;
        }}
      }}
      case 'stale-while-revalidate': {{
        const cached = await cache.match(request);
        const fetchPromise = fetch(request).then((response) => {{
          cache.put(request, response.clone());
          return response;
        }});
        return cached || fetchPromise;
      }}
      case 'network-only':
        return fetch(request);
      case 'cache-only':
        return cache.match(request);
      default:
        return fetch(request);
    }}
  }})());
}});"#,
        cache_name = config.cache_name,
        precache = precache_array,
        runtime = runtime_rules.join(",\n"),
        skip_waiting = if config.skip_waiting { "\n  self.skipWaiting();" } else { "" },
        clients_claim = if config.clients_claim { "\n  self.clients.claim();" } else { "" },
        offline = offline_block
    )
}

/// Generate a Web App Manifest JSON
pub fn generate_manifest(manifest: &WebAppManifest) -> String {
    let icons: Vec<String> = manifest
        .icons
        .iter()
        .map(|icon| {
            format!(
                r#"    {{
      "src": "{}",
      "sizes": "{}",
      "type": "{}",
      "purpose": "{}"
    }}"#,
                icon.src, icon.sizes, icon.type_, icon.purpose
            )
        })
        .collect();

    format!(
        r#"{{
  "name": "{}",
  "short_name": "{}",
  "description": "{}",
  "start_url": "{}",
  "display": "{}",
  "background_color": "{}",
  "theme_color": "{}",
  "icons": [
{}
  ]
}}"#,
        manifest.name,
        manifest.short_name,
        manifest.description,
        manifest.start_url,
        manifest.display.as_str(),
        manifest.background_color,
        manifest.theme_color,
        icons.join(",\n")
    )
}

/// Generate HTML tags for PWA registration
pub fn generate_pwa_tags(sw_path: &str, manifest_path: &str) -> String {
    let template = r##"<link rel="manifest" href="__MANIFEST__" />
<meta name="theme-color" content="#000000" />
<script>
  if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
      navigator.serviceWorker.register('__SW__').then((reg) => {
        console.log('SW registered:', reg.scope);
      }).catch((err) => {
        console.log('SW registration failed:', err);
      });
    });
  }
</script>"##;
    template
        .replace("__MANIFEST__", manifest_path)
        .replace("__SW__", sw_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_service_worker() {
        let config = ServiceWorkerConfig::default();
        let sw = generate_service_worker(&config);
        assert!(sw.contains("CACHE_NAME"));
        assert!(sw.contains("install"));
        assert!(sw.contains("activate"));
        assert!(sw.contains("fetch"));
        assert!(sw.contains("stale-while-revalidate"));
        assert!(sw.contains("cache-first"));
    }

    #[test]
    fn test_generate_manifest() {
        let manifest = WebAppManifest::default();
        let json = generate_manifest(&manifest);
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"icons\""));
        assert!(json.contains("\"display\""));
        assert!(json.contains("standalone"));
    }

    #[test]
    fn test_generate_pwa_tags() {
        let tags = generate_pwa_tags("/sw.js", "/manifest.json");
        assert!(tags.contains("rel=\"manifest\""));
        assert!(tags.contains("serviceWorker"));
        assert!(tags.contains("/sw.js"));
    }
}
