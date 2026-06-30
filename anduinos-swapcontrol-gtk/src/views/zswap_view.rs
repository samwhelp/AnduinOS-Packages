use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;

use crate::i18n::i18n;
use crate::swap::{zswap, persist};
use crate::utils;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ZswapView {
        pub status_icon: RefCell<Option<gtk::Image>>,
        pub status_label: RefCell<Option<gtk::Label>>,
        pub enabled_switch: RefCell<Option<gtk::Switch>>,
        pub compressor_combo: RefCell<Option<gtk::DropDown>>,
        pub compressor_list: RefCell<gtk::StringList>,
        pub pool_scale: RefCell<Option<gtk::Scale>>,
        pub threshold_scale: RefCell<Option<gtk::Scale>>,
        pub shrinker_switch: RefCell<Option<gtk::Switch>>,
        pub apply_btn: RefCell<Option<gtk::Button>>,
        pub apply_revealer: RefCell<Option<gtk::Revealer>>,
        pub spinner: RefCell<Option<gtk::Spinner>>,
        pub rec_label: RefCell<Option<gtk::Label>>,
        pub refreshing: RefCell<bool>,
        pub orig_pool: RefCell<u8>,
        pub orig_threshold: RefCell<u8>,
        pub orig_shrinker: RefCell<bool>,
        pub orig_compressor: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ZswapView {
        const NAME: &'static str = "ZswapView";
        type Type = super::ZswapView;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for ZswapView {
        fn constructed(&self) { self.parent_constructed(); self.obj().setup_ui(); }
    }
    impl WidgetImpl for ZswapView {}
    impl BoxImpl for ZswapView {}
}

glib::wrapper! {
    pub struct ZswapView(ObjectSubclass<imp::ZswapView>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl ZswapView {
    pub fn new() -> Self { glib::Object::builder().build() }

    fn setup_ui(&self) {
        let imp = self.imp();
        self.set_orientation(gtk::Orientation::Vertical);
        self.set_spacing(18);
        self.set_margin_start(24); self.set_margin_end(24);
        self.set_margin_top(24); self.set_margin_bottom(24);
        self.set_vexpand(true);

        self.append(&gtk::Label::builder().label(&i18n("Zswap Configuration"))
            .css_classes(["title-1"]).halign(gtk::Align::Start).build());
        self.append(&gtk::Label::builder()
            .label(&i18n("Compressed write-back cache — intercepts pages before disk swap"))
            .css_classes(["caption"]).halign(gtk::Align::Start).margin_start(2).build());

        let rec_label = gtk::Label::builder()
            .css_classes(["caption"]).halign(gtk::Align::Start).margin_start(2)
            .use_markup(true).build();
        self.append(&rec_label);
        *imp.rec_label.borrow_mut() = Some(rec_label);

        // Spinner
        let spinner = gtk::Spinner::builder().halign(gtk::Align::Center).visible(false).build();
        self.append(&spinner);
        *imp.spinner.borrow_mut() = Some(spinner);

        // ─── Status card with toggle ────────────────────────────────
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
        status_text.append(&gtk::Label::builder().label(&i18n("Compressed write-back cache for swap"))
            .css_classes(["caption"]).halign(gtk::Align::Start).build());
        status_inner.append(&status_text);
        *imp.status_label.borrow_mut() = Some(status_label);

        let enabled_switch = gtk::Switch::builder().valign(gtk::Align::Center).build();
        status_inner.append(&enabled_switch);
        status_card.append(&status_inner);
        self.append(&status_card);
        *imp.enabled_switch.borrow_mut() = Some(enabled_switch);

        // ─── Advanced expander ──────────────────────────────────────
        let expander = gtk::Expander::builder().label(&i18n("Advanced settings")).margin_top(6).build();
        let adv_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(16)
            .margin_start(12).margin_top(12).build();

        // Compressor
        let compressor_list = gtk::StringList::new(&["lzo"]);
        *imp.compressor_list.borrow_mut() = compressor_list;
        let compressor_combo = gtk::DropDown::builder().model(&*imp.compressor_list.borrow()).build();
        adv_box.append(&labeled_widget(&i18n("Compression algorithm"),
            &i18n("lzo-rle is fastest, zstd saves more space"),
            &compressor_combo));
        *imp.compressor_combo.borrow_mut() = Some(compressor_combo);

        // Max pool % slider
        let pool_scale = gtk::Scale::builder().orientation(gtk::Orientation::Horizontal)
            .adjustment(&gtk::Adjustment::new(20.0, 1.0, 50.0, 1.0, 5.0, 0.0))
            .draw_value(true).value_pos(gtk::PositionType::Right).hexpand(true).build();
        adv_box.append(&labeled_widget(&i18n("Max pool (% of RAM)"),
            &i18n("How much RAM zswap can use for compressed pages"),
            &pool_scale));
        *imp.pool_scale.borrow_mut() = Some(pool_scale);

        // Accept threshold % slider
        let threshold_scale = gtk::Scale::builder().orientation(gtk::Orientation::Horizontal)
            .adjustment(&gtk::Adjustment::new(90.0, 0.0, 100.0, 1.0, 5.0, 0.0))
            .draw_value(true).value_pos(gtk::PositionType::Right).hexpand(true).build();
        adv_box.append(&labeled_widget(&i18n("Accept threshold (%)"),
            &i18n("Pages compressing worse than this are rejected — lower = more aggressive"),
            &threshold_scale));
        *imp.threshold_scale.borrow_mut() = Some(threshold_scale);

        // Shrinker
        let shrinker_switch = gtk::Switch::builder().valign(gtk::Align::Center).build();
        let shrinker_row = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(12).build();
        let shrinker_text = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2).hexpand(true).build();
        shrinker_text.append(&gtk::Label::builder().label(&i18n("Memory shrinker")).halign(gtk::Align::Start).build());
        shrinker_text.append(&gtk::Label::builder().label(&i18n("Reclaim pool memory when system is under pressure"))
            .css_classes(["caption"]).halign(gtk::Align::Start).build());
        shrinker_row.append(&shrinker_text);
        shrinker_row.append(&shrinker_switch);
        adv_box.append(&shrinker_row);
        *imp.shrinker_switch.borrow_mut() = Some(shrinker_switch);

        expander.set_child(Some(&adv_box));
        self.append(&expander);

        // ─── Apply button (revealed on change) ──────────────────────
        let revealer = gtk::Revealer::builder().transition_type(gtk::RevealerTransitionType::SlideDown).build();
        let apply_btn = gtk::Button::builder().label(&i18n("Apply Settings"))
            .halign(gtk::Align::Center).css_classes(["suggested-action", "pill"]).build();
        revealer.set_child(Some(&apply_btn));
        self.append(&revealer);
        *imp.apply_btn.borrow_mut() = Some(apply_btn);
        *imp.apply_revealer.borrow_mut() = Some(revealer);

        self.connect_signals();
    }

    fn connect_signals(&self) {
        let imp = self.imp();

        // Enable/disable toggle — use notify::active (fires AFTER visual change)
        if let Some(sw) = imp.enabled_switch.borrow().as_ref() {
            let weak_self = self.downgrade();
            sw.connect_notify_local(Some("active"), move |sw, _| {
                if let Some(view) = weak_self.upgrade() {
                    if *view.imp().refreshing.borrow() { return; }
                    view.set_enabled(sw.is_active());
                }
            });
        }

        // Apply button
        if let Some(btn) = imp.apply_btn.borrow().as_ref() {
            let weak_self = self.downgrade();
            btn.connect_clicked(move |_| {
                if let Some(view) = weak_self.upgrade() {
                    view.apply_all_settings();
                }
            });
        }

        // Track changes on advanced controls (skip during refresh to avoid false triggers)
        let weak_self = self.downgrade();
        let check = move || { if let Some(v) = weak_self.upgrade() { if !*v.imp().refreshing.borrow() { v.check_changes(); } } };

        if let Some(s) = imp.pool_scale.borrow().as_ref() {
            let c = check.clone(); s.connect_value_changed(move |_| c());
        }
        if let Some(s) = imp.threshold_scale.borrow().as_ref() {
            let c = check.clone(); s.connect_value_changed(move |_| c());
        }
        if let Some(sw) = imp.shrinker_switch.borrow().as_ref() {
            let c = check.clone();
            sw.connect_notify_local(Some("active"), move |_, _| { c(); });
        }
        if let Some(cb) = imp.compressor_combo.borrow().as_ref() {
            let c = check; cb.connect_selected_item_notify(move |_| c());
        }
    }

    fn check_changes(&self) {
        let imp = self.imp();
        let pool = imp.pool_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(20);
        let thresh = imp.threshold_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(90);
        let shrink = imp.shrinker_switch.borrow().as_ref().map(|s| s.is_active()).unwrap_or(true);
        let comp_idx = imp.compressor_combo.borrow().as_ref().map(|c| c.selected()).unwrap_or(0);
        let comp = imp.compressor_list.borrow().string(comp_idx).map(|s| s.to_string()).unwrap_or_default();

        let changed = pool != *imp.orig_pool.borrow()
            || thresh != *imp.orig_threshold.borrow()
            || shrink != *imp.orig_shrinker.borrow()
            || comp != *imp.orig_compressor.borrow();

        if let Some(r) = imp.apply_revealer.borrow().as_ref() {
            r.set_reveal_child(changed);
        }
    }

    fn set_busy(&self, busy: bool) {
        let imp = self.imp();
        if let Some(s) = imp.spinner.borrow().as_ref() { s.set_visible(busy); }
        if let Some(b) = imp.apply_btn.borrow().as_ref() { b.set_sensitive(!busy); }
        // Don't gray out the switch — spinner is enough visual feedback
    }

    fn show_error(&self, msg: &str) {
        let win = self.root().and_then(|r| r.downcast::<gtk::Window>().ok());
        if let Some(p) = win { utils::show_error(&p, &i18n("Error"), msg); }
    }

    fn set_enabled(&self, enabled: bool) {
        let weak_self = self.downgrade();
        self.set_busy(true);
        glib::spawn_future_local(async move {
            let result = if enabled { zswap::enable_zswap() } else { zswap::disable_zswap() };
            if result.is_ok() {
                if enabled {
                    // Persist with current (still-kernel-default or user-set) values
                    if let Ok(cfg) = zswap::read_zswap_config() {
                        let _ = persist::persist_zswap(true, &cfg.compressor, cfg.max_pool_percent, cfg.accept_threshold_percent, cfg.shrinker_enabled);
                    }
                } else {
                    let _ = persist::remove_zswap_persistence();
                }
            }
            if let Some(view) = weak_self.upgrade() {
                view.set_busy(false); view.refresh_all();
                if let Err(e) = result { view.show_error(&e); }
            }
        });
    }

    fn apply_all_settings(&self) {
        let imp = self.imp();
        let comp_idx = imp.compressor_combo.borrow().as_ref().map(|c| c.selected()).unwrap_or(0);
        let compressor = imp.compressor_list.borrow().string(comp_idx).map(|s| s.to_string()).unwrap_or_else(|| "lzo".into());
        let pool = imp.pool_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(20);
        let thresh = imp.threshold_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(90);
        let shrink = imp.shrinker_switch.borrow().as_ref().map(|s| s.is_active()).unwrap_or(true);
        let enabled = imp.enabled_switch.borrow().as_ref().map(|s| s.is_active()).unwrap_or(false);
        let weak_self = self.downgrade();
        self.set_busy(true);

        glib::spawn_future_local(async move {
            let mut errs = Vec::new();
            if let Err(e) = zswap::set_compressor(&compressor) { errs.push(format!("Compressor: {}", e)); }
            if let Err(e) = zswap::set_max_pool_percent(pool) { errs.push(format!("Pool: {}", e)); }
            if let Err(e) = zswap::set_accept_threshold(thresh) { errs.push(format!("Threshold: {}", e)); }
            if let Err(e) = zswap::set_shrinker(shrink) { errs.push(format!("Shrinker: {}", e)); }
            if let Err(e) = persist::persist_zswap(enabled, &compressor, pool, thresh, shrink) { errs.push(format!("Persist: {}", e)); }

            if let Some(view) = weak_self.upgrade() {
                view.set_busy(false); view.refresh_all();
                if !errs.is_empty() { view.show_error(&errs.join("\n")); }
                // Hide apply button after save
                if let Some(r) = view.imp().apply_revealer.borrow().as_ref() { r.set_reveal_child(false); }
            }
        });
    }

    fn refresh_all(&self) {
        self.refresh_data();
        if let Some(root) = self.root() {
            if let Ok(win) = root.downcast::<crate::window::SwapcontrolWindow>() { win.refresh_views(); }
        }
    }

    pub fn refresh_data(&self) {
        let imp = self.imp();
        *imp.refreshing.borrow_mut() = true;

        // Recommendation label
        if let Some(l) = imp.rec_label.borrow().as_ref() {
            let total_ram = crate::swap::sysctl::read_total_ram().unwrap_or(32 * 1024 * 1024 * 1024);
            let ram_gb = total_ram as f64 / (1024.0 * 1024.0 * 1024.0);
            let (pool, thresh) = zswap_recommend(ram_gb);
            l.set_markup(&format!(
                "<i>Recommended: lzo-rle, pool {}%, threshold {}% (for {:.0} GiB RAM)</i>",
                pool, thresh, ram_gb
            ));
        }

        // Update compressor list
        let compressors = zswap::get_available_compressors();
        let need_update = {
            let cl = imp.compressor_list.borrow();
            compressors.len() != cl.n_items() as usize
        };
        if need_update {
            let nl = gtk::StringList::new(&compressors.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            if let Some(cb) = imp.compressor_combo.borrow().as_ref() { cb.set_model(Some(&nl)); }
            *imp.compressor_list.borrow_mut() = nl;
        }

        if let Ok(cfg) = zswap::read_zswap_config() {
            // Status display
            let on_str = i18n("Active");
            let off_str = i18n("Inactive");
            if let Some(l) = imp.status_label.borrow().as_ref() {
                l.set_text(if cfg.enabled { &on_str } else { &off_str });
            }
            if let Some(icon) = imp.status_icon.borrow().as_ref() {
                icon.set_icon_name(if cfg.enabled { Some("emblem-ok-symbolic") } else { Some("emblem-important-symbolic") });
            }
            if let Some(sw) = imp.enabled_switch.borrow().as_ref() { sw.set_active(cfg.enabled); }
            if let Some(cb) = imp.compressor_combo.borrow().as_ref() {
                if let Some(pos) = compressors.iter().position(|a| a == &cfg.compressor) { cb.set_selected(pos as u32); }
            }
            if let Some(s) = imp.pool_scale.borrow().as_ref() { s.set_value(cfg.max_pool_percent as f64); }
            if let Some(s) = imp.threshold_scale.borrow().as_ref() { s.set_value(cfg.accept_threshold_percent as f64); }
            if let Some(sw) = imp.shrinker_switch.borrow().as_ref() { sw.set_active(cfg.shrinker_enabled); }

            // Cache originals for change detection
            *imp.orig_pool.borrow_mut() = cfg.max_pool_percent;
            *imp.orig_threshold.borrow_mut() = cfg.accept_threshold_percent;
            *imp.orig_shrinker.borrow_mut() = cfg.shrinker_enabled;
            *imp.orig_compressor.borrow_mut() = cfg.compressor.clone();

            // Hide apply button after refresh (values are now "original")
            if let Some(r) = imp.apply_revealer.borrow().as_ref() { r.set_reveal_child(false); }
        }

        *imp.refreshing.borrow_mut() = false;
    }
}

/// Recommend zswap pool % and accept threshold based on total RAM.
fn zswap_recommend(ram_gb: f64) -> (u8, u8) {
    if ram_gb <= 4.0 {
        (50, 70) // tiny: aggressive pool, aggressive threshold
    } else if ram_gb <= 8.0 {
        (30, 80) // small: moderate
    } else if ram_gb <= 16.0 {
        (25, 85) // medium
    } else if ram_gb <= 64.0 {
        (20, 90) // large: default kernel settings
    } else {
        (10, 90) // huge: barely need zswap
    }
}

// ─── Helper ──────────────────────────────────────────────────────────────────

fn labeled_widget(title: &str, hint: &str, widget: &impl gtk::prelude::IsA<gtk::Widget>) -> gtk::Box {
    let box_ = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(2).build();
    box_.append(&gtk::Label::builder().label(title).halign(gtk::Align::Start).build());
    box_.append(&gtk::Label::builder().label(hint).css_classes(["caption"]).halign(gtk::Align::Start).build());
    box_.append(widget);
    box_
}
