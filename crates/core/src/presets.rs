// Framework presets — configuration templates for Remix, Astro, Qwik, SvelteKit, Nuxt, Angular.
//
// Each preset provides:
//   - Default config values (entry, framework, plugins, aliases)
//   - Required dependencies
//   - HTML entry template
//   - tsconfig.json template

use crate::config::{Framework, PledgeConfig, BuildMode, OutputFormat};

/// A framework preset
#[derive(Debug, Clone)]
pub struct FrameworkPreset {
    pub name: &'static str,
    pub display_name: &'static str,
    pub framework: Framework,
    pub entry: &'static str,
    pub dependencies: &'static [&'static str],
    pub dev_dependencies: &'static [&'static str],
    pub aliases: &'static [(&'static str, &'static str)],
    pub html_template: &'static str,
    pub tsconfig: &'static str,
    pub description: &'static str,
}

/// All available framework presets
pub static PRESETS: &[FrameworkPreset] = &[
    FrameworkPreset {
        name: "remix",
        display_name: "Remix",
        framework: Framework::React,
        entry: "src/entry.client.tsx",
        dependencies: &["@remix-run/react", "@remix-run/node", "react", "react-dom"],
        dev_dependencies: &["@types/react", "@types/react-dom", "typescript"],
        aliases: &[("~", "src")],
        html_template: r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Remix App</title>
</head>
<body>
    <div id="root"></div>
    <script type="module" src="/src/entry.client.tsx"></script>
</body>
</html>"#,
        tsconfig: r#"{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["DOM", "DOM.Iterable", "ES2022"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "baseUrl": ".",
    "paths": { "~/*": ["src/*"] }
  }
}"#,
        description: "Remix — full-stack web framework with nested routing",
    },
    FrameworkPreset {
        name: "astro",
        display_name: "Astro",
        framework: Framework::Auto,
        entry: "src/pages/index.astro",
        dependencies: &["astro"],
        dev_dependencies: &[],
        aliases: &[],
        html_template: r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Astro Site</title>
</head>
<body>
    <div id="root"></div>
</body>
</html>"#,
        tsconfig: r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true
  }
}"#,
        description: "Astro — content-driven sites with island architecture",
    },
    FrameworkPreset {
        name: "qwik",
        display_name: "Qwik",
        framework: Framework::React,
        entry: "src/entry.tsx",
        dependencies: &["@builder.io/qwik", "@builder.io/qwik-react"],
        dev_dependencies: &["typescript"],
        aliases: &[("@", "src")],
        html_template: r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Qwik App</title>
</head>
<body>
    <div id="root"></div>
    <script type="module" src="/src/entry.tsx"></script>
</body>
</html>"#,
        tsconfig: r#"{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["DOM", "DOM.Iterable", "ES2022"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "baseUrl": ".",
    "paths": { "@/*": ["src/*"] }
  }
}"#,
        description: "Qwik — resumable framework for instant loading",
    },
    FrameworkPreset {
        name: "sveltekit",
        display_name: "SvelteKit",
        framework: Framework::Svelte,
        entry: "src/app.html",
        dependencies: &["@sveltejs/kit", "svelte"],
        dev_dependencies: &["typescript", "svelte-check"],
        aliases: &[("$lib", "src/lib")],
        html_template: r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>SvelteKit App</title>
    %sveltekit.head%
</head>
<body data-sveltekit-preload-data="hover">
    <div style="display: contents">%sveltekit.body%</div>
</body>
</html>"#,
        tsconfig: r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "baseUrl": ".",
    "paths": { "$lib": ["src/lib"], "$lib/*": ["src/lib/*"] }
  }
}"#,
        description: "SvelteKit — application framework for Svelte",
    },
    FrameworkPreset {
        name: "nuxt",
        display_name: "Nuxt",
        framework: Framework::Vue,
        entry: "src/app.ts",
        dependencies: &["nuxt", "vue"],
        dev_dependencies: &["typescript"],
        aliases: &[("~", "src"), ("~~", "src")],
        html_template: r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Nuxt App</title>
</head>
<body>
    <div id="__nuxt"></div>
    <script type="module" src="/src/app.ts"></script>
</body>
</html>"#,
        tsconfig: r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "baseUrl": ".",
    "paths": { "~/*": ["src/*"], "~~/*": ["src/*"] }
  }
}"#,
        description: "Nuxt — intuitive Vue framework with SSR",
    },
    FrameworkPreset {
        name: "angular",
        display_name: "Angular",
        framework: Framework::Auto,
        entry: "src/main.ts",
        dependencies: &["@angular/core", "@angular/platform-browser", "@angular/platform-browser-dynamic", "rxjs", "zone.js"],
        dev_dependencies: &["@angular/cli", "typescript"],
        aliases: &[],
        html_template: r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Angular App</title>
</head>
<body>
    <app-root></app-root>
    <script type="module" src="/src/main.ts"></script>
</body>
</html>"#,
        tsconfig: r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "experimentalDecorators": true,
    "strict": true
  }
}"#,
        description: "Angular — platform for building mobile and desktop apps",
    },
];

/// Get a preset by name
pub fn get_preset(name: &str) -> Option<&'static FrameworkPreset> {
    PRESETS.iter().find(|p| p.name == name)
}

/// List all available preset names
pub fn list_presets() -> Vec<&'static str> {
    PRESETS.iter().map(|p| p.name).collect()
}

/// Generate a PledgeConfig from a preset
pub fn config_from_preset(preset: &FrameworkPreset) -> PledgeConfig {
    let mut config = PledgeConfig::default();
    config.framework = preset.framework;
    config.entry = vec![preset.entry.to_string()];
    config.mode = BuildMode::Development;
    config.output_format = OutputFormat::Esm;
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_preset() {
        assert!(get_preset("remix").is_some());
        assert!(get_preset("astro").is_some());
        assert!(get_preset("qwik").is_some());
        assert!(get_preset("sveltekit").is_some());
        assert!(get_preset("nuxt").is_some());
        assert!(get_preset("angular").is_some());
        assert!(get_preset("nonexistent").is_none());
    }

    #[test]
    fn test_list_presets() {
        let names = list_presets();
        assert_eq!(names.len(), 6);
        assert!(names.contains(&"remix"));
    }
}
