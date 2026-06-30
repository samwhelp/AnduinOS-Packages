use crate::swap::types::HibernationStatus;
use std::fs;

/// Detect hibernation capability from multiple sources.
pub fn check_hibernation() -> HibernationStatus {
    let mut status = HibernationStatus::default();

    // 1. Check /sys/power/state — does the kernel support suspend-to-disk?
    if let Ok(content) = fs::read_to_string(crate::config::SYS_POWER_STATE) {
        status.system_supports = content.split_whitespace().any(|s| s == "disk");
    }

    // 2. Check /sys/power/disk — is it enabled? "[disabled]" means explicitly off
    if let Ok(content) = fs::read_to_string(crate::config::SYS_POWER_DISK) {
        status.enabled = !content.trim().starts_with("[disabled]");
    }

    // 3. Parse /proc/cmdline for resume= and resume_offset=
    if let Ok(content) = fs::read_to_string(crate::config::PROC_CMDLINE) {
        for token in content.split_whitespace() {
            if let Some(val) = token.strip_prefix("resume=") {
                status.resume_device = Some(val.to_string());
            }
            if let Some(val) = token.strip_prefix("resume_offset=") {
                status.resume_offset = val.parse::<u64>().ok();
            }
        }
    }

    // 4. Check /etc/initramfs-tools/conf.d/resume
    if let Ok(content) = fs::read_to_string(crate::config::INITRAMFS_RESUME) {
        for line in content.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("RESUME=") {
                let val = val.trim_matches('"');
                if !val.is_empty() {
                    status.initramfs_resume = Some(val.to_string());
                }
            }
        }
    }

    status
}

/// Check whether hibernation is actively configured (any resume source present).
pub fn is_hibernation_configured() -> bool {
    let status = check_hibernation();
    status.enabled || status.resume_device.is_some() || status.initramfs_resume.is_some()
}

/// Validate that a given swap size (in bytes) is sufficient for hibernation.
/// Rule of thumb: swap must be at least as large as total RAM.
pub fn validate_swap_size_for_hibernation(swap_bytes: u64) -> Result<(), String> {
    let total_ram = crate::swap::sysctl::read_total_ram()?;

    if swap_bytes < total_ram {
        let ram_gb = total_ram as f64 / (1024.0 * 1024.0 * 1024.0);
        let swap_gb = swap_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        return Err(format!(
            "Swap size ({:.1} GiB) is smaller than total RAM ({:.1} GiB). \
             This may cause hibernation to fail. Please increase swap size to at least {:.1} GiB.",
            swap_gb, ram_gb, ram_gb
        ));
    }

    Ok(())
}

/// Get the physical offset of /swapfile for resume_offset= in GRUB.
/// Uses `filefrag -v /swapfile` to find the first extent's physical offset.
pub fn get_swapfile_offset() -> Option<u64> {
    use std::process::Command;

    let output = Command::new("filefrag")
        .args(["-v", crate::config::SWAPFILE_PATH])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    // Look for the first extent line (not header)
    // Format: "   0:        0..    4095:    2048..    6143:     30:"
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Filesystem") || line.starts_with("ext:") {
            continue;
        }
        // Look for the first physical offset after the logical extents
        // e.g. "0: 0.. X: Y.. Z:" → Z is the physical start in filesystem blocks
        if let Some(colon_idx) = line.find(':') {
            let rest = &line[colon_idx + 1..];
            let parts: Vec<&str> = rest.split("..").collect();
            if parts.len() >= 2 {
                // The physical start is in the second ".."-delimited segment after the second ":"
                // Simpler approach: look for "    数字.." pattern (second colon group)
                let segments: Vec<&str> = line.split(':').collect();
                if segments.len() >= 4 {
                    // segments[3] is like "    2048..    6143"
                    let phys = segments[3].trim();
                    if let Some(dot) = phys.find("..") {
                        let start = &phys[..dot].trim();
                        if let Ok(block) = start.parse::<u64>() {
                            // Convert filesystem blocks to bytes (typical block size = 4096)
                            // Actually filefrag uses filesystem block size; we return the block offset
                            // and the caller multiplies by block size as needed
                            return Some(block);
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_hibernation() {
        let status = check_hibernation();
        // This system has hibernation disabled
        assert!(!status.enabled);
    }

    #[test]
    fn test_is_hibernation_configured() {
        let result = is_hibernation_configured();
        // Should be false on this system
        assert!(!result);
    }
}
