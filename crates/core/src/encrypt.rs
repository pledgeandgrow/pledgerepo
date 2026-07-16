// Build-time string encryption (#109)
//
// Encrypts sensitive strings in source at build time, decrypts at runtime
// via injected shim. Prevents plain-text secrets in bundles.

use crate::config::EncryptConfig;
use anyhow::Result;
use tracing::info;

/// Simple XOR-based encryption (lightweight, obfuscation-level)
/// For production use, consider AES-GCM via WASM.
fn xor_encrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % key.len()])
        .collect()
}

/// Generate a 32-byte encryption key from config or randomly
fn get_or_create_key(config: &EncryptConfig) -> Vec<u8> {
    if let Some(ref key_hex) = config.key {
        // Parse hex string to bytes
        if key_hex.len() == 64 {
            (0..key_hex.len())
                .step_by(2)
                .filter_map(|i| u8::from_str_radix(&key_hex[i..i + 2], 16).ok())
                .collect()
        } else {
            // Use the string bytes directly, padded/truncated to 32
            let bytes = key_hex.as_bytes();
            let mut key = vec![0u8; 32];
            for (i, &b) in bytes.iter().enumerate().take(32) {
                key[i] = b;
            }
            key
        }
    } else {
        // Generate a deterministic key from build timestamp + keys
        let seed = format!(
            "{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            config.keys.join(","),
        );
        let seed_bytes = seed.as_bytes();
        let mut key = vec![0u8; 32];
        for (i, &b) in seed_bytes.iter().enumerate().take(32) {
            key[i] = b;
        }
        key
    }
}

/// Encrypt a single string value
pub fn encrypt_value(value: &str, key: &[u8]) -> String {
    let encrypted = xor_encrypt(value.as_bytes(), key);
    // Base64 encode for safe embedding
    base64_encode(&encrypted)
}

/// Decrypt a single string value (used in the runtime shim)
pub fn decrypt_value(encrypted: &str, key: &[u8]) -> String {
    let decoded = base64_decode(encrypted);
    let decrypted = xor_encrypt(&decoded, key);
    String::from_utf8_lossy(&decrypted).to_string()
}

/// Transform source code: encrypt sensitive string literals
/// Replaces string literals matching configured keys with encrypted versions
pub fn encrypt_strings(code: &str, config: &EncryptConfig) -> Result<(String, Vec<u8>)> {
    if !config.enabled || config.keys.is_empty() {
        return Ok((code.to_string(), Vec::new()));
    }

    let key = get_or_create_key(config);
    let mut result = code.to_string();

    // For each configured key, find its value in process.env or define
    // and replace occurrences in the code with encrypted versions
    for key_name in &config.keys {
        // Look up the value from process.env
        if let Ok(value) = std::env::var(key_name) {
            if result.contains(&value) {
                let encrypted = encrypt_value(&value, &key);
                // Replace the plain-text value with a decryption call
                let replacement = format!(
                    "__pledge_decrypt(\"{}\")",
                    encrypted,
                );
                result = result.replace(&value, &replacement);
            }
        }
    }

    // Inject the decryption shim at the top of the code
    let key_b64 = base64_encode(&key);
    let shim = format!(
        r#"// Build-time string encryption shim (#109)
const __pledge_key = __pledge_b64dec("{}");
function __pledge_b64dec(s) {{
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let bytes = [];
  for (let i = 0; i < s.length; i += 4) {{
    let n = (chars.indexOf(s[i]) << 18) | (chars.indexOf(s[i+1]) << 12);
    if (s[i+2] !== '=') n |= (chars.indexOf(s[i+2]) << 6);
    if (s[i+3] !== '=') n |= chars.indexOf(s[i+3]);
    bytes.push((n >> 16) & 0xff);
    if (s[i+2] !== '=') bytes.push((n >> 8) & 0xff);
    if (s[i+3] !== '=') bytes.push(n & 0xff);
  }}
  return new Uint8Array(bytes);
}}
function __pledge_decrypt(enc) {{
  const decoded = __pledge_b64dec(enc);
  const result = new Uint8Array(decoded.length);
  for (let i = 0; i < decoded.length; i++) {{
    result[i] = decoded[i] ^ __pledge_key[i % __pledge_key.length];
  }}
  return new TextDecoder().decode(result);
}}

"#,
        key_b64,
    );

    result = format!("{}\n{}", shim, result);

    info!("String encryption: {} keys encrypted", config.keys.len());
    Ok((result, key))
}

/// Simple base64 encoder
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

        let n = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Simple base64 decoder
fn base64_decode(s: &str) -> Vec<u8> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let s = s.as_bytes();

    for chunk in s.chunks(4) {
        let mut n = 0u32;
        let mut pad = 0;

        for (i, &c) in chunk.iter().enumerate() {
            if c == b'=' {
                pad += 1;
            } else {
                let idx = CHARS.iter().position(|&x| x == c).unwrap_or(0);
                n |= (idx as u32) << (18 - i * 6);
            }
        }

        result.push(((n >> 16) & 0xff) as u8);
        if pad < 2 {
            result.push(((n >> 8) & 0xff) as u8);
        }
        if pad < 1 {
            result.push((n & 0xff) as u8);
        }
    }

    result
}
