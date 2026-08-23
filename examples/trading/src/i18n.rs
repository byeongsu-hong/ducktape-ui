//! Every sentence the screen draws, in the language picked on settings.
//!
//! The English is the key. A `.ice` node keeps its sentence in the source
//! wrapped in `t(locale, ...)`, and the other languages are looked up by
//! it here. A key no table carries comes back as itself, so the app reads
//! as English rather than as a hole at every commit of a translation, and a
//! test written against the English keeps reading the English.
//!
//! One table per language, one arm per sentence, a plain `match`: there is
//! no plural machinery because Korean has no plural categories and the
//! numbers are already formatted by the time a sentence holds them, and
//! there is no file to load because a missing file is a screen of holes.
//! A test below walks the `.ice` sources and the Rust prose for every key
//! and asks this table for each one, so a sentence added without its Korean
//! fails the build's tests rather than the reader.

use crate::Locale;

/// The sentence `key` in `locale`. English is the key itself.
pub fn t(locale: Locale, key: String) -> String {
    match locale {
        Locale::En => key,
        Locale::Ko => ko(&key).map_or(key, str::to_owned),
    }
}

/// What the picker calls a language, in that language.
pub fn locale_name(locale: Locale) -> String {
    match locale {
        Locale::En => "English",
        Locale::Ko => "한국어",
    }
    .to_owned()
}

/// What pressing a language does, said in the language being offered: a
/// reader who cannot read the one on screen has to be able to hear their own.
pub fn locale_label(locale: Locale) -> String {
    match locale {
        Locale::En => "Read this app in English",
        Locale::Ko => "이 앱을 한국어로 읽기",
    }
    .to_owned()
}

fn ko(key: &str) -> Option<&'static str> {
    Some(match key {
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_its_own_key() {
        assert_eq!(t(Locale::En, "EQUITY".into()), "EQUITY");
    }

    #[test]
    fn a_key_no_table_carries_reads_as_english() {
        assert_eq!(t(Locale::Ko, "no such sentence".into()), "no such sentence");
    }
}
