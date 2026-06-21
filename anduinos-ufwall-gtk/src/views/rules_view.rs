use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use crate::i18n::i18n;
use crate::ufw::types::UfwStatus;
use crate::widgets::rule_row::RuleRow;
use crate::widgets::add_rule_dialog::AddRuleDialog;

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct RulesView {
        pub stack: RefCell<Option<gtk::Stack>>,
        pub rules_group: RefCell<Option<adw::PreferencesGroup>>,
        pub added_rows: RefCell<Vec<gtk::Widget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RulesView {
        const NAME: &'static str = "RulesView";
        type Type = super::RulesView;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for RulesView {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }
    
    impl WidgetImpl for RulesView {}
    impl BinImpl for RulesView {}
}

glib::wrapper! {
    pub struct RulesView(ObjectSubclass<imp::RulesView>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RulesView {
    pub fn new() -> Self {
        glib::Object::builder()
            .build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        let page = adw::PreferencesPage::new();
        let rules_group = adw::PreferencesGroup::builder()
            .title(i18n("Firewall Rules"))
            .build();
        page.add(&rules_group);
        
        let empty_state = adw::StatusPage::builder()
            .title(i18n("No Rules"))
            .description(i18n("There are no rules configured yet."))
            .icon_name("network-firewall-symbolic")
            .build();

        let add_btn = gtk::Button::builder()
            .label(i18n("Add Rule"))
            .css_classes(["suggested-action", "pill"])
            .halign(gtk::Align::Center)
            .margin_top(12)
            .build();

        let weak_self = self.downgrade();
        add_btn.connect_clicked(move |_| {
            if let Some(view) = weak_self.upgrade() {
                if let Some(root) = view.root() {
                    let dialog = AddRuleDialog::new();
                    dialog.present(Some(&root));
                }
            }
        });

        rules_group.set_header_suffix(Some(&add_btn));

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
            
        stack.add_named(&page, Some("rules"));
        stack.add_named(&empty_state, Some("empty"));

        self.set_child(Some(&stack));

        *imp.rules_group.borrow_mut() = Some(rules_group);
        *imp.stack.borrow_mut() = Some(stack);
    }

    pub fn update(&self, status: &UfwStatus) {
        let imp = self.imp();
        self.set_sensitive(status.active);

        if let Some(group) = imp.rules_group.borrow().as_ref() {
            for row in imp.added_rows.borrow().iter() {
                group.remove(row);
            }
            imp.added_rows.borrow_mut().clear();

            if status.rules.is_empty() {
                if let Some(stack) = imp.stack.borrow().as_ref() {
                    stack.set_visible_child_name("empty");
                }
            } else {
                if let Some(stack) = imp.stack.borrow().as_ref() {
                    stack.set_visible_child_name("rules");
                }

                let mut added = imp.added_rows.borrow_mut();
                for rule in &status.rules {
                    let row = RuleRow::new(rule);
                    group.add(&row);
                    added.push(row.upcast::<gtk::Widget>());
                }
            }
        }
    }
}

impl Default for RulesView {
    fn default() -> Self {
        Self::new()
    }
}
