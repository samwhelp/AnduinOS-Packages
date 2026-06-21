use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use std::cell::Cell;
use std::rc::Rc;

use crate::i18n::i18n;
use crate::ufw::types::{UfwStatus, Policy, Direction};
use crate::ufw::backend;

mod imp {
    use super::*;
    use std::cell::RefCell;

    pub struct StatusView {
        pub active_switch: RefCell<Option<adw::SwitchRow>>,
        pub policy_group: RefCell<Option<adw::PreferencesGroup>>,
        pub incoming_combo: RefCell<Option<adw::ComboRow>>,
        pub outgoing_combo: RefCell<Option<adw::ComboRow>>,
        pub logging_combo: RefCell<Option<adw::ComboRow>>,
        pub is_updating: Rc<Cell<bool>>,
    }

    impl Default for StatusView {
        fn default() -> Self {
            Self {
                active_switch: Default::default(),
                policy_group: Default::default(),
                incoming_combo: Default::default(),
                outgoing_combo: Default::default(),
                logging_combo: Default::default(),
                is_updating: Rc::new(Cell::new(false)),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StatusView {
        const NAME: &'static str = "StatusView";
        type Type = super::StatusView;
        type ParentType = adw::PreferencesPage;
    }

    impl ObjectImpl for StatusView {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }
    
    impl WidgetImpl for StatusView {}
    impl PreferencesPageImpl for StatusView {}
}

glib::wrapper! {
    pub struct StatusView(ObjectSubclass<imp::StatusView>)
        @extends gtk::Widget, adw::PreferencesPage,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl StatusView {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("title", i18n("Status"))
            .property("icon-name", "security-high-symbolic")
            .build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        let status_group = adw::PreferencesGroup::builder().build();
        
        let active_switch = adw::SwitchRow::builder()
            .title(i18n("Firewall"))
            .subtitle(i18n("Inactive"))
            .build();
            
        status_group.add(&active_switch);
        self.add(&status_group);

        let policy_group = adw::PreferencesGroup::builder()
            .title(i18n("Default Policies"))
            .sensitive(false)
            .build();
            
        let incoming_combo = adw::ComboRow::builder()
            .title(i18n("Incoming"))
            .model(&gtk::StringList::new(&[
                &i18n("Deny"),
                &i18n("Allow"),
                &i18n("Reject"),
            ]))
            .build();
            
        let outgoing_combo = adw::ComboRow::builder()
            .title(i18n("Outgoing"))
            .model(&gtk::StringList::new(&[
                &i18n("Deny"),
                &i18n("Allow"),
                &i18n("Reject"),
            ]))
            .build();
            
        policy_group.add(&incoming_combo);
        policy_group.add(&outgoing_combo);
        self.add(&policy_group);

        let logging_group = adw::PreferencesGroup::builder()
            .title(i18n("Logging"))
            .build();

        let logging_combo = adw::ComboRow::builder()
            .title(i18n("Log Level"))
            .model(&gtk::StringList::new(&[
                &i18n("Off"),
                &i18n("Low"),
                &i18n("Medium"),
                &i18n("High"),
                &i18n("Full"),
            ]))
            .build();
            
        logging_group.add(&logging_combo);
        self.add(&logging_group);

        let is_updating = imp.is_updating.clone();

        active_switch.connect_active_notify(glib::clone!(
            #[weak] active_switch,
            #[strong] is_updating,
            move |_| {
                if is_updating.get() { return; }
                let active = active_switch.is_active();
                
                let switch_clone = active_switch.clone();
                glib::spawn_future_local(async move {
                    switch_clone.set_sensitive(false);
                    let result = tokio::task::spawn_blocking(move || {
                        backend::set_enabled(active)
                    }).await.unwrap();
                    
                    if let Err(e) = result {
                        eprintln!("Error toggling firewall: {}", e);
                    }
                    switch_clone.set_sensitive(true);
                });
            }
        ));

        incoming_combo.connect_selected_notify(glib::clone!(
            #[strong] is_updating,
            move |combo| {
                if is_updating.get() { return; }
                let policy = Policy::from_index(combo.selected());

                let combo_clone = combo.clone();
                glib::spawn_future_local(async move {
                    combo_clone.set_sensitive(false);
                    let _ = tokio::task::spawn_blocking(move || {
                        backend::set_default_policy(Direction::In, policy)
                    }).await.unwrap();
                    combo_clone.set_sensitive(true);
                });
            }
        ));

        outgoing_combo.connect_selected_notify(glib::clone!(
            #[strong] is_updating,
            move |combo| {
                if is_updating.get() { return; }
                let policy = Policy::from_index(combo.selected());

                let combo_clone = combo.clone();
                glib::spawn_future_local(async move {
                    combo_clone.set_sensitive(false);
                    let _ = tokio::task::spawn_blocking(move || {
                        backend::set_default_policy(Direction::Out, policy)
                    }).await.unwrap();
                    combo_clone.set_sensitive(true);
                });
            }
        ));

        logging_combo.connect_selected_notify(glib::clone!(
            #[strong] is_updating,
            move |combo| {
                if is_updating.get() { return; }
                let level = match combo.selected() {
                    0 => "off",
                    1 => "low",
                    2 => "medium",
                    3 => "high",
                    _ => "full",
                };
                
                let combo_clone = combo.clone();
                glib::spawn_future_local(async move {
                    combo_clone.set_sensitive(false);
                    let _ = tokio::task::spawn_blocking(move || {
                        backend::set_logging(level)
                    }).await.unwrap();
                    combo_clone.set_sensitive(true);
                });
            }
        ));

        *imp.active_switch.borrow_mut() = Some(active_switch);
        *imp.policy_group.borrow_mut() = Some(policy_group);
        *imp.incoming_combo.borrow_mut() = Some(incoming_combo);
        *imp.outgoing_combo.borrow_mut() = Some(outgoing_combo);
        *imp.logging_combo.borrow_mut() = Some(logging_combo);
    }

    pub fn update(&self, status: &UfwStatus) {
        let imp = self.imp();
        imp.is_updating.set(true);

        if let Some(switch) = imp.active_switch.borrow().as_ref() {
            switch.set_active(status.active);
            if status.active {
                switch.set_subtitle(&i18n("Active"));
            } else {
                switch.set_subtitle(&i18n("Inactive"));
            }
        }

        if let Some(group) = imp.policy_group.borrow().as_ref() {
            group.set_sensitive(status.active);
        }

        if let Some(combo) = imp.incoming_combo.borrow().as_ref() {
            combo.set_selected(status.default_incoming.index());
        }

        if let Some(combo) = imp.outgoing_combo.borrow().as_ref() {
            combo.set_selected(status.default_outgoing.index());
        }

        if let Some(combo) = imp.logging_combo.borrow().as_ref() {
            let idx = match status.logging.to_lowercase().as_str() {
                "low" => 1,
                "medium" => 2,
                "high" => 3,
                "full" => 4,
                _ => 0, // off
            };
            combo.set_selected(idx);
            combo.set_sensitive(status.active);
        }

        imp.is_updating.set(false);
    }
}
