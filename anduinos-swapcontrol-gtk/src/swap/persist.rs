//! Persistence layer: generates systemd units and config files so that
//! zswap/zram/sysctl settings survive reboot.
//!
//! Strategy:
//!   - sysctl     → /etc/sysctl.d/90-anduinos-swapcontrol.conf (already done by sysctl.rs)
//!   - zswap      → systemd oneshot service that writes sysfs on boot
//!   - zram       → systemd service that creates zram device on boot
//!
//! All files are written via the helper (single polkit auth).

use super::exec;
use crate::config;

const ZSWAP_SERVICE: &str = "/etc/systemd/system/anduinos-zswap.service";
const ZRAM_SERVICE: &str = "/etc/systemd/system/anduinos-zram.service";

// ─── Zswap persistence ──────────────────────────────────────────────────────

/// Generate and install a systemd oneshot service that applies zswap settings at boot.
pub fn persist_zswap(enabled: bool, compressor: &str, max_pool_pct: u8, accept_threshold: u8, shrinker: bool) -> Result<String, String> {
    let enabled_val = if enabled { "1" } else { "0" };
    let shrinker_val = if shrinker { "Y" } else { "N" };

    let unit = format!(
        "# Managed by anduinos-swapcontrol-gtk. Do not edit manually.\n\
         [Unit]\n\
         Description=AnduinOS Zswap Configuration\n\
         Before=swap.target\n\n\
         [Service]\n\
         Type=oneshot\n\
         RemainAfterExit=yes\n\
         ExecStart=/usr/lib/anduinos-swapcontrol/helper tee {zswap_enabled} <<< \"{enabled_val}\"\n\
         ExecStart=/usr/lib/anduinos-swapcontrol/helper tee {zswap_compressor} <<< \"{compressor}\"\n\
         ExecStart=/usr/lib/anduinos-swapcontrol/helper tee {zswap_pool} <<< \"{max_pool_pct}\"\n\
         ExecStart=/usr/lib/anduinos-swapcontrol/helper tee {zswap_threshold} <<< \"{accept_threshold}\"\n\
         ExecStart=/usr/lib/anduinos-swapcontrol/helper tee {zswap_shrinker} <<< \"{shrinker_val}\"\n\n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        zswap_enabled = config::ZSWAP_ENABLED,
        zswap_compressor = config::ZSWAP_COMPRESSOR,
        zswap_pool = config::ZSWAP_MAX_POOL_PERCENT,
        zswap_threshold = config::ZSWAP_ACCEPT_THRESHOLD,
        zswap_shrinker = config::ZSWAP_SHRINKER,
        enabled_val = enabled_val,
        compressor = compressor,
        max_pool_pct = max_pool_pct,
        accept_threshold = accept_threshold,
        shrinker_val = shrinker_val,
    );

    if !enabled {
        // If zswap is disabled, just remove the service so it won't start on boot
        let _ = exec::run_helper("rm", &["-f", ZSWAP_SERVICE]);
        let _ = exec::run_helper("systemctl", &["disable", "anduinos-zswap.service"]);
        return Ok("Zswap persistence disabled".to_string());
    }

    // Write the unit file
    exec::write_sysfs(ZSWAP_SERVICE, &unit)?;

    // Enable the service (ignore errors if systemctl isn't available)
    let _ = exec::run_helper("systemctl", &["daemon-reload"]);
    let _ = exec::run_helper("systemctl", &["enable", "anduinos-zswap.service"]);

    Ok("Zswap persistence enabled".to_string())
}

/// Remove zswap persistence (called when disabling zswap).
pub fn remove_zswap_persistence() -> Result<String, String> {
    let _ = exec::run_helper("systemctl", &["disable", "--now", "anduinos-zswap.service"]);
    let _ = exec::run_helper("rm", &["-f", ZSWAP_SERVICE]);
    let _ = exec::run_helper("systemctl", &["daemon-reload"]);
    Ok("Zswap persistence removed".to_string())
}

// ─── Zram persistence ───────────────────────────────────────────────────────

/// Generate and install a systemd service that creates a zram device at boot.
/// If `devices` is empty, removes any existing zram persistence.
pub fn persist_zram(devices: &[(u64, String, i32)]) -> Result<String, String> {
    // devices: Vec<(size_mb, algorithm, priority)>

    if devices.is_empty() {
        // Remove persistence
        let _ = exec::run_helper("swapoff", &["/dev/zram0"]);
        let _ = exec::run_helper("zramctl", &["-r", "/dev/zram0"]);
        let _ = exec::run_helper("systemctl", &["disable", "--now", "anduinos-zram.service"]);
        let _ = exec::run_helper("rm", &["-f", ZRAM_SERVICE]);
        let _ = exec::run_helper("systemctl", &["daemon-reload"]);
        return Ok("Zram persistence removed".to_string());
    }

    let mut exec_start_lines = String::new();

    for (i, (size_mb, algo, priority)) in devices.iter().enumerate() {
        exec_start_lines.push_str(&format!(
            "# Device {}: {} MiB, algo={}, priority={}\n\
             ExecStart=/usr/lib/anduinos-swapcontrol/helper modprobe zram\n\
             ExecStart=/bin/bash -c 'DEV=$(/usr/lib/anduinos-swapcontrol/helper zramctl -f -s {}M -a {}) && /usr/lib/anduinos-swapcontrol/helper mkswap $DEV && /usr/lib/anduinos-swapcontrol/helper swapon -p {} $DEV'\n",
            i, size_mb, algo, priority,
            size_mb, algo, priority
        ));
    }

    let unit = format!(
        "# Managed by anduinos-swapcontrol-gtk. Do not edit manually.\n\
         [Unit]\n\
         Description=AnduinOS Zram Devices\n\
         Before=swap.target\n\n\
         [Service]\n\
         Type=oneshot\n\
         RemainAfterExit=yes\n\
         {}\n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        exec_start_lines
    );

    exec::write_sysfs(ZRAM_SERVICE, &unit)?;

    let _ = exec::run_helper("systemctl", &["daemon-reload"]);
    let _ = exec::run_helper("systemctl", &["enable", "anduinos-zram.service"]);

    Ok("Zram persistence enabled".to_string())
}
