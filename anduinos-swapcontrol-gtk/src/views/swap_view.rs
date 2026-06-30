use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;

use crate::i18n::i18n;
use crate::swap::{swapfile, sysctl, hibernation, zswap};
use crate::utils;
use crate::widgets::usage_bar::UsageBar;

const COMPRESSORS: &[&str] = &["lzo", "lz4", "lz4hc", "zstd", "deflate", "842"];

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct SwapView {
        pub operation_running: RefCell<bool>,
        pub usage_bar: RefCell<Option<UsageBar>>,
        pub status_icon: RefCell<Option<gtk::Image>>,
        pub status_label: RefCell<Option<gtk::Label>>,
        pub usage_label: RefCell<Option<gtk::Label>>,
        pub enable_switch: RefCell<Option<gtk::Switch>>,
        pub swappiness_scale: RefCell<Option<gtk::Scale>>,
        pub size_scale: RefCell<Option<gtk::Scale>>,
        pub apply_revealer: RefCell<Option<gtk::Revealer>>,
        pub apply_btn: RefCell<Option<gtk::Button>>,
        pub spinner: RefCell<Option<gtk::Spinner>>,
        pub refreshing: RefCell<bool>,
        pub orig_swap: RefCell<u8>,
        pub orig_size: RefCell<u64>,
        // Zswap widgets
        pub zswap_card: RefCell<Option<gtk::Box>>,
        pub zswap_switch: RefCell<Option<gtk::Switch>>,
        pub compressor_dropdown: RefCell<Option<gtk::DropDown>>,
        pub pool_scale: RefCell<Option<gtk::Scale>>,
        pub threshold_scale: RefCell<Option<gtk::Scale>>,
        pub shrinker_switch: RefCell<Option<gtk::Switch>>,
        pub zswap_adv_box: RefCell<Option<gtk::Box>>,
        pub orig_compressor: RefCell<String>,
        pub orig_pool: RefCell<u8>,
        pub orig_threshold: RefCell<u8>,
        pub orig_shrinker: RefCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SwapView {
        const NAME: &'static str = "SwapView";
        type Type = super::SwapView;
        type ParentType = gtk::Box;
    }
    impl ObjectImpl for SwapView {
        fn constructed(&self) { self.parent_constructed(); self.obj().setup_ui(); }
    }
    impl WidgetImpl for SwapView {}
    impl BoxImpl for SwapView {}
}

glib::wrapper! {
    pub struct SwapView(ObjectSubclass<imp::SwapView>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl SwapView {
    pub fn new() -> Self { glib::Object::builder().build() }

    fn setup_ui(&self) {
        let imp = self.imp();
        self.set_orientation(gtk::Orientation::Vertical);
        self.set_spacing(0);
        self.set_vexpand(true);
        let scroll = gtk::ScrolledWindow::builder().vexpand(true).build();
        let inner = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18)
            .margin_start(24).margin_end(24).margin_top(24).margin_bottom(24).build();
        scroll.set_child(Some(&inner));


        // Title + subtitle
        inner.append(&gtk::Label::builder().label(&i18n("Disk Swap Configuration"))
            .css_classes(["title-1"]).halign(gtk::Align::Start).build());
        inner.append(&gtk::Label::builder()
            .label(&i18n("Disk-based swap file — the last line of defense when RAM is full"))
            .css_classes(["caption"]).halign(gtk::Align::Start).margin_start(2).build());

        // Recommendation
        {
            let total_ram = sysctl::read_total_ram().unwrap_or(32 * 1024 * 1024 * 1024);
            let ram_gb = total_ram as f64 / (1024.0 * 1024.0 * 1024.0);
            let rec_size = (total_ram / (1024*1024*1024)).max(1);
            let sw = if ram_gb >= 16.0 { 10 } else if ram_gb >= 8.0 { 30 } else { 60 };
            inner.append(&gtk::Label::builder().use_markup(true)
                .label(&{let s=format!("<i>Recommended: {} GiB swap, swappiness {} (for {:.0} GiB RAM)</i>", rec_size, sw, ram_gb); s})
                .css_classes(["caption"]).halign(gtk::Align::Start).margin_start(2).build());
        }

        // Spinner
        let spinner = gtk::Spinner::builder().halign(gtk::Align::Center).visible(false).build();
        inner.append(&spinner);
        *imp.spinner.borrow_mut() = Some(spinner);

        // ─── Status card ────────────────────────────────────────────
        let status_card = gtk::Box::builder().orientation(gtk::Orientation::Vertical)
            .css_classes(["card"]).spacing(8).build();
        let status_inner = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(12)
            .margin_start(16).margin_end(16).margin_top(14).margin_bottom(14).build();

        let status_icon = gtk::Image::builder().pixel_size(32).build();
        status_inner.append(&status_icon);
        *imp.status_icon.borrow_mut() = Some(status_icon);

        let status_text = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2).hexpand(true).build();
        let status_label = gtk::Label::builder().label(&i18n("Loading...")).css_classes(["heading"]).halign(gtk::Align::Start).build();
        status_text.append(&status_label);
        let usage_label = gtk::Label::builder().label("").css_classes(["caption"]).halign(gtk::Align::Start).build();
        status_text.append(&usage_label);
        status_inner.append(&status_text);
        *imp.status_label.borrow_mut() = Some(status_label);
        *imp.usage_label.borrow_mut() = Some(usage_label);

        let enable_switch = gtk::Switch::builder().valign(gtk::Align::Center).build();
        status_inner.append(&enable_switch);
        status_card.append(&status_inner);
        inner.append(&status_card);
        *imp.enable_switch.borrow_mut() = Some(enable_switch);

        // Usage bar
        let usage_bar = UsageBar::new("Swap usage", (0.21, 0.52, 0.89));
        inner.append(&usage_bar);
        *imp.usage_bar.borrow_mut() = Some(usage_bar);

        // ─── Zswap sub-card (visible only when swap is active) ──────
        let zswap_card = gtk::Box::builder().orientation(gtk::Orientation::Vertical)
            .css_classes(["card"]).spacing(8).visible(false).build();
        let zswap_inner = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(12)
            .margin_start(16).margin_end(16).margin_top(14).margin_bottom(14).build();
        let zswap_icon = gtk::Image::builder().icon_name("document-save-symbolic").pixel_size(24).build();
        zswap_inner.append(&zswap_icon);
        let zswap_text = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2).hexpand(true).build();
        zswap_text.append(&gtk::Label::builder().label(&i18n("Zswap")).css_classes(["heading"]).halign(gtk::Align::Start).build());
        zswap_text.append(&gtk::Label::builder().label(&i18n("Compress swap pages in a RAM pool — faster than disk swap"))
            .css_classes(["caption"]).halign(gtk::Align::Start).build());
        zswap_inner.append(&zswap_text);
        let zswap_switch = gtk::Switch::builder().valign(gtk::Align::Center).build();
        zswap_inner.append(&zswap_switch);
        zswap_card.append(&zswap_inner);
        inner.append(&zswap_card);
        *imp.zswap_card.borrow_mut() = Some(zswap_card);
        *imp.zswap_switch.borrow_mut() = Some(zswap_switch);

        // ─── Size slider (always visible) ───────────────────────────
        let total_ram = sysctl::read_total_ram().unwrap_or(32 * 1024 * 1024 * 1024);
        let max_size = (total_ram / (1024*1024*1024) * 2).max(64);
        let hiber_active = hibernation::is_hibernation_configured();
        let min_size = if hiber_active { (total_ram / (1024*1024*1024)).max(1) + 1 } else { 1 };
        let rec_size = (total_ram / (1024*1024*1024)).max(1);
        let size_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12)
            .css_classes(["card"]).margin_top(6).build();
        let size_inner = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6)
            .margin_start(16).margin_end(16).margin_top(14).margin_bottom(14).build();
        let size_scale = gtk::Scale::builder().orientation(gtk::Orientation::Horizontal)
            .adjustment(&gtk::Adjustment::new(rec_size.max(min_size) as f64, min_size as f64, max_size as f64, 1.0, 4.0, 0.0))
            .draw_value(true).value_pos(gtk::PositionType::Right).hexpand(true).build();
        size_inner.append(&gtk::Label::builder().label(&i18n("Swap file size")).css_classes(["heading"]).halign(gtk::Align::Start).build());
        let hint_str = if hiber_active {
            format!("Hibernation active — minimum {} GiB (RAM + 1 GiB). Recommended: {} GiB", min_size, rec_size.max(min_size))
        } else {
            format!("Hibernation not detected — recommended: {} GiB", rec_size)
        };
        size_inner.append(&gtk::Label::builder().label(&hint_str).css_classes(["caption"]).halign(gtk::Align::Start).build());
        size_inner.append(&size_scale);
        size_box.append(&size_inner);
        inner.append(&size_box);
        *imp.size_scale.borrow_mut() = Some(size_scale);

        // ─── Advanced expander ──────────────────────────────────────
        let expander = gtk::Expander::builder().label(&i18n("Advanced settings")).margin_top(6).build();
        let adv_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(16)
            .margin_start(12).margin_top(12).build();

        let ram_gb = total_ram as f64 / (1024.0 * 1024.0 * 1024.0);
        let rec_sw = if ram_gb >= 16.0 { 10 } else if ram_gb >= 8.0 { 30 } else { 60 };
        let swappiness_scale = gtk::Scale::builder().orientation(gtk::Orientation::Horizontal)
            .adjustment(&gtk::Adjustment::new(rec_sw as f64, 0.0, 100.0, 1.0, 10.0, 0.0))
            .draw_value(true).value_pos(gtk::PositionType::Right).hexpand(true).build();
        adv_box.append(&labeled_widget(
            &i18n("Swappiness"),
            &format!("How aggressively the kernel swaps — lower = stay in RAM longer (recommended: {})", rec_sw),
            &swappiness_scale));
        *imp.swappiness_scale.borrow_mut() = Some(swappiness_scale);

        // ── Zswap advanced rows ────────────────────────────────────
        let zswap_adv_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(16)
            .visible(false).build();

        let compressor_dropdown = gtk::DropDown::from_strings(COMPRESSORS);
        zswap_adv_box.append(&labeled_widget(
            &i18n("Compression algorithm"),
            &i18n("lz4 = fast, zstd = best compression, lzo = safest fallback. Changing requires zswap restart."),
            &compressor_dropdown));

        let pool_scale = gtk::Scale::builder().orientation(gtk::Orientation::Horizontal)
            .adjustment(&gtk::Adjustment::new(20.0, 1.0, 50.0, 1.0, 5.0, 0.0))
            .draw_value(true).value_pos(gtk::PositionType::Right).hexpand(true).build();
        zswap_adv_box.append(&labeled_widget(
            &i18n("Maximum pool percent"),
            &i18n("Max % of RAM the compressed pool may occupy. Recommended: 20%"),
            &pool_scale));

        let threshold_scale = gtk::Scale::builder().orientation(gtk::Orientation::Horizontal)
            .adjustment(&gtk::Adjustment::new(90.0, 0.0, 100.0, 1.0, 10.0, 0.0))
            .draw_value(true).value_pos(gtk::PositionType::Right).hexpand(true).build();
        zswap_adv_box.append(&labeled_widget(
            &i18n("Accept threshold percent"),
            &i18n("Only store pages that compress to ≤ this % of original. Higher = more pages cached. Recommended: 90%"),
            &threshold_scale));

        let shrinker_switch = gtk::Switch::builder().valign(gtk::Align::Center).build();
        let shrinker_row = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(12).build();
        let shrinker_label = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2).hexpand(true).build();
        shrinker_label.append(&gtk::Label::builder().label(&i18n("Shrinker")).halign(gtk::Align::Start).build());
        shrinker_label.append(&gtk::Label::builder()
            .label(&i18n("Reclaim pool pages under memory pressure. Recommended: ON"))
            .css_classes(["caption"]).halign(gtk::Align::Start).build());
        shrinker_row.append(&shrinker_label);
        shrinker_row.append(&shrinker_switch);
        zswap_adv_box.append(&shrinker_row);

        adv_box.append(&zswap_adv_box);
        *imp.zswap_adv_box.borrow_mut() = Some(zswap_adv_box);
        *imp.compressor_dropdown.borrow_mut() = Some(compressor_dropdown);
        *imp.pool_scale.borrow_mut() = Some(pool_scale);
        *imp.threshold_scale.borrow_mut() = Some(threshold_scale);
        *imp.shrinker_switch.borrow_mut() = Some(shrinker_switch);

        expander.set_child(Some(&adv_box));
        inner.append(&expander);

        // ─── Apply button (revealed on change) ─────────────────────
        let revealer = gtk::Revealer::builder().transition_type(gtk::RevealerTransitionType::SlideDown).build();
        let apply_btn = gtk::Button::builder().label(&i18n("Apply Settings"))
            .halign(gtk::Align::Center).css_classes(["suggested-action", "pill"]).build();
        revealer.set_child(Some(&apply_btn));
        inner.append(&revealer);
        *imp.apply_btn.borrow_mut() = Some(apply_btn);
        *imp.apply_revealer.borrow_mut() = Some(revealer);

        self.append(&scroll);
        self.connect_signals();
    }

    fn connect_signals(&self) {
        let imp = self.imp();

        // Enable/Disable switch — use notify::active (fires AFTER visual change)
        if let Some(sw) = imp.enable_switch.borrow().as_ref() {
            let weak_self = self.downgrade();
            sw.connect_notify_local(Some("active"), move |sw, _| {
                if let Some(v) = weak_self.upgrade() {
                    if *v.imp().refreshing.borrow() { return; }
                    v.toggle_swap(sw.is_active());
                }
            });
        }

        // Apply button
        if let Some(btn) = imp.apply_btn.borrow().as_ref() {
            let weak_self = self.downgrade();
            btn.connect_clicked(move |_| { if let Some(v) = weak_self.upgrade() { v.apply_all(); } });
        }

        // Zswap enable/disable switch
        if let Some(zsw) = imp.zswap_switch.borrow().as_ref() {
            let weak_self = self.downgrade();
            zsw.connect_notify_local(Some("active"), move |zsw, _| {
                if let Some(v) = weak_self.upgrade() {
                    if *v.imp().refreshing.borrow() { return; }
                    v.toggle_zswap(zsw.is_active());
                }
            });
        }

        // Track changes on advanced controls
        let weak_self = self.downgrade();
        let check = move || { if let Some(v) = weak_self.upgrade() { if !*v.imp().refreshing.borrow() { v.check_changes(); } } };
        if let Some(s) = imp.swappiness_scale.borrow().as_ref() { let c = check.clone(); s.connect_value_changed(move |_| c()); }
        if let Some(s) = imp.size_scale.borrow().as_ref() { let c = check.clone(); s.connect_value_changed(move |_| c()); }
        if let Some(d) = imp.compressor_dropdown.borrow().as_ref() { let c = check.clone(); d.connect_notify_local(Some("selected"), move |_, _| c()); }
        if let Some(s) = imp.pool_scale.borrow().as_ref() { let c = check.clone(); s.connect_value_changed(move |_| c()); }
        if let Some(s) = imp.threshold_scale.borrow().as_ref() { let c = check.clone(); s.connect_value_changed(move |_| c()); }
        if let Some(s) = imp.shrinker_switch.borrow().as_ref() { let c = check; s.connect_notify_local(Some("active"), move |_, _| c()); }
    }

    fn check_changes(&self) {
        let imp = self.imp();
        let sw = imp.swappiness_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(10);
        let sz = imp.size_scale.borrow().as_ref().map(|s| s.value() as u64).unwrap_or(2);
        let comp = imp.compressor_dropdown.borrow().as_ref().and_then(|d| {
            let idx = d.selected() as usize;
            COMPRESSORS.get(idx).map(|s| s.to_string())
        }).unwrap_or_else(|| "lzo".to_string());
        let pool = imp.pool_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(20);
        let thresh = imp.threshold_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(90);
        let shrinker = imp.shrinker_switch.borrow().as_ref().map(|s| s.is_active()).unwrap_or(true);
        let changed = sw != *imp.orig_swap.borrow()
            || sz != *imp.orig_size.borrow()
            || comp != *imp.orig_compressor.borrow()
            || pool != *imp.orig_pool.borrow()
            || thresh != *imp.orig_threshold.borrow()
            || shrinker != *imp.orig_shrinker.borrow();
        if let Some(r) = imp.apply_revealer.borrow().as_ref() { r.set_reveal_child(changed); }
    }

    fn run_op(&self, msg: String, task: impl FnOnce() -> Result<(), String> + Send + 'static) {
        if self.imp().operation_running.borrow().clone() { return; }
        *self.imp().operation_running.borrow_mut() = true;
        let w = self.downgrade();
        glib::spawn_future_local(async move {
            let win = w.upgrade().and_then(|v| v.root().and_then(|r| r.downcast::<gtk::Window>().ok()));
            let result: Result<(), String> = if let Some(p) = win {
                crate::progress_dialog::run_with_progress(&p, &msg, task).await
            } else { task() };
            if let Some(v) = w.upgrade() {
                *v.imp().operation_running.borrow_mut() = false;
                v.refresh_data();
                if let Err(e) = result {
                    let win = v.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                    if let Some(p) = win { utils::show_error(&p, &i18n("Error"), &e); }
                }
                if let Some(r) = v.imp().apply_revealer.borrow().as_ref() { r.set_reveal_child(false); }
            }
        });
    }

    fn toggle_zswap(&self, enable: bool) {
        let weak = self.downgrade();
        let msg = if enable { i18n("Enabling Zswap...") } else { i18n("Disabling Zswap...") };
        let do_it = move || { if let Some(v) = weak.upgrade() { v.run_op(msg, move || { if enable { zswap::enable_zswap().map(|_| ()) } else { zswap::disable_zswap().map(|_| ()) } }); } };
        do_it();
    }
    fn toggle_swap(&self, enable: bool) {
        let is_active = swapfile::is_swap_active();
        if enable == is_active { return; }
        let needs_confirm = is_active && hibernation::is_hibernation_configured();
        let weak_self = self.downgrade();

        let do_toggle = move || {
            let msg = if is_active { i18n("Disabling swap…") } else { i18n("Enabling swap…") };
            if let Some(v) = weak_self.upgrade() {
                v.run_op(msg, move || {
                    if is_active { swapfile::disable_swapfile().map(|_| ()) } else { swapfile::enable_swapfile().map(|_| ()) }
                });
            }
        };

        if needs_confirm {
            let win = self.root().and_then(|r| r.downcast::<gtk::Window>().ok());
            if let Some(p) = win {
                utils::show_confirm(&p, &i18n("Hibernation Warning"),
                    &i18n("Disabling swap will break hibernation. Continue?"), do_toggle);
            }
        } else { do_toggle(); }
    }

    fn apply_all(&self) {
        let imp = self.imp();
        let sw = imp.swappiness_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(10);
        let sz = imp.size_scale.borrow().as_ref().map(|s| s.value() as u64).unwrap_or(2);
        let comp = imp.compressor_dropdown.borrow().as_ref().and_then(|d| {
            let idx = d.selected() as usize;
            COMPRESSORS.get(idx).map(|s| s.to_string())
        }).unwrap_or_else(|| "lzo".to_string());
        let pool = imp.pool_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(20);
        let thresh = imp.threshold_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(90);
        let shrinker = imp.shrinker_switch.borrow().as_ref().map(|s| s.is_active()).unwrap_or(true);
        let orig_comp = imp.orig_compressor.borrow().clone();
        let orig_pool = *imp.orig_pool.borrow();
        let orig_thresh = *imp.orig_threshold.borrow();
        let orig_shrinker = *imp.orig_shrinker.borrow();
        let weak_self = self.downgrade();
        let weak_self2 = weak_self.clone();

        let needs_warn = hibernation::is_hibernation_configured() && {
            let total_ram = sysctl::read_total_ram().unwrap_or(0);
            (sz * 1024 * 1024 * 1024) < total_ram
        };

        let do_apply = move || {
            if let Some(v) = weak_self2.upgrade() {
                v.run_op(i18n("Applying settings…"), move || {
                    let mut errs = Vec::new();
                    if let Err(e) = sysctl::set_swappiness(sw) { errs.push(format!("Swappiness: {}", e)); }
                    if let Err(e) = swapfile::resize_swapfile(sz) { errs.push(format!("Resize: {}", e)); }
                    if comp != orig_comp {
                        if let Err(e) = zswap::set_compressor(&comp) { errs.push(format!("Compressor: {}", e)); }
                    }
                    if pool != orig_pool {
                        if let Err(e) = zswap::set_max_pool_percent(pool) { errs.push(format!("Pool: {}", e)); }
                    }
                    if thresh != orig_thresh {
                        if let Err(e) = zswap::set_accept_threshold(thresh) { errs.push(format!("Threshold: {}", e)); }
                    }
                    if shrinker != orig_shrinker {
                        if let Err(e) = zswap::set_shrinker(shrinker) { errs.push(format!("Shrinker: {}", e)); }
                    }
                    if errs.is_empty() { Ok(()) } else { Err(errs.join("\n")) }
                });
            }
        };

        if needs_warn {
            let total_ram = sysctl::read_total_ram().unwrap_or(0);
            let ram_gb = total_ram as f64 / (1024.0*1024.0*1024.0);
            if let Some(view) = weak_self.upgrade() {
                let win = view.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                if let Some(p) = win {
                    utils::show_confirm(&p, &i18n("Hibernation Warning"),
                        &format!("Swap ({:.0} GiB) < RAM ({:.1} GiB) — hibernation may fail. Continue?", sz, ram_gb),
                        do_apply);
                }
            }
        } else { do_apply(); }
    }

    pub fn refresh_data(&self) {
        let imp = self.imp();
        *imp.refreshing.borrow_mut() = true;

        if let Ok(status) = swapfile::read_swap_status() {
            let a_str = i18n("Active"); let i_str = i18n("Inactive");
            if let Some(l) = imp.status_label.borrow().as_ref() {
                l.set_text(if status.active { &a_str } else { &i_str });
            }
            if let Some(icon) = imp.status_icon.borrow().as_ref() {
                icon.set_icon_name(if status.active { Some("emblem-ok-symbolic") } else { Some("emblem-important-symbolic") });
            }
            if let Some(l) = imp.usage_label.borrow().as_ref() {
                if status.active {
                    let total_gb = status.size_bytes as f64 / (1024.0*1024.0*1024.0);
                    let used_gb = status.used_bytes as f64 / (1024.0*1024.0*1024.0);
                    l.set_text(&format!("{:.1} GiB used / {:.1} GiB total", used_gb, total_gb));
                } else {
                    l.set_text("");
                }
            }
            if let Some(bar) = imp.usage_bar.borrow().as_ref() {
                if status.active && status.size_bytes > 0 {
                    let frac = status.used_bytes as f64 / status.size_bytes as f64;
                    let used_gb = status.used_bytes as f64 / (1024.0*1024.0*1024.0);
                    let total_gb = status.size_bytes as f64 / (1024.0*1024.0*1024.0);
                    bar.set_fraction(frac, &format!("{:.1} / {:.1} GiB", used_gb, total_gb));
                } else {
                    bar.set_fraction(0.0, &i18n("Inactive"));
                }
            }
            if let Some(sw) = imp.enable_switch.borrow().as_ref() {
                sw.set_active(status.active);
            }
            // Update size slider to match actual swapfile
            if status.active {
                let gb = status.size_bytes / (1024*1024*1024);
                if let Some(s) = imp.size_scale.borrow().as_ref() { s.set_value(gb as f64); }
                *imp.orig_size.borrow_mut() = gb;
            }
        }

        if let Ok(val) = sysctl::read_swappiness() {
            if let Some(s) = imp.swappiness_scale.borrow().as_ref() { s.set_value(val as f64); }
            *imp.orig_swap.borrow_mut() = val;
        }

        // Zswap state
        let swap_active = imp.enable_switch.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false);
        if let Some(card) = imp.zswap_card.borrow().as_ref() { card.set_visible(swap_active); }
        if swap_active {
            if let Ok(zc) = zswap::read_zswap_config() {
                if let Some(zsw) = imp.zswap_switch.borrow().as_ref() { zsw.set_active(zc.enabled); }
                if let Some(d) = imp.compressor_dropdown.borrow().as_ref() {
                    let idx = COMPRESSORS.iter().position(|c| *c == zc.compressor).unwrap_or(0);
                    d.set_selected(idx as u32);
                }
                if let Some(s) = imp.pool_scale.borrow().as_ref() { s.set_value(zc.max_pool_percent as f64); }
                if let Some(s) = imp.threshold_scale.borrow().as_ref() { s.set_value(zc.accept_threshold_percent as f64); }
                if let Some(s) = imp.shrinker_switch.borrow().as_ref() { s.set_active(zc.shrinker_enabled); }
                if let Some(b) = imp.zswap_adv_box.borrow().as_ref() { b.set_visible(zc.enabled); }
                *imp.orig_compressor.borrow_mut() = zc.compressor;
                *imp.orig_pool.borrow_mut() = zc.max_pool_percent;
                *imp.orig_threshold.borrow_mut() = zc.accept_threshold_percent;
                *imp.orig_shrinker.borrow_mut() = zc.shrinker_enabled;
            }
        } else {
            if let Some(b) = imp.zswap_adv_box.borrow().as_ref() { b.set_visible(false); }
        }

        if let Some(r) = imp.apply_revealer.borrow().as_ref() { r.set_reveal_child(false); }
        *imp.refreshing.borrow_mut() = false;
    }
}

fn labeled_widget(title: &str, hint: &str, widget: &impl gtk::prelude::IsA<gtk::Widget>) -> gtk::Box {
    let b = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2).build();
    b.append(&gtk::Label::builder().label(title).halign(gtk::Align::Start).build());
    b.append(&gtk::Label::builder().label(hint).css_classes(["caption"]).halign(gtk::Align::Start).build());
    b.append(widget);
    b
}
