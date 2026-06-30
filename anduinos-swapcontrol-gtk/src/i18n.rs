use gettextrs::{gettext, TextDomain};

use crate::config;

/// Initialize gettext for i18n support.
/// Does not panic if translations are missing — falls back to English.
pub fn init() {
    TextDomain::new(config::GETTEXT_PACKAGE)
        .codeset("UTF-8")
        .init()
        .ok(); // Don't panic if locale files are not installed yet
}

/// Translate a string using gettext.
#[allow(dead_code)]
pub fn i18n(s: &str) -> String {
    gettext(s)
}
