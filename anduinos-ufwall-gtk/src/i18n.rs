use gettextrs::{bindtextdomain, bind_textdomain_codeset, gettext, textdomain};

use crate::config;

/// Initialize gettext for i18n support.
pub fn init() {
    bindtextdomain(config::GETTEXT_PACKAGE, config::LOCALEDIR)
        .expect("Unable to bind text domain");
    bind_textdomain_codeset(config::GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set text domain codeset");
    textdomain(config::GETTEXT_PACKAGE).expect("Unable to set text domain");
}

/// Translate a string using gettext.
#[allow(dead_code)]
pub fn i18n(s: &str) -> String {
    gettext(s)
}
