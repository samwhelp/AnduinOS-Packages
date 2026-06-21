use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use crate::ufw::types::UfwRule;
use crate::ufw::backend;

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct RuleRow {
        pub rule_number: RefCell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RuleRow {
        const NAME: &'static str = "RuleRow";
        type Type = super::RuleRow;
        type ParentType = adw::ActionRow;
    }

    impl ObjectImpl for RuleRow {}
    impl WidgetImpl for RuleRow {}
    impl ListBoxRowImpl for RuleRow {}
    impl PreferencesRowImpl for RuleRow {}
    impl ActionRowImpl for RuleRow {}
}

glib::wrapper! {
    pub struct RuleRow(ObjectSubclass<imp::RuleRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl RuleRow {
    pub fn new(rule: &UfwRule) -> Self {
        let obj: Self = glib::Object::builder()
            .property("title", &rule.title())
            .property("subtitle", &rule.subtitle())
            .build();

        *obj.imp().rule_number.borrow_mut() = rule.number;

        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["destructive-action"])
            .build();

        let num = rule.number;
        delete_btn.connect_clicked(move |btn| {
            btn.set_sensitive(false);
            glib::spawn_future_local(async move {
                let _ = tokio::task::spawn_blocking(move || {
                    backend::delete_rule(num)
                }).await.unwrap();
                // Monitor will pick up changes, or button stays disabled until UI refresh removes the row
            });
        });

        obj.add_suffix(&delete_btn);
        obj
    }
}
