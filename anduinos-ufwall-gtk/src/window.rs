use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use gtk::gio;

use std::cell::RefCell;

use crate::application::UfwallApplication;
use crate::i18n::i18n;
use crate::views::{status_view::StatusView, rules_view::RulesView, profiles_view::ProfilesView};
use crate::ufw::monitor;
use crate::ufw::backend;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct UfwallWindow {
        pub split_view: RefCell<Option<adw::OverlaySplitView>>,
        pub stack: RefCell<Option<gtk::Stack>>,
        pub status_view: RefCell<Option<StatusView>>,
        pub rules_view: RefCell<Option<RulesView>>,
        pub profiles_view: RefCell<Option<ProfilesView>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UfwallWindow {
        const NAME: &'static str = "UfwallWindow";
        type Type = super::UfwallWindow;
        type ParentType = adw::ApplicationWindow;
    }

    impl ObjectImpl for UfwallWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_ui();
            obj.setup_monitoring();
        }
    }
    
    impl WidgetImpl for UfwallWindow {}
    impl WindowImpl for UfwallWindow {}
    impl ApplicationWindowImpl for UfwallWindow {}
    impl AdwApplicationWindowImpl for UfwallWindow {}
}

glib::wrapper! {
    pub struct UfwallWindow(ObjectSubclass<imp::UfwallWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable, 
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl UfwallWindow {
    pub fn new(app: &UfwallApplication) -> Self {
        glib::Object::builder()
            .property("application", app)
            .property("title", i18n("Firewall"))
            .property("default-width", 900)
            .property("default-height", 650)
            .property("icon-name", "com.anduinos.ufwall")
            .build()
    }

    fn setup_ui(&self) {
        let imp = self.imp();

        // Menu button (right)
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .build();
        menu_button.add_css_class("primary");
        
        let menu_model = gio::Menu::new();
        menu_model.append(Some(&i18n("About")), Some("app.about"));
        menu_button.set_menu_model(Some(&menu_model));

        // Sidebar setup
        let sidebar_list = gtk::ListBox::builder()
            .css_classes(["navigation-sidebar"])
            .selection_mode(gtk::SelectionMode::Single)
            .build();
        
        let status_row = gtk::ListBoxRow::builder().child(&gtk::Label::new(Some(&i18n("Status")))).build();
        let rules_row = gtk::ListBoxRow::builder().child(&gtk::Label::new(Some(&i18n("Rules")))).build();
        let profiles_row = gtk::ListBoxRow::builder().child(&gtk::Label::new(Some(&i18n("Profiles")))).build();

        sidebar_list.append(&status_row);
        sidebar_list.append(&rules_row);
        sidebar_list.append(&profiles_row);
        sidebar_list.select_row(Some(&status_row));

        let sidebar_header = adw::HeaderBar::builder()
            .show_end_title_buttons(false)
            .show_start_title_buttons(true)
            .build();

        let sidebar_toolbar = adw::ToolbarView::builder()
            .content(&sidebar_list)
            .build();
        sidebar_toolbar.add_top_bar(&sidebar_header);

        // Content Setup
        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
        
        let status_view = StatusView::new();
        let rules_view = RulesView::new();
        let profiles_view = ProfilesView::new();

        stack.add_named(&status_view, Some("status"));
        stack.add_named(&rules_view, Some("rules"));
        stack.add_named(&profiles_view, Some("profiles"));

        let content_header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(true)
            .build();
        content_header.pack_end(&menu_button);

        let content_toolbar = adw::ToolbarView::builder()
            .content(&stack)
            .build();
        content_toolbar.add_top_bar(&content_header);

        // Split view
        let split_view = adw::OverlaySplitView::builder()
            .sidebar(&sidebar_toolbar)
            .content(&content_toolbar)
            .max_sidebar_width(260.0)
            .min_sidebar_width(200.0)
            .build();

        // Connect hamburger menu (shows split view toggle in narrow mode)
        let toggle_button = gtk::ToggleButton::builder()
            .icon_name("sidebar-show-symbolic")
            .build();
        content_header.pack_start(&toggle_button);

        split_view.bind_property("show-sidebar", &toggle_button, "active")
            .sync_create()
            .bidirectional()
            .build();

        // Only show hamburger when collapsed
        split_view.bind_property("collapsed", &toggle_button, "visible")
            .sync_create()
            .build();
        
        // Disable title buttons on sidebar when collapsed
        split_view.bind_property("collapsed", &sidebar_header, "show-start-title-buttons")
            .sync_create()
            .invert_boolean()
            .build();

        // Connect list box to stack
        let stack_clone = stack.clone();
        sidebar_list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let index = row.index();
                let name = match index {
                    0 => "status",
                    1 => "rules",
                    2 => "profiles",
                    _ => return,
                };
                stack_clone.set_visible_child_name(name);
            }
        });

        // Set content and save refs
        self.set_content(Some(&split_view));

        *imp.split_view.borrow_mut() = Some(split_view);
        *imp.stack.borrow_mut() = Some(stack);
        *imp.status_view.borrow_mut() = Some(status_view);
        *imp.rules_view.borrow_mut() = Some(rules_view);
        *imp.profiles_view.borrow_mut() = Some(profiles_view);
        
        self.refresh_views();
    }

    fn refresh_views(&self) {
        let imp = self.imp();
        
        // Read status using backend
        match backend::read_status() {
            Ok(status) => {
                if let Some(view) = imp.status_view.borrow().as_ref() {
                    view.update(&status);
                }
                if let Some(view) = imp.rules_view.borrow().as_ref() {
                    view.update(&status);
                }
                if let Some(view) = imp.profiles_view.borrow().as_ref() {
                    // Pass rules for checking allowed profiles
                    view.update_rules(&status.rules);
                }
            }
            Err(e) => {
                eprintln!("Failed to read ufw status: {}", e);
            }
        }
        
        // Read profiles
        match backend::read_profiles() {
            Ok(profiles) => {
                if let Some(view) = imp.profiles_view.borrow().as_ref() {
                    view.update_profiles(profiles);
                }
            }
            Err(e) => {
                eprintln!("Failed to read ufw profiles: {}", e);
            }
        }
    }

    fn setup_monitoring(&self) {
        let weak_self = self.downgrade();
        crate::ufw::monitor::UfwMonitor::start(move || {
            if let Some(window) = weak_self.upgrade() {
                window.refresh_views();
            }
        });
    }
}
