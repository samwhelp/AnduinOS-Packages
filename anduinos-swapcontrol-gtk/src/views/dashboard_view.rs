use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;

use crate::i18n::i18n;
use crate::swap::{swapfile, zswap, zram, hibernation, ram_info, sysctl};
use crate::widgets::memory_ring::{MemoryRing, Segment};
use crate::widgets::usage_bar::UsageBar;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct DashboardView {
        pub ring: RefCell<Option<MemoryRing>>,
        pub legend: RefCell<Option<gtk::Box>>,
        pub zram_bar: RefCell<Option<UsageBar>>,
        pub zswap_bar: RefCell<Option<UsageBar>>,
        pub swap_bar: RefCell<Option<UsageBar>>,
        pub last_dmidecode: RefCell<Option<std::time::Instant>>,
        pub ram_total: RefCell<Option<gtk::Label>>,
        pub ram_type: RefCell<Option<gtk::Label>>,
        pub ram_speed: RefCell<Option<gtk::Label>>,
        pub ram_dimm: RefCell<Option<gtk::Label>>,
        pub swap_sub: RefCell<Option<gtk::Label>>,
        pub zswap_sub: RefCell<Option<gtk::Label>>,
        pub zram_sub: RefCell<Option<gtk::Label>>,
        pub hiber_sub: RefCell<Option<gtk::Label>>,
        pub swappiness_sub: RefCell<Option<gtk::Label>>,
        pub recommendation_box: RefCell<Option<gtk::Box>>,
        pub refresh_timer: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DashboardView {
        const NAME: &'static str = "DashboardView";
        type Type = super::DashboardView;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for DashboardView {
        fn constructed(&self) { self.parent_constructed(); self.obj().setup_ui(); self.obj().start_auto_refresh(); }
        fn dispose(&self) { self.obj().stop_auto_refresh(); }
    }
    impl WidgetImpl for DashboardView {}
    impl BoxImpl for DashboardView {}
}

glib::wrapper! {
    pub struct DashboardView(ObjectSubclass<imp::DashboardView>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

#[derive(Default)]
struct MemInfo { total: u64, used: u64, buffers: u64, cached: u64, free: u64 }

fn read_meminfo() -> MemInfo {
    let mut info = MemInfo::default();
    let Ok(content) = std::fs::read_to_string("/proc/meminfo") else { return info };
    for line in content.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() < 2 { continue; }
        let kb: u64 = p[1].parse().unwrap_or(0) * 1024;
        match p[0].trim_end_matches(':') {
            "MemTotal" => info.total = kb, "MemFree" => info.free = kb,
            "Buffers" => info.buffers = kb, "Cached" => info.cached = kb, _ => {}
        }
    }
    info.used = info.total.saturating_sub(info.free + info.buffers + info.cached);
    info
}

impl DashboardView {
    pub fn new() -> Self { glib::Object::builder().build() }

    fn start_auto_refresh(&self) {
        let weak = self.downgrade();
        let id = glib::timeout_add_local(std::time::Duration::from_secs(5), move || {
            if let Some(view) = weak.upgrade() {
                view.refresh_data();
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
        *self.imp().refresh_timer.borrow_mut() = Some(id);
    }

    fn stop_auto_refresh(&self) {
        if let Some(id) = self.imp().refresh_timer.borrow_mut().take() {
            id.remove();
        }
    }

    fn setup_ui(&self) {
        let imp = self.imp();
        self.set_orientation(gtk::Orientation::Vertical);
        self.set_spacing(18);
        self.set_margin_start(24); self.set_margin_end(24);
        self.set_margin_top(24); self.set_margin_bottom(24);
        self.set_vexpand(true);

        // Title
        self.append(&gtk::Label::builder().label(&i18n("Memory Overview"))
            .css_classes(["title-1"]).halign(gtk::Align::Start).build());
        self.append(&gtk::Label::builder()
            .label(&i18n("Real-time RAM, swap, and compression subsystem status"))
            .css_classes(["caption"]).halign(gtk::Align::Start).margin_start(2).build());
        // Swappiness recommendation
        {
            let total_ram = crate::swap::sysctl::read_total_ram().unwrap_or(32 * 1024 * 1024 * 1024);
            let ram_gb = total_ram as f64 / (1024.0 * 1024.0 * 1024.0);
            let sw = if ram_gb >= 16.0 { 10 } else if ram_gb >= 8.0 { 30 } else { 60 };
            self.append(&gtk::Label::builder().use_markup(true)
                .label(&format!("<i>Recommended swappiness: {} (for {:.0} GiB RAM)</i>", sw, ram_gb))
                .css_classes(["caption"]).halign(gtk::Align::Start).margin_start(2).build());
        }

        // ─── Recommendations (context-aware tips) ─────────────────────
        let rec_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(8)
            .margin_top(8).build();
        self.append(&rec_box);
        *imp.recommendation_box.borrow_mut() = Some(rec_box);

        // ─── RAM spec bar ────────────────────────────────────────────
        let spec = gtk::Grid::builder().row_spacing(8).column_spacing(8).column_homogeneous(true).build();
        let (c0, l0) = mini_stat("Total RAM", "...");
        let (c1, l1) = mini_stat("Type", "...");
        let (c2, l2) = mini_stat("Speed", "...");
        let (c3, l3) = mini_stat("Channels", "...");
        spec.attach(&c0,0,0,1,1); spec.attach(&c1,1,0,1,1);
        spec.attach(&c2,2,0,1,1); spec.attach(&c3,3,0,1,1);
        self.append(&spec);
        *imp.ram_total.borrow_mut() = Some(l0);
        *imp.ram_type.borrow_mut() = Some(l1);
        *imp.ram_speed.borrow_mut() = Some(l2);
        *imp.ram_dimm.borrow_mut() = Some(l3);

        // ─── Ring + Bars row ─────────────────────────────────────────
        let middle = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(24).build();

        // Left side: ring + legend
        let ring_col = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(8)
            .halign(gtk::Align::Center).valign(gtk::Align::Start).build();

        let ring = MemoryRing::new();
        ring.set_halign(gtk::Align::Center);
        ring.set_valign(gtk::Align::Start);
        ring_col.append(&ring);
        *imp.ring.borrow_mut() = Some(ring);

        let legend = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(14)
            .halign(gtk::Align::Center).build();
        ring_col.append(&legend);
        *imp.legend.borrow_mut() = Some(legend);

        middle.append(&ring_col);

        // Right: 3 usage bars (align top to match ring)
        let bars = gtk::Box::builder().orientation(gtk::Orientation::Vertical)
            .spacing(16).vexpand(true).valign(gtk::Align::Start).hexpand(true)
            .margin_top(10).build();

        let zram_bar = UsageBar::new("Zram", (1.0, 0.47, 0.0));
        bars.append(&zram_bar);
        *imp.zram_bar.borrow_mut() = Some(zram_bar);

        let zswap_bar = UsageBar::new("Zswap", (0.20, 0.82, 0.48));
        bars.append(&zswap_bar);
        *imp.zswap_bar.borrow_mut() = Some(zswap_bar);

        let swap_bar = UsageBar::new("Swap", (0.21, 0.52, 0.89));
        bars.append(&swap_bar);
        *imp.swap_bar.borrow_mut() = Some(swap_bar);

        middle.append(&bars);
        self.append(&middle);

        // ─── Bottom status cards ────────────────────────────────────
        let grid = gtk::Grid::builder().row_spacing(8).column_spacing(8).column_homogeneous(true).build();

        let (c1, s1) = info_card("drive-harddisk-symbolic", "Swap", "");
        let (c2, s2) = info_card("emblem-synchronizing-symbolic", "Zswap", "");
        let (c3, s3) = info_card("media-flash-symbolic", "Zram", "");
        let (c4, s4) = info_card("weather-clear-night-symbolic", "Hibernation", "");
        let (c5, s5) = info_card("preferences-system-symbolic", "Swappiness", "");

        grid.attach(&c1,0,0,1,1); grid.attach(&c2,1,0,1,1);
        grid.attach(&c3,2,0,1,1); grid.attach(&c4,3,0,1,1);
        grid.attach(&c5,4,0,1,1);
        self.append(&grid);

        *imp.swap_sub.borrow_mut() = Some(s1);
        *imp.zswap_sub.borrow_mut() = Some(s2);
        *imp.zram_sub.borrow_mut() = Some(s3);
        *imp.hiber_sub.borrow_mut() = Some(s4);
        *imp.swappiness_sub.borrow_mut() = Some(s5);
    }

    pub fn refresh_data(&self) {
        let imp = self.imp();

        // ─── RAM hardware ────────────────────────────────────────────
        let ram = ram_info::read_ram_basic(); // non-blocking, no pkexec

        // Try dmidecode with 30s cooldown between retries (prevents auth-spam but recovers)
        let now = std::time::Instant::now();
        let should_try = imp.last_dmidecode.borrow()
            .map(|t| now.duration_since(t) > std::time::Duration::from_secs(30))
            .unwrap_or(true);
        if should_try {
            *imp.last_dmidecode.borrow_mut() = Some(now);
            let (tx, rx) = async_channel::bounded(1);
            tokio::spawn(async move {
                let full = ram_info::read_ram_info_full();
                let _ = tx.send(full).await;
            });
            let weak = self.downgrade();
            glib::spawn_future_local(async move {
                if let Ok(full) = rx.recv().await {
                    if let Some(view) = weak.upgrade() {
                        let imp = view.imp();
                        if !full.ram_type.is_empty() {
                            if let Some(l) = imp.ram_type.borrow().as_ref() { l.set_text(&full.ram_type); }
                        }
                        if full.speed_mts > 0 {
                            let s = format!("{} MT/s", full.speed_mts);
                            if let Some(l) = imp.ram_speed.borrow().as_ref() { l.set_text(&s); }
                        }
                        let pop: Vec<_> = full.dimms.iter().filter(|d| d.size_gb > 0).collect();
                        if !pop.is_empty() {
                            let d = format!("{}×{}GB", pop.len(), pop[0].size_gb);
                            if let Some(l) = imp.ram_dimm.borrow().as_ref() { l.set_text(&d); }
                        }
                    }
                }
            });
        }
        // Only update RAM labels if dmidecode hasn't already loaded real data
        let ram_gb_str = format!("{:.0} GiB", ram.total_gb);
        if let Some(l) = imp.ram_total.borrow().as_ref() {
            l.set_text(&ram_gb_str); // total RAM always updates
        }
        // Type/Speed/Dimm: immutable hardware — set once, never touch again
        let has_real_data = imp.ram_type.borrow().as_ref()
            .map(|l| l.label().as_str() != "-" && l.label().as_str() != "..." && !l.label().as_str().contains("Auth"))
            .unwrap_or(false);
        if !has_real_data {
            if let Some(l) = imp.ram_type.borrow().as_ref() {
                l.set_text(if ram.ram_type.is_empty() { "-" } else { &ram.ram_type });
            }
            let speed_str = format!("{} MT/s", ram.speed_mts);
            if let Some(l) = imp.ram_speed.borrow().as_ref() {
                l.set_text(if ram.speed_mts > 0 { &speed_str } else { "-" });
            }
            if let Some(l) = imp.ram_dimm.borrow().as_ref() {
                let pop: Vec<_> = ram.dimms.iter().filter(|d| d.size_gb > 0).collect();
                let auth_str = i18n("Auth needed");
                let dimm_str = if !pop.is_empty() { format!("{}×{}GB", pop.len(), pop[0].size_gb) } else { String::new() };
                l.set_text(if pop.is_empty() { &auth_str } else { &dimm_str });
            }
        }

        // ─── Ring chart ─────────────────────────────────────────────
        let mem = read_meminfo();
        if let Some(ring) = imp.ring.borrow().as_ref() {
            ring.set_segments(vec![
                Segment { label: "Used".into(), value: mem.used as f64, color: (0.89,0.20,0.20) },
                Segment { label: "Buffers".into(), value: mem.buffers as f64, color: (0.20,0.55,0.91) },
                Segment { label: "Cached".into(), value: mem.cached as f64, color: (0.20,0.82,0.48) },
                Segment { label: "Free".into(), value: mem.free as f64, color: (0.60,0.60,0.60) },
            ]);
        }
        // Legend
        if let Some(legend) = imp.legend.borrow().as_ref() {
            while let Some(c) = legend.first_child() { legend.remove(&c); }
            let items: [(&str, (f64,f64,f64), u64); 4] = [
                ("Used", (0.89,0.20,0.20), mem.used),
                ("Buf", (0.20,0.55,0.91), mem.buffers),
                ("Cache", (0.20,0.82,0.48), mem.cached),
                ("Free", (0.60,0.60,0.60), mem.free),
            ];
            for (name, (r,g,b), val) in &items {
                let item = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(3).build();
                let dot = gtk::DrawingArea::builder().content_width(8).content_height(8)
                    .halign(gtk::Align::Center).valign(gtk::Align::Center).build();
                let (dr,dg,db) = (*r,*g,*b);
                dot.set_draw_func(move |_, ctx, w, h| {
                    ctx.set_source_rgb(dr,dg,db);
                    ctx.arc(w as f64/2., h as f64/2., 3.5, 0., 2.*std::f64::consts::PI); ctx.fill().ok();
                });
                item.append(&dot);
                let gb = *val as f64 / (1024.*1024.*1024.);
                item.append(&gtk::Label::builder().label(&format!("{} {:.1}G", name, gb)).css_classes(["caption"]).build());
                legend.append(&item);
            }
        }

        // ─── Zram bar ────────────────────────────────────────────────
        let devs = zram::read_zram_devices();
        if let Some(bar) = imp.zram_bar.borrow().as_ref() {
            if devs.is_empty() {
                bar.set_fraction(0.0, &i18n("No device"));
            } else {
                let d = &devs[0];
                let total_gb = d.size_bytes as f64 / (1024.0*1024.0*1024.0);
                let used_gb = d.used_bytes as f64 / (1024.0*1024.0*1024.0);
                // Use /proc/swaps usage (survives zram reset), mm_stat for compression ratio
                if used_gb > 0.001 {
                    let frac = d.used_bytes as f64 / d.size_bytes as f64;
                    let saved = if d.orig_data_size > 1024 * 1024 {
                        (1.0 - d.compr_data_size as f64 / d.orig_data_size as f64) * 100.0
                    } else { 0.0 };
                    let bar_str = format!("{:.2} / {:.1} GiB", used_gb, total_gb);
                    if saved > 0.0 {
                        bar.set_fraction(frac, &format!("{} · saved {:.0}%", bar_str, saved));
                    } else {
                        bar.set_fraction(frac, &bar_str);
                    }
                } else {
                    let bar_str = format!("Idle ({:.1} GiB available)", total_gb);
                    bar.set_fraction(0.0, &bar_str);
                }
            }
        }

        // ─── Zswap bar ───────────────────────────────────────────────
        if let Ok(cfg) = zswap::read_zswap_config() {
            if let Some(bar) = imp.zswap_bar.borrow().as_ref() {
                if cfg.enabled {
                    bar.set_fraction(cfg.max_pool_percent as f64 / 100.0,
                        &format!("{} · pool {}%", cfg.compressor, cfg.max_pool_percent));
                } else {
                    bar.set_fraction(0.0, &i18n("Disabled"));
                }
            }
        }

        // ─── Swap bar ────────────────────────────────────────────────
        if let Ok(status) = swapfile::read_swap_status() {
            if let Some(bar) = imp.swap_bar.borrow().as_ref() {
                if status.active && status.size_bytes > 0 {
                    let frac = status.used_bytes as f64 / status.size_bytes as f64;
                    let used_gb = status.used_bytes as f64 / (1024.0*1024.0*1024.0);
                    let total_gb = status.size_bytes as f64 / (1024.0*1024.0*1024.0);
                    bar.set_fraction(frac, &format!("{:.1} / {:.1} GiB", used_gb, total_gb));
                } else {
                    bar.set_fraction(0.0, &i18n("Inactive"));
                }
            }
        }

        // ─── Bottom cards ────────────────────────────────────────────
        if let Ok(status) = swapfile::read_swap_status() {
            let txt = if status.active {
                format!("{:.1} GiB", status.size_bytes as f64 / (1024.0*1024.0*1024.0))
            } else { i18n("Off") };
            if let Some(l) = imp.swap_sub.borrow().as_ref() { l.set_text(&txt); }
        }
        if let Ok(cfg) = zswap::read_zswap_config() {
            let a = i18n("On"); let d = i18n("Off");
            if let Some(l) = imp.zswap_sub.borrow().as_ref() { l.set_text(if cfg.enabled { &a } else { &d }); }
        }
        let devs = zram::read_zram_devices();
        if let Some(l) = imp.zram_sub.borrow().as_ref() {
            let dev_str = format!("{} dev", devs.len());
            let none_str = i18n("None");
            l.set_text(if devs.is_empty() { &none_str } else { &dev_str });
        }
        let h = hibernation::check_hibernation();
        let en = i18n("On"); let dis = i18n("Off");
        if let Some(l) = imp.hiber_sub.borrow().as_ref() { l.set_text(if h.enabled { &en } else { &dis }); }
        if let Ok(sw) = sysctl::read_swappiness() {
            let sw_str = format!("{}", sw);
            if let Some(l) = imp.swappiness_sub.borrow().as_ref() { l.set_text(&sw_str); }
        }

        // ─── Recommendations (at most one, highest priority first) ─────
        if let Some(rec_box) = imp.recommendation_box.borrow().as_ref() {
            while let Some(c) = rec_box.first_child() { rec_box.remove(&c); }

            let swap_active = swapfile::is_swap_active();
            let has_zram = !zram::read_zram_devices().is_empty();
            let zswap_on = zswap::read_zswap_config().map(|c| c.enabled).unwrap_or(false);

            if !swap_active {
                let card = build_rec_card((0.93, 0.55, 0.0),
                    &i18n("No disk swap detected"),
                    &i18n("When RAM is full the kernel may kill applications. Enable disk swap for a reliable safety net."));
                rec_box.append(&card);
            } else if !has_zram {
                let card = build_rec_card((0.93, 0.55, 0.0),
                    &i18n("No Zram device detected"),
                    &i18n("Zram compresses swap pages in RAM — it's 10× faster than disk swap and dramatically reduces I/O. Create a Zram device for snappier performance under memory pressure."));
                rec_box.append(&card);
            } else if !zswap_on {
                let card = build_rec_card((0.93, 0.55, 0.0),
                    &i18n("Zswap is not enabled"),
                    &i18n("Zswap adds a compressed RAM write-back cache between Zram and disk swap, preventing the system from freezing when swap is finally needed. Enable it for a smoother experience."));
                rec_box.append(&card);
            } else {
                let card = build_rec_card((0.15, 0.72, 0.25),
                    &i18n("Optimal memory configuration"),
                    &i18n("Zram → Zswap → Disk Swap — your three-tier memory defense is fully armed. Maximum performance and safety."));
                rec_box.append(&card);
            }
        }
    }
}

fn build_rec_card(accent: (f64, f64, f64), title: &str, subtitle: &str) -> gtk::Box {
    let card = gtk::Box::builder().orientation(gtk::Orientation::Horizontal)
        .css_classes(["card"]).spacing(12).build();
    let bar = gtk::DrawingArea::builder().content_width(4).vexpand(true).halign(gtk::Align::Fill).valign(gtk::Align::Fill).build();
    let (r, g, b) = accent;
    bar.set_draw_func(move |_, ctx, w, h| {
        ctx.set_source_rgb(r, g, b);
        ctx.rectangle(0.0, 0.0, w as f64, h as f64);
        ctx.fill().ok();
    });
    card.append(&bar);
    let inner = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(4)
        .hexpand(true).margin_start(8).margin_end(16).margin_top(12).margin_bottom(12).build();
    inner.append(&gtk::Label::builder().label(title).css_classes(["heading"]).halign(gtk::Align::Start).build());
    inner.append(&gtk::Label::builder().label(subtitle).css_classes(["caption"]).wrap(true).halign(gtk::Align::Start).build());
    card.append(&inner);
    card
}

fn info_card(icon: &str, title: &str, subtitle: &str) -> (gtk::Box, gtk::Label) {
    let card = gtk::Box::builder().orientation(gtk::Orientation::Vertical).css_classes(["card"]).build();
    let inner = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(8)
        .margin_start(10).margin_end(10).margin_top(8).margin_bottom(8).build();
    inner.append(&gtk::Image::builder().icon_name(icon).pixel_size(20).build());
    let tb = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(1).hexpand(true).build();
    tb.append(&gtk::Label::builder().label(title).css_classes(["caption"]).halign(gtk::Align::Start).build());
    let sub = gtk::Label::builder().label(subtitle).css_classes(["heading"]).halign(gtk::Align::Start).build();
    tb.append(&sub);
    inner.append(&tb);
    card.append(&inner);
    (card, sub)
}

fn mini_stat(label: &str, value: &str) -> (gtk::Box, gtk::Label) {
    let card = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2)
        .css_classes(["card"]).build();
    let inner = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2)
        .margin_start(8).margin_end(8).margin_top(8).margin_bottom(8).build();
    let val = gtk::Label::builder().label(value).css_classes(["title-3"])
        .halign(gtk::Align::Center).build();
    let cap = gtk::Label::builder().label(label).css_classes(["caption"])
        .halign(gtk::Align::Center).build();
    inner.append(&val); inner.append(&cap);
    card.append(&inner);
    (card, val)
}
