use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use std::cell::RefCell;

use crate::i18n::i18n;
use crate::ufw::stats;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct DashboardView {
        pub status_label: RefCell<Option<gtk::Label>>,
        pub blocked_count_label: RefCell<Option<gtk::Label>>,
        pub top_ips_group: RefCell<Option<adw::PreferencesGroup>>,
        pub top_ports_group: RefCell<Option<adw::PreferencesGroup>>,
        pub listening_group: RefCell<Option<adw::PreferencesGroup>>,
        pub log_list: RefCell<Option<gtk::ListBox>>,
        pub log_count_combo: RefCell<Option<adw::ComboRow>>,
        pub refresh_btn: RefCell<Option<gtk::Button>>,
        // Track added rows for each group so we can remove them on refresh
        pub top_ips_rows: RefCell<Vec<gtk::Widget>>,
        pub top_ports_rows: RefCell<Vec<gtk::Widget>>,
        pub listening_rows: RefCell<Vec<gtk::Widget>>,
        pub log_rows: RefCell<Vec<gtk::Widget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DashboardView {
        const NAME: &'static str = "DashboardView";
        type Type = super::DashboardView;
        type ParentType = adw::PreferencesPage;
    }

    impl ObjectImpl for DashboardView {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }

    impl WidgetImpl for DashboardView {}
    impl PreferencesPageImpl for DashboardView {}
}

glib::wrapper! {
    pub struct DashboardView(ObjectSubclass<imp::DashboardView>)
        @extends gtk::Widget, adw::PreferencesPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DashboardView {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("title", i18n("Dashboard"))
            .property("icon-name", "network-firewall-symbolic")
            .build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        // Status banner
        let status_group = adw::PreferencesGroup::builder().build();
        let status_label = gtk::Label::builder()
            .label(i18n("Loading..."))
            .halign(gtk::Align::Start)
            .build();
        status_label.add_css_class("title-4");
        status_group.add(&status_label);
        self.add(&status_group);

        // Summary group
        let summary_group = adw::PreferencesGroup::builder()
            .title(i18n("Summary"))
            .build();
        let blocked_count_label = gtk::Label::builder()
            .label(i18n("Loading blocked events..."))
            .halign(gtk::Align::Start)
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .margin_bottom(6)
            .build();
        summary_group.add(&blocked_count_label);
        self.add(&summary_group);

        // Top blocked IPs
        let top_ips_group = adw::PreferencesGroup::builder()
            .title(i18n("Top Blocked Source IPs"))
            .build();
        self.add(&top_ips_group);

        // Top targeted ports
        let top_ports_group = adw::PreferencesGroup::builder()
            .title(i18n("Top Targeted Ports"))
            .build();
        self.add(&top_ports_group);

        // Listening ports
        let listening_group = adw::PreferencesGroup::builder()
            .title(i18n("Listening Ports"))
            .build();
        self.add(&listening_group);

        // Log viewer
        let log_group = adw::PreferencesGroup::builder()
            .title(i18n("Recent Blocked Events"))
            .build();

        let log_count_combo = adw::ComboRow::builder()
            .title(i18n("Show"))
            .model(&gtk::StringList::new(&["50", "100", "500", "1000"]))
            .build();
        log_count_combo.set_selected(0);
        log_group.add(&log_count_combo);

        let log_list = gtk::ListBox::builder()
            .css_classes(["boxed-list"])
            .build();
        log_group.add(&log_list);

        self.add(&log_group);

        // Refresh button (wrapped in a group)
        let refresh_btn = gtk::Button::builder()
            .label(i18n("Refresh"))
            .halign(gtk::Align::Center)
            .margin_top(12)
            .margin_bottom(12)
            .build();
        let refresh_group = adw::PreferencesGroup::builder().build();
        refresh_group.add(&refresh_btn);
        self.add(&refresh_group);

        // Connect signals
        let weak_self = self.downgrade();
        log_count_combo.connect_selected_notify(move |_| {
            if let Some(view) = weak_self.upgrade() {
                view.refresh_data();
            }
        });

        let weak_self = self.downgrade();
        refresh_btn.connect_clicked(move |_| {
            if let Some(view) = weak_self.upgrade() {
                view.refresh_data();
            }
        });

        *imp.status_label.borrow_mut() = Some(status_label);
        *imp.blocked_count_label.borrow_mut() = Some(blocked_count_label);
        *imp.top_ips_group.borrow_mut() = Some(top_ips_group);
        *imp.top_ports_group.borrow_mut() = Some(top_ports_group);
        *imp.listening_group.borrow_mut() = Some(listening_group);
        *imp.log_list.borrow_mut() = Some(log_list);
        *imp.log_count_combo.borrow_mut() = Some(log_count_combo);
        *imp.refresh_btn.borrow_mut() = Some(refresh_btn);
    }

    pub fn refresh_data(&self) {
        let imp = self.imp();
        let limit = match imp.log_count_combo.borrow().as_ref().unwrap().selected() {
            0 => 50,
            1 => 100,
            2 => 500,
            _ => 1000,
        };

        if let Some(btn) = imp.refresh_btn.borrow().as_ref() {
            btn.set_sensitive(false);
        }

        let weak_self = self.downgrade();
        glib::spawn_future_local(async move {
            let events = tokio::task::spawn_blocking(move || {
                stats::read_blocked_events(limit)
            })
            .await
            .unwrap();

            let listening = tokio::task::spawn_blocking(|| stats::read_listening_ports())
                .await
                .unwrap();

            if let Some(view) = weak_self.upgrade() {
                view.populate(events, listening);
            }
        });
    }

    fn populate(
        &self,
        events: Result<Vec<stats::BlockedEvent>, crate::ufw::types::UfwError>,
        listening: Result<Vec<stats::ListeningPort>, crate::ufw::types::UfwError>,
    ) {
        let imp = self.imp();

        let events = events.unwrap_or_default();
        let listening = listening.unwrap_or_default();
        let blocked_count = events.iter().filter(|e| e.action == "BLOCK").count();

        // Status label
        if let Some(label) = imp.status_label.borrow().as_ref() {
            if blocked_count > 0 {
                label.set_text(&format!(
                    "{} ({} {})",
                    i18n("Firewall Active"),
                    blocked_count,
                    i18n("recent blocked events")
                ));
            } else {
                label.set_text(&i18n("Firewall Active"));
            }
        }

        // Blocked count
        if let Some(label) = imp.blocked_count_label.borrow().as_ref() {
            label.set_text(&format!(
                "{}: {}  |  {}: {}",
                i18n("Recent events shown"),
                events.len(),
                i18n("Blocked"),
                blocked_count
            ));
        }

        // Top blocked IPs
        if let Some(group) = imp.top_ips_group.borrow().as_ref() {
            for row in imp.top_ips_rows.borrow().iter() {
                group.remove(row);
            }
            imp.top_ips_rows.borrow_mut().clear();

            let top = stats::top_blocked_ips(&events, 5);
            if top.is_empty() {
                let row = adw::ActionRow::builder()
                    .title(i18n("No blocked IPs in recent events"))
                    .build();
                group.add(&row);
                imp.top_ips_rows.borrow_mut().push(row.upcast::<gtk::Widget>());
            } else {
                for (ip, count) in &top {
                    let row = adw::ActionRow::builder()
                        .title(ip)
                        .subtitle(&format!("{}: {}", i18n("Blocked times"), count))
                        .build();
                    group.add(&row);
                    imp.top_ips_rows.borrow_mut().push(row.upcast::<gtk::Widget>());
                }
            }
        }

        // Top targeted ports
        if let Some(group) = imp.top_ports_group.borrow().as_ref() {
            for row in imp.top_ports_rows.borrow().iter() {
                group.remove(row);
            }
            imp.top_ports_rows.borrow_mut().clear();

            let top = stats::top_blocked_ports(&events, 5);
            if top.is_empty() {
                let row = adw::ActionRow::builder()
                    .title(i18n("No targeted ports in recent events"))
                    .build();
                group.add(&row);
                imp.top_ports_rows.borrow_mut().push(row.upcast::<gtk::Widget>());
            } else {
                for (port, count) in &top {
                    let row = adw::ActionRow::builder()
                        .title(port)
                        .subtitle(&format!("{}: {}", i18n("Blocked times"), count))
                        .build();
                    group.add(&row);
                    imp.top_ports_rows.borrow_mut().push(row.upcast::<gtk::Widget>());
                }
            }
        }

        // Listening ports
        if let Some(group) = imp.listening_group.borrow().as_ref() {
            for row in imp.listening_rows.borrow().iter() {
                group.remove(row);
            }
            imp.listening_rows.borrow_mut().clear();

            if listening.is_empty() {
                let row = adw::ActionRow::builder()
                    .title(i18n("No listening ports detected"))
                    .build();
                group.add(&row);
                imp.listening_rows.borrow_mut().push(row.upcast::<gtk::Widget>());
            } else {
                for lp in &listening {
                    let proc_display = if lp.process.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", lp.process)
                    };
                    let row = adw::ActionRow::builder()
                        .title(&format!("{}/{} {}{}", lp.port, lp.protocol, lp.local_address, proc_display))
                        .build();
                    group.add(&row);
                    imp.listening_rows.borrow_mut().push(row.upcast::<gtk::Widget>());
                }
            }
        }

        // Log viewer
        if let Some(list) = imp.log_list.borrow().as_ref() {
            for row in imp.log_rows.borrow().iter() {
                list.remove(row);
            }
            imp.log_rows.borrow_mut().clear();

            for event in &events {
                let row = adw::ActionRow::builder()
                    .title(&format!(
                        "[{}] {}:{} → {}:{} ({})",
                        event.action,
                        event.src_ip,
                        event.src_port,
                        event.dst_ip,
                        event.dst_port,
                        event.protocol
                    ))
                    .subtitle(&event.timestamp)
                    .build();
                list.append(&row);
                imp.log_rows.borrow_mut().push(row.upcast::<gtk::Widget>());
            }
        }

        // Re-enable refresh
        if let Some(btn) = imp.refresh_btn.borrow().as_ref() {
            btn.set_sensitive(true);
        }
    }
}
