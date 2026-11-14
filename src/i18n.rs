//! Compiled, allocation-free UI translations. Technical device/OS errors and
//! user-authored preset metadata retain their original text.
use serde::{Deserialize, Serialize};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(usize)]
pub enum Language {
    #[default]
    En,
    Ru,
    De,
    Fr,
    Es,
    Pt,
    It,
    Pl,
    Tr,
    Uk,
}
impl Language {
    pub const ALL: [Self; 10] = [
        Self::En,
        Self::Ru,
        Self::De,
        Self::Fr,
        Self::Es,
        Self::Pt,
        Self::It,
        Self::Pl,
        Self::Tr,
        Self::Uk,
    ];
    pub fn name(self) -> &'static str {
        [
            "English",
            "Русский",
            "Deutsch",
            "Français",
            "Español",
            "Português",
            "Italiano",
            "Polski",
            "Türkçe",
            "Українська",
        ][self as usize]
    }
    pub fn from_code(code: &str) -> Option<Self> {
        ["en", "ru", "de", "fr", "es", "pt", "it", "pl", "tr", "uk"]
            .iter()
            .position(|value| *value == code)
            .map(|index| Self::ALL[index])
    }
}
thread_local! { static CURRENT: Cell<Language> = const { Cell::new(Language::En) }; }
include!(concat!(env!("OUT_DIR"), "/translations.rs"));
pub fn set_language(language: Language) {
    CURRENT.set(language);
}
pub fn t(key: &str) -> &str {
    lookup(CURRENT.get(), key)
}
fn lookup(language: Language, key: &str) -> &str {
    if language == Language::En {
        return key;
    }
    MESSAGES
        .binary_search_by_key(&key, |(source, _)| source)
        .map(|index| MESSAGES[index].1[language as usize - 1])
        .unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_language_round_trips_and_translates_the_main_controls() {
        for language in Language::ALL {
            let json = serde_json::to_string(&language).unwrap();
            assert_eq!(serde_json::from_str::<Language>(&json).unwrap(), language);
            assert_eq!(Language::from_code(json.trim_matches('"')), Some(language));
            for key in [
                "Voices",
                "Start voice",
                "Settings",
                "Language",
                "Save voice",
                "Recordings",
            ] {
                assert!(!lookup(language, key).is_empty());
                if language != Language::En {
                    assert_ne!(lookup(language, key), key, "{language:?}: {key}");
                }
            }
        }
        assert_eq!(Language::from_code("unknown"), None);
        assert_eq!(
            lookup(Language::De, "User's own preset name"),
            "User's own preset name"
        );
    }
    #[test]
    fn catalog_has_complete_sorted_unique_entries() {
        assert!(MESSAGES.len() > 100);
        for pair in MESSAGES.windows(2) {
            assert!(pair[0].0 < pair[1].0);
        }
        for (_, translations) in MESSAGES {
            assert!(translations.iter().all(|text| !text.trim().is_empty()));
        }
    }
}
