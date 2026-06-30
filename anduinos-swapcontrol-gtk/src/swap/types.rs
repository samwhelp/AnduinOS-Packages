/// Represents the status of a swap device (file or partition).
#[derive(Debug, Clone, Default)]
pub struct SwapStatus {
    pub active: bool,
    pub path: String,
    pub size_bytes: u64,
    pub used_bytes: u64,
    pub priority: i32,
}

/// Represents a zswap configuration snapshot.
#[derive(Debug, Clone, Default)]
pub struct ZswapConfig {
    pub enabled: bool,
    pub compressor: String,
    pub max_pool_percent: u8,
    pub accept_threshold_percent: u8,
    pub shrinker_enabled: bool,
}

/// Runtime statistics for zswap (from debugfs, only when enabled).
#[derive(Debug, Clone, Default)]
pub struct ZswapStats {
    pub stored_pages: u64,
    pub stored_bytes: u64,
    pub pool_total_size: u64,
    pub pool_limit_hit: u64,
    pub written_back_pages: u64,
    pub rejected_compress_poor: u64,
    pub rejected_kmemcache_fail: u64,
}

/// Represents a zram block device.
#[derive(Debug, Clone, Default)]
pub struct ZramDevice {
    pub name: String,            // e.g. "zram0"
    pub size_bytes: u64,
    pub used_bytes: u64,
    pub compr_data_size: u64,    // compressed size in RAM
    pub orig_data_size: u64,     // original uncompressed size
    pub mem_used_total: u64,     // total memory used (metadata + compressed)
    pub comp_algorithm: String,
    pub swap_priority: i32,
}

/// Summary of the hibernation subsystem state.
#[derive(Debug, Clone, Default)]
pub struct HibernationStatus {
    /// true if /sys/power/state contains "disk"
    pub system_supports: bool,
    /// true if /sys/power/disk is not "[disabled]"
    pub enabled: bool,
    /// resume= argument from kernel cmdline (if any)
    pub resume_device: Option<String>,
    /// resume_offset= from kernel cmdline (swapfile case)
    pub resume_offset: Option<u64>,
    /// RESUME= from /etc/initramfs-tools/conf.d/resume
    pub initramfs_resume: Option<String>,
}

/// Memory overview used by the dashboard.
#[derive(Debug, Clone, Default)]
pub struct MemoryOverview {
    pub total_ram_bytes: u64,
    pub available_ram_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub zswap_enabled: bool,
    pub zram_devices: Vec<ZramDevice>,
    pub hibernation: HibernationStatus,
}

/// Scene presets for one-click configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Performance,
    Balanced,
    MemorySaver,
    Custom,
}

impl Preset {
    pub fn swappiness(&self) -> u8 {
        match self {
            Preset::Performance => 1,
            Preset::Balanced => 10,
            Preset::MemorySaver => 60,
            Preset::Custom => 0, // user-defined
        }
    }

    pub fn from_swappiness(v: u8) -> Self {
        match v {
            1 => Preset::Performance,
            10 => Preset::Balanced,
            60 => Preset::MemorySaver,
            _ => Preset::Custom,
        }
    }
}
