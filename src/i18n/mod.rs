//! KayaBot's interface strings, in English (canonical) and French.
//!
//! The Preferences skeleton, theme and accent labels are not here: they come
//! from `colony_ui::i18n`, which [`t`] falls back to, and both tables follow
//! the locale set with [`set_locale`].

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::OnceLock;

use colony_ui::i18n::Locale;

const EN: &str = include_str!("en.json");
const FR: &str = include_str!("fr.json");

fn table(locale: Locale) -> &'static HashMap<String, String> {
    static EN_TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    static FR_TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    let (cell, source) = match locale {
        Locale::En => (&EN_TABLE, EN),
        Locale::Fr => (&FR_TABLE, FR),
    };
    // Both files are parsed by the tests below, so this cannot fail on a
    // checkout that passed them.
    cell.get_or_init(|| serde_json::from_str(source).expect("locale file is valid JSON"))
}

/// Switches KayaBot's strings and the shared colony-ui labels together.
pub fn set_locale(locale: Locale) {
    colony_ui::i18n::set_locale(locale);
}

/// The locale the environment asks for: `LC_ALL`, then `LC_MESSAGES`, then
/// `LANG`. English when none is set.
pub fn system_locale() -> Locale {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
        .map_or(Locale::En, |tag| Locale::from_tag(&tag))
}

/// The string for `key` in the active locale. Falls back to English, then to
/// the shared colony-ui labels, then to the key itself, so a missing string is
/// visible rather than blank.
pub fn t(key: &str) -> &'static str {
    let locale = colony_ui::i18n::locale();
    table(locale)
        .get(key)
        .or_else(|| table(Locale::En).get(key))
        .map(String::as_str)
        .unwrap_or_else(|| colony_ui::i18n::t(key))
}

/// [`t`] with every `{name}` placeholder replaced by its value.
pub fn tf(key: &str, args: &[(&str, &dyn Display)]) -> String {
    fill(t(key), args)
}

fn fill(text: &str, args: &[(&str, &dyn Display)]) -> String {
    args.iter().fold(text.to_string(), |text, (name, value)| {
        text.replace(&format!("{{{name}}}"), &value.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    fn keys(locale: Locale) -> BTreeSet<&'static str> {
        table(locale).keys().map(String::as_str).collect()
    }

    #[test]
    fn fr_and_en_have_identical_key_sets() {
        let en = keys(Locale::En);
        let fr = keys(Locale::Fr);
        let only_en: Vec<_> = en.difference(&fr).collect();
        let only_fr: Vec<_> = fr.difference(&en).collect();
        assert!(
            only_en.is_empty() && only_fr.is_empty(),
            "missing in fr.json: {only_en:?}, missing in en.json: {only_fr:?}"
        );
    }

    #[test]
    fn no_string_is_empty_or_uses_a_long_dash() {
        for locale in [Locale::En, Locale::Fr] {
            for (key, value) in table(locale) {
                assert!(!value.trim().is_empty(), "{key} is empty");
                assert!(
                    !value.contains(['\u{2013}', '\u{2014}']),
                    "{key} uses an en or em dash: {value}"
                );
            }
        }
    }

    #[test]
    fn placeholders_match_between_locales() {
        let placeholders = |text: &str| -> BTreeSet<String> {
            text.split('{')
                .skip(1)
                .filter_map(|rest| rest.split_once('}').map(|(name, _)| name.to_string()))
                .collect()
        };
        for (key, en) in table(Locale::En) {
            let fr = &table(Locale::Fr)[key];
            assert_eq!(placeholders(en), placeholders(fr), "{key}");
        }
    }

    #[test]
    fn tf_fills_placeholders() {
        let filled = fill(
            "{count} file(s) in {dir}, {count} in all",
            &[("count", &3), ("dir", &"/tmp")],
        );
        assert_eq!(filled, "3 file(s) in /tmp, 3 in all");
        assert_eq!(tf("no_such_key", &[]), "no_such_key");
    }

    /// Every key the interface asks for exists, here or in colony-ui. A typo
    /// would otherwise render as the raw key.
    #[test]
    fn every_key_used_in_the_source_exists() {
        let mut missing = Vec::new();
        let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
        for path in rust_files(&ui) {
            let source = std::fs::read_to_string(&path).expect("read source");
            for key in keys_in(&source) {
                let shared = colony_ui::i18n::t_in(Locale::En, key) != key;
                if !table(Locale::En).contains_key(key) && !shared {
                    missing.push(format!("{}: {key}", path.display()));
                }
            }
        }
        assert!(missing.is_empty(), "unknown keys: {missing:#?}");
    }

    /// The literal keys passed to `t(` and `tf(`.
    fn keys_in(source: &str) -> Vec<&str> {
        let mut keys = Vec::new();
        for call in ["t(", "tf("] {
            for (at, _) in source.match_indices(call) {
                let standalone = source[..at]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
                let literal = source[at + call.len()..].trim_start().strip_prefix('"');
                if let (true, Some(literal)) = (standalone, literal)
                    && let Some((key, _)) = literal.split_once('"')
                {
                    keys.push(key);
                }
            }
        }
        keys
    }

    #[test]
    fn keys_are_found_across_lines() {
        let source = "t(\"one\"); tf(\n    \"two\", &[]); expect(\"not a key\"); x.t(\"three\")";
        assert_eq!(keys_in(source), ["one", "three", "two"]);
    }

    fn rust_files(dir: &Path) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(rust_files(&path));
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
        files
    }
}
