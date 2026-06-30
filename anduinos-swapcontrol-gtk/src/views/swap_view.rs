use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;

use crate::i18n::i18n;
use crate::swap::{swapfile, sysctl, hibernation};
use crate::utils;
use crate::widgets::usage_bar::UsageBar;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct SwapView {
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
        self.set_spacing(18);
        self.set_margin_start(24); self.set_margin_end(24);
        self.set_margin_top(24); self.set_margin_bottom(24);
        self.set_vexpand(true);

        // Title + subtitle
        self.append(&gtk::Label::builder().label(&i18n("Disk Swap Configuration"))
            .css_classes(["title-1"]).halign(gtk::Align::Start).build());
        self.append(&gtk::Label::builder()
            .label(&i18n("Disk-based swap file — the last line of defense when RAM is full"))
            .css_classes(["caption"]).halign(gtk::Align::Start).margin_start(2).build());

        // Recommendation
        {
            let total_ram = sysctl::read_total_ram().unwrap_or(32 * 1024 * 1024 * 1024);
            let ram_gb = total_ram as f64 / (1024.0 * 1024.0 * 1024.0);
            let rec_size = (total_ram / (1024*1024*1024)).max(1);
            let sw = if ram_gb >= 16.0 { 10 } else if ram_gb >= 8.0 { 30 } else { 60 };
            self.append(&gtk::Label::builder().use_markup(true)
                .label(&{let s=format!("<i>Recommended: {} GiB swap, swappiness {} (for {:.0} GiB RAM)</i>", rec_size, sw, ram_gb); s})
                .css_classes(["caption"]).halign(gtk::Align::Start).margin_start(2).build());
        }

        // Spinner
        let spinner = gtk::Spinner::builder().halign(gtk::Align::Center).visible(false).build();
        self.append(&spinner);
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
        self.append(&status_card);
        *imp.enable_switch.borrow_mut() = Some(enable_switch);

        // Usage bar
        let usage_bar = UsageBar::new("Swap usage", (0.21, 0.52, 0.89));
        self.append(&usage_bar);
        *imp.usage_bar.borrow_mut() = Some(usage_bar);

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
        self.append(&size_box);
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

        expander.set_child(Some(&adv_box));
        self.append(&expander);

        // ─── Apply button (revealed on change) ─────────────────────
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

        // Track changes on advanced controls
        let weak_self = self.downgrade();
        let check = move || { if let Some(v) = weak_self.upgrade() { if !*v.imp().refreshing.borrow() { v.check_changes(); } } };
        if let Some(s) = imp.swappiness_scale.borrow().as_ref() { let c = check.clone(); s.connect_value_changed(move |_| c()); }
        if let Some(s) = imp.size_scale.borrow().as_ref() { let c = check; s.connect_value_changed(move |_| c()); }
    }

    fn check_changes(&self) {
        let imp = self.imp();
        let sw = imp.swappiness_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(10);
        let sz = imp.size_scale.borrow().as_ref().map(|s| s.value() as u64).unwrap_or(2);
        let changed = sw != *imp.orig_swap.borrow() || sz != *imp.orig_size.borrow();
        if let Some(r) = imp.apply_revealer.borrow().as_ref() { r.set_reveal_child(changed); }
    }

    fn set_busy(&self, busy: bool) {
        let imp = self.imp();
        if let Some(s) = imp.spinner.borrow().as_ref() { s.set_visible(busy); }
        if let Some(b) = imp.apply_btn.borrow().as_ref() { b.set_sensitive(!busy); }
    }

    fn toggle_swap(&self, enable: bool) {
        let is_active = swapfile::is_swap_active();
        if enable == is_active { return; }
        let weak_self = self.downgrade();
        let needs_confirm = is_active && hibernation::is_hibernation_configured();

        let do_toggle = {
            let weak_self = weak_self.clone();
            move || {
                let msg = if is_active { i18n("Disabling swap…") } else { i18n("Enabling swap…") };
                glib::spawn_future_local(async move {
                    let result = {
                        let weak = weak_self.clone();
                        let msg = msg.clone();
                        let win = weak.upgrade().and_then(|v| v.root().and_then(|r| r.downcast::<gtk::Window>().ok()));
                        if let Some(p) = win {
                            crate::progress_dialog::run_with_progress(&p, &msg, move || {
                                if is_active { swapfile::disable_swapfile() } else { swapfile::enable_swapfile() }
                            }).await
                        } else {
                            if is_active { swapfile::disable_swapfile() } else { swapfile::enable_swapfile() }
                        }
                    };
                    if let Some(view) = weak_self.upgrade() {
                        view.refresh_all();
                        if let Err(e) = result {
                            let win = view.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                            if let Some(p) = win { utils::show_error(&p, &i18n("Error"), &e); }
                        }
                    }
                });
            }
        };

        if needs_confirm {
            if let Some(view) = weak_self.upgrade() {
                let win = view.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                if let Some(p) = win {
                    utils::show_confirm(&p, &i18n("Hibernation Warning"),
                        &i18n("Disabling swap will break hibernation. Continue?"), do_toggle);
                }
            }
        } else { do_toggle(); }
    }

    fn apply_all(&self) {
        let imp = self.imp();
        let sw = imp.swappiness_scale.borrow().as_ref().map(|s| s.value() as u8).unwrap_or(10);
        let sz = imp.size_scale.borrow().as_ref().map(|s| s.value() as u64).unwrap_or(2);
        let weak_self = self.downgrade();

        let needs_warn = hibernation::is_hibernation_configured() && {
            let total_ram = sysctl::read_total_ram().unwrap_or(0);
            (sz * 1024 * 1024 * 1024) < total_ram
        };

        let do_apply = {
            let weak_self = weak_self.clone();
            move || {
                glib::spawn_future_local(async move {
                    let result = {
                        let weak = weak_self.clone();
                        let win = weak.upgrade().and_then(|v| v.root().and_then(|r| r.downcast::<gtk::Window>().ok()));
                        if let Some(p) = win {
                            crate::progress_dialog::run_with_progress(&p, &i18n("Resizing swap file…"), move || {
                                let mut errs = Vec::new();
                                if let Err(e) = sysctl::set_swappiness(sw) { errs.push(format!("Swappiness: {}", e)); }
                                if let Err(e) = swapfile::resize_swapfile(sz) { errs.push(format!("Resize: {}", e)); }
                                if errs.is_empty() { Ok(()) } else { Err(errs.join("\n")) }
                            }).await
                        } else {
                            let mut errs = Vec::new();
                            if let Err(e) = sysctl::set_swappiness(sw) { errs.push(format!("Swappiness: {}", e)); }
                            if let Err(e) = swapfile::resize_swapfile(sz) { errs.push(format!("Resize: {}", e)); }
                            if errs.is_empty() { Ok(()) } else { Err(errs.join("\n")) }
                        }
                    };
                    if let Some(view) = weak_self.upgrade() {
                        view.refresh_all();
                        if let Err(e) = result { let win = view.root().and_then(|r| r.downcast::<gtk::Window>().ok()); if let Some(p) = win { utils::show_error(&p, &i18n("Error"), &e); } }
                        if let Some(r) = view.imp().apply_revealer.borrow().as_ref() { r.set_reveal_child(false); }
                    }
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

    fn refresh_all(&self) {
        self.refresh_data();
        if let Some(root) = self.root() {
            if let Ok(win) = root.downcast::<crate::window::SwapcontrolWindow>() { win.refresh_views(); }
        }
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
