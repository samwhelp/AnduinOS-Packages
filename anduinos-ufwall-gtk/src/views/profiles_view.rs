use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use std::cell::RefCell;

use crate::i18n::i18n;
use crate::ufw::types::{AppProfile, UfwRule};
use crate::ufw::backend;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct ProfilesView {
        pub stack: RefCell<Option<gtk::Stack>>,
        pub profiles_group: RefCell<Option<adw::PreferencesGroup>>,
        pub added_rows: RefCell<Vec<gtk::Widget>>,
        pub current_rules: RefCell<Vec<UfwRule>>,
        pub is_updating: RefCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProfilesView {
        const NAME: &'static str = "ProfilesView";
        type Type = super::ProfilesView;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for ProfilesView {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_ui();
        }
    }
    
    impl WidgetImpl for ProfilesView {}
    impl BinImpl for ProfilesView {}
}

glib::wrapper! {
    pub struct ProfilesView(ObjectSubclass<imp::ProfilesView>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ProfilesView {
    pub fn new() -> Self {
        glib::Object::builder()
            .build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        let page = adw::PreferencesPage::new();
        let profiles_group = adw::PreferencesGroup::builder()
            .title(i18n("Application Profiles"))
            .description(i18n("Allow applications with predefined rules."))
            .build();
        page.add(&profiles_group);
        
        let empty_state = adw::StatusPage::builder()
            .title(i18n("No Profiles"))
            .description(i18n("No application profiles found in /etc/ufw/applications.d/"))
            .icon_name("applications-system-symbolic")
            .build();

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
        stack.add_named(&page, Some("profiles"));
        stack.add_named(&empty_state, Some("empty"));

        self.set_child(Some(&stack));

        *imp.profiles_group.borrow_mut() = Some(profiles_group);
        *imp.stack.borrow_mut() = Some(stack);
    }

    pub fn update_rules(&self, rules: &[UfwRule]) {
        let imp = self.imp();
        *imp.current_rules.borrow_mut() = rules.to_vec();
    }

    pub fn update_profiles(&self, profiles: Vec<AppProfile>) {
        let imp = self.imp();
        *imp.is_updating.borrow_mut() = true;

        if let Some(group) = imp.profiles_group.borrow().as_ref() {
            for row in imp.added_rows.borrow().iter() {
                group.remove(row);
            }
            imp.added_rows.borrow_mut().clear();

            if profiles.is_empty() {
                if let Some(stack) = imp.stack.borrow().as_ref() {
                    stack.set_visible_child_name("empty");
                }
            } else {
                if let Some(stack) = imp.stack.borrow().as_ref() {
                    stack.set_visible_child_name("profiles");
                }

                let rules = imp.current_rules.borrow();

                for profile in profiles {
                    let is_allowed = backend::is_app_allowed(&rules, &profile);
                    
                    let switch = adw::SwitchRow::builder()
                        .title(&profile.title)
                        .subtitle(&format!("{} ({})", profile.description, profile.ports))
                        .active(is_allowed)
                        .build();

                    let is_updating = imp.is_updating.clone();
                    let name = profile.name.clone();

                    switch.connect_active_notify(move |sw| {
                        if *is_updating.borrow() { return; }
                        let active = sw.is_active();
                        let name_clone = name.clone();
                        
                        let sw_clone = sw.clone();
                        glib::spawn_future_local(async move {
                            sw_clone.set_sensitive(false);
                            
                            let result = tokio::task::spawn_blocking(move || {
                                if active {
                                    backend::allow_app(&name_clone)
                                } else {
                                    backend::delete_app(&name_clone)
                                }
                            }).await.unwrap();
                            
                            if let Err(e) = result {
                                eprintln!("Error toggling profile: {}", e);
                            }
                            sw_clone.set_sensitive(true);
                        });
                    });

                    group.add(&switch);
                    imp.added_rows.borrow_mut().push(switch.upcast::<gtk::Widget>());
                }
            }
        }

        *imp.is_updating.borrow_mut() = false;
    }
}

impl Default for ProfilesView {
    fn default() -> Self {
        Self::new()
    }
}
