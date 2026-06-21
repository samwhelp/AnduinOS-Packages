use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use std::cell::RefCell;

use crate::i18n::i18n;
use crate::ufw::types::{Action, Direction, Protocol, RuleParams};
use crate::ufw::backend;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct AddRuleDialog {
        pub preset_combo: RefCell<Option<adw::ComboRow>>,
        pub port_entry: RefCell<Option<adw::EntryRow>>,
        pub action_combo: RefCell<Option<adw::ComboRow>>,
        pub dir_combo: RefCell<Option<adw::ComboRow>>,
        pub proto_combo: RefCell<Option<adw::ComboRow>>,
        pub source_entry: RefCell<Option<adw::EntryRow>>,
        pub dest_entry: RefCell<Option<adw::EntryRow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AddRuleDialog {
        const NAME: &'static str = "AddRuleDialog";
        type Type = super::AddRuleDialog;
        type ParentType = adw::Dialog;
    }

    impl ObjectImpl for AddRuleDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }
    
    impl WidgetImpl for AddRuleDialog {}
    impl AdwDialogImpl for AddRuleDialog {}
}

glib::wrapper! {
    pub struct AddRuleDialog(ObjectSubclass<imp::AddRuleDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl AddRuleDialog {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("title", i18n("Add Rule"))
            .property("content-width", 400)
            .build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        let page = adw::PreferencesPage::new();
        
        let basic_group = adw::PreferencesGroup::builder().build();

        let preset_combo = adw::ComboRow::builder()
            .title(i18n("Service"))
            .model(&gtk::StringList::new(&[
                &i18n("Custom"),
                "SSH (22)",
                "HTTP (80)",
                "HTTPS (443)",
                "FTP (21)",
                "SMTP (25)",
                "POP3 (110)",
                "IMAP (143)",
                "DNS (53)",
                "MySQL (3306)",
                "PostgreSQL (5432)",
                "RDP (3389)",
                "VNC (5900)",
            ]))
            .build();

        let port_entry = adw::EntryRow::builder()
            .title(i18n("Port / Service Name"))
            .build();
            
        let action_combo = adw::ComboRow::builder()
            .title(i18n("Action"))
            .model(&gtk::StringList::new(&[
                &i18n("Allow"),
                &i18n("Deny"),
                &i18n("Reject"),
                &i18n("Limit"),
            ]))
            .build();

        basic_group.add(&preset_combo);
        basic_group.add(&port_entry);
        basic_group.add(&action_combo);
        page.add(&basic_group);

        let adv_group = adw::PreferencesGroup::builder().build();
        let expander = adw::ExpanderRow::builder()
            .title(i18n("Advanced"))
            .build();

        let dir_combo = adw::ComboRow::builder()
            .title(i18n("Direction"))
            .model(&gtk::StringList::new(&[
                &i18n("In"),
                &i18n("Out"),
            ]))
            .build();
            
        let proto_combo = adw::ComboRow::builder()
            .title(i18n("Protocol"))
            .model(&gtk::StringList::new(&[
                &i18n("Both"),
                &i18n("TCP"),
                &i18n("UDP"),
            ]))
            .build();

        let source_entry = adw::EntryRow::builder()
            .title(i18n("Source IP"))
            .build();

        let dest_entry = adw::EntryRow::builder()
            .title(i18n("Destination IP"))
            .build();

        expander.add_row(&dir_combo);
        expander.add_row(&proto_combo);
        expander.add_row(&source_entry);
        expander.add_row(&dest_entry);
        adv_group.add(&expander);
        page.add(&adv_group);

        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();

        let cancel_btn = gtk::Button::with_label(&i18n("Cancel"));
        let add_btn = gtk::Button::builder()
            .label(i18n("Add"))
            .css_classes(["suggested-action"])
            .build();

        header.pack_start(&cancel_btn);
        header.pack_end(&add_btn);

        let toolbar = adw::ToolbarView::builder()
            .content(&page)
            .build();
        toolbar.add_top_bar(&header);

        self.set_child(Some(&toolbar));

        *imp.preset_combo.borrow_mut() = Some(preset_combo.clone());
        *imp.port_entry.borrow_mut() = Some(port_entry.clone());
        *imp.action_combo.borrow_mut() = Some(action_combo.clone());
        *imp.dir_combo.borrow_mut() = Some(dir_combo.clone());
        *imp.proto_combo.borrow_mut() = Some(proto_combo.clone());
        *imp.source_entry.borrow_mut() = Some(source_entry.clone());
        *imp.dest_entry.borrow_mut() = Some(dest_entry.clone());

        preset_combo.connect_selected_notify(glib::clone!(
            #[weak] port_entry,
            move |combo| {
                let text = match combo.selected() {
                    0 => "",
                    1 => "22",
                    2 => "80",
                    3 => "443",
                    4 => "21",
                    5 => "25",
                    6 => "110",
                    7 => "143",
                    8 => "53",
                    9 => "3306",
                    10 => "5432",
                    11 => "3389",
                    12 => "5900",
                    _ => "",
                };
                if combo.selected() != 0 {
                    port_entry.set_text(text);
                    port_entry.set_visible(false);
                } else {
                    port_entry.set_visible(true);
                }
            }
        ));

        let weak_self = self.downgrade();
        cancel_btn.connect_clicked(move |_| {
            if let Some(dialog) = weak_self.upgrade() {
                dialog.close();
            }
        });

        let weak_self2 = self.downgrade();
        add_btn.connect_clicked(move |btn| {
            if let Some(dialog) = weak_self2.upgrade() {
                dialog.on_add_clicked(btn);
            }
        });
    }

    fn on_add_clicked(&self, btn: &gtk::Button) {
        let imp = self.imp();
        
        let port = imp.port_entry.borrow().as_ref().unwrap().text().to_string();
        if port.trim().is_empty() {
            return;
        }

        let action = Action::from_index(imp.action_combo.borrow().as_ref().unwrap().selected());
        
        let dir_combo = imp.dir_combo.borrow();
        let direction = Some(match dir_combo.as_ref().unwrap().selected() {
            0 => Direction::In,
            _ => Direction::Out,
        });

        let proto_combo = imp.proto_combo.borrow();
        let protocol = Some(Protocol::from_index(proto_combo.as_ref().unwrap().selected()));

        let from_text = imp.source_entry.borrow().as_ref().unwrap().text().to_string();
        let from = if from_text.trim().is_empty() { None } else { Some(from_text) };

        let to_text = imp.dest_entry.borrow().as_ref().unwrap().text().to_string();
        let to = if to_text.trim().is_empty() { None } else { Some(to_text) };

        let params = RuleParams {
            port,
            action,
            direction,
            protocol,
            from,
            to,
        };

        btn.set_sensitive(false);
        let weak_self = self.downgrade();
        
        glib::spawn_future_local(async move {
            let result = tokio::task::spawn_blocking(move || {
                backend::add_rule(&params)
            }).await.unwrap();
            
            if let Some(dialog) = weak_self.upgrade() {
                match result {
                    Ok(_) => { dialog.close(); }
                    Err(e) => {
                        eprintln!("Failed to add rule: {}", e);
                        dialog.close(); 
                    }
                }
            }
        });
    }
}

impl Default for AddRuleDialog {
    fn default() -> Self {
        Self::new()
    }
}
