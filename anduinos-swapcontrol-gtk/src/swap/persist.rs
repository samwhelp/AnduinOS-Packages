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

const ZRAM_SERVICE: &str = "/etc/systemd/system/anduinos-zram.service";

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
