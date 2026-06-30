use crate::swap::types::ZswapConfig;
use std::fs;

use crate::config;
use super::exec;

/// Read the current zswap configuration from sysfs.
pub fn read_zswap_config() -> Result<ZswapConfig, String> {
    let enabled = read_sysfs_bool(config::ZSWAP_ENABLED)?;
    let compressor = read_sysfs_string(config::ZSWAP_COMPRESSOR)?;
    let max_pool_percent = read_sysfs_u8(config::ZSWAP_MAX_POOL_PERCENT)?;
    let accept_threshold_percent = read_sysfs_u8(config::ZSWAP_ACCEPT_THRESHOLD)?;
    let shrinker_enabled = read_sysfs_bool(config::ZSWAP_SHRINKER)?;

    Ok(ZswapConfig {
        enabled,
        compressor,
        max_pool_percent,
        accept_threshold_percent,
        shrinker_enabled,
    })
}

/// Check which compression algorithms are available in the running kernel.
/// Reads /proc/crypto for loaded crypto modules.
pub fn get_available_compressors() -> Vec<String> {
    let content = match fs::read_to_string(config::PROC_CRYPTO) {
        Ok(c) => c,
        Err(_) => return vec!["lzo".to_string()], // fallback: lzo is always available
    };

    let mut compressors = Vec::new();

    // zswap-supported algorithms and their /proc/crypto names
    let candidates = [
        ("lzo", "lzo"),
        ("lz4", "lz4"),
        ("lz4hc", "lz4hc"),
        ("zstd", "zstd"),
        ("deflate", "deflate"),
        ("842", "842"),
    ];

    for (algo, crypto_name) in &candidates {
        if content.contains(&format!("name         : {crypto_name}"))
            || content.contains(&format!("driver       : {crypto_name}-"))
        {
            if !compressors.contains(&algo.to_string()) {
                compressors.push(algo.to_string());
            }
        }
    }

    if compressors.is_empty() {
        compressors.push("lzo".to_string());
    }

    compressors.sort();
    compressors.dedup();
    compressors
}

// ─── Internal sysfs helpers ──────────────────────────────────────────────────

fn read_sysfs_string(path: &str) -> Result<String, String> {
    fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("Cannot read {path}: {e}"))
}

fn read_sysfs_bool(path: &str) -> Result<bool, String> {
    let val = read_sysfs_string(path)?;
    Ok(val == "1" || val.eq_ignore_ascii_case("Y") || val.eq_ignore_ascii_case("true"))
}

fn read_sysfs_u8(path: &str) -> Result<u8, String> {
    let val = read_sysfs_string(path)?;
    val.parse::<u8>()
        .map_err(|e| format!("Cannot parse {path} as u8: {e}"))
}

// ─── Write operations (require pkexec) ─────────────────────────────────────

/// Enable zswap via pkexec tee.
pub fn enable_zswap() -> Result<String, String> {
    exec::write_sysfs(config::ZSWAP_ENABLED, "1")
}

/// Disable zswap via pkexec tee.
pub fn disable_zswap() -> Result<String, String> {
    exec::write_sysfs(config::ZSWAP_ENABLED, "0")
}

/// Set the zswap compressor algorithm.
/// Checks /proc/crypto first to warn if the module needs loading.
pub fn set_compressor(algo: &str) -> Result<String, String> {
    let available = get_available_compressors();
    if !available.iter().any(|a| a == algo) {
        // Try loading the compression module first
        let _ = exec::run_modprobe(algo);
        // Re-check
        let available2 = get_available_compressors();
        if !available2.iter().any(|a| a == algo) {
            return Err(format!(
                "Compressor '{}' is not available. Available: {}.",
                algo, available2.join(", ")
            ));
        }
    }
    exec::write_sysfs(config::ZSWAP_COMPRESSOR, algo)
}

/// Set zswap max pool percent (of total RAM).
pub fn set_max_pool_percent(pct: u8) -> Result<String, String> {
    exec::write_sysfs(config::ZSWAP_MAX_POOL_PERCENT, &pct.to_string())
}

/// Set zswap accept threshold percent (compression ratio threshold).
pub fn set_accept_threshold(pct: u8) -> Result<String, String> {
    exec::write_sysfs(config::ZSWAP_ACCEPT_THRESHOLD, &pct.to_string())
}

/// Enable/disable the zswap shrinker (reclaims pool under memory pressure).
pub fn set_shrinker(enabled: bool) -> Result<String, String> {
    let val = if enabled { "Y" } else { "N" };
    exec::write_sysfs(config::ZSWAP_SHRINKER, val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_zswap_config() {
        let result = read_zswap_config();
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_available_compressors() {
        let compressors = get_available_compressors();
        assert!(!compressors.is_empty());
        assert!(compressors.contains(&"lzo".to_string()));
    }
}
