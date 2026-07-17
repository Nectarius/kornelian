use dioxus::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Language {
    English,
    Greek,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Greek => "Ελληνικά",
        }
    }
}

#[derive(Clone)]
pub struct I18nContext {
    pub current_language: Signal<Language>,
    pub en_translations: HashMap<String, String>,
    pub el_translations: HashMap<String, String>,
}

impl I18nContext {
    pub fn translate(&self, key: &str) -> String {
        let lang = *self.current_language.read();
        let map = match lang {
            Language::English => &self.en_translations,
            Language::Greek => &self.el_translations,
        };
        
        map.get(key).cloned().unwrap_or_else(|| key.to_string())
    }
}

fn parse_properties(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

pub fn init_i18n() {
    let en_content = include_str!("../locales/en.properties");
    let el_content = include_str!("../locales/el.properties");

    let context = I18nContext {
        current_language: use_signal(|| Language::English),
        en_translations: parse_properties(en_content),
        el_translations: parse_properties(el_content),
    };

    use_context_provider(|| context);
}

pub fn use_i18n() -> I18nContext {
    use_context::<I18nContext>()
}
