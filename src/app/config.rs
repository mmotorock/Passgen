use crate::app::password::{MAX_LENGTH, MAX_WORDS, MIN_LENGTH, MIN_WORDS};
use crate::app::state::Style;
use crate::app::themes::DARK;
use crate::app::styles::{GROUPS_RANGE, PIN_LENGTH_RANGE, PRONOUNCEABLE_GROUPS_RANGE};
use crate::app::utils::get_config_directory;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// The name of the settings file inside the config folder.
pub const CONFIG_FILE: &str = "config.toml";

const FILE_HEADER: &str = "# Passgen settings. The app rewrites this file whenever a setting changes,\n\
                           # so edit it while the app is closed.\n\n";

/// All user settings, stored as TOML in config.toml.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Config {
    /// The colour theme's ID: "light", "dark" or the name of a file in the themes folder.
    pub theme: String,
    /// The style selected when the app opens, and used by the CLI unless told otherwise.
    pub default_style: Style,
    /// A custom wordlist; empty means the built-in list.
    pub words_file_path: PathBuf,
    /// The character sets (used by the Random and Segmented styles) and the Random length.
    pub characters: CharacterSettings,
    pub segmented: SegmentedSettings,
    pub grouped: GroupedSettings,
    pub pronounceable: PronounceableSettings,
    pub pin: PinSettings,
    pub words: WordSettings,
}

/// Settings for the Segmented style; its characters come from `CharacterSettings`.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct SegmentedSettings {
    pub length: usize,
}

/// Settings for the Grouped style.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct GroupedSettings {
    pub groups: usize,
}

/// Settings for the Pronounceable style.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct PronounceableSettings {
    pub groups: usize,
    pub include_number: bool,
}

/// Settings for the PIN style.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct PinSettings {
    pub length: usize,
}

/// Settings for character-based passwords.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct CharacterSettings {
    pub length: usize,
    pub use_lowercase: bool,
    pub use_uppercase: bool,
    pub use_numbers: bool,
    pub use_special: bool,
    pub lowercase: String,
    pub uppercase: String,
    pub numbers: String,
    pub special: String,
}

/// Settings for word-based passphrases.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct WordSettings {
    pub count: usize,
    pub use_separator: bool,
    pub separator: String,
    /// Uppercase the first letter of every word.
    pub capitalize: bool,
    pub include_number: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: DARK.to_string(),
            default_style: Style::Segmented,
            words_file_path: PathBuf::new(),
            characters: CharacterSettings::default(),
            segmented: SegmentedSettings::default(),
            grouped: GroupedSettings::default(),
            pronounceable: PronounceableSettings::default(),
            pin: PinSettings::default(),
            words: WordSettings::default(),
        }
    }
}

impl Default for SegmentedSettings {
    fn default() -> Self {
        Self { length: 16 }
    }
}

impl Default for GroupedSettings {
    fn default() -> Self {
        Self { groups: 3 }
    }
}

impl Default for PronounceableSettings {
    fn default() -> Self {
        Self { groups: 4, include_number: true }
    }
}

impl Default for PinSettings {
    fn default() -> Self {
        Self { length: 6 }
    }
}

impl Default for CharacterSettings {
    /// The default sets leave out characters that are hard to tell apart or to name,
    /// so passwords can be read out over the phone:
    /// - look-alikes: 0 O o, 1 l I, B (8)
    /// - letters whose two cases look alike (c k p s u v w x z): only the lowercase is kept
    /// - symbols that are tiny, ambiguous or hard to name: ' " ` | ; : , . _ ^ ~ \ / and brackets
    fn default() -> Self {
        Self {
            length: 16,
            use_lowercase: true,
            use_uppercase: true,
            use_numbers: true,
            use_special: true,
            lowercase: "abcdefghijkmnpqrstuvwxyz".to_string(),
            uppercase: "ADEFGHJLMNQRTY".to_string(),
            numbers: "23456789".to_string(),
            special: "!@#$%&*+=?-".to_string(),
        }
    }
}

impl Default for WordSettings {
    fn default() -> Self {
        Self {
            count: 4,
            use_separator: true,
            separator: "-".to_string(),
            capitalize: true,
            include_number: false,
        }
    }
}

impl Config {
    /// Loads the settings from `path`, returning them with any messages worth showing.
    ///
    /// A missing file is created with defaults. A file in the format used by earlier
    /// versions is converted, and anything unreadable is kept as `config.toml.bak`.
    /// Invalid or unknown entries fall back to their defaults and are reported.
    pub fn load(path: &Path) -> (Config, Vec<String>) {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let config = Config::default();
                let message = match config.save(path) {
                    Ok(()) => "Created config.toml with default settings".to_string(),
                    Err(e) => e,
                };
                return (config, vec![message]);
            }
            Err(e) => return (Config::default(), vec![format!("Could not read config.toml: {}", e)]),
        };

        let mut messages = Vec::new();
        let mut config = match from_toml(&text) {
            Ok((config, ignored)) => {
                if text.lines().any(|line| line.trim_start().starts_with("default_tab")) {
                    if let Err(e) = config.save(path) {
                        messages.push(e);
                    }
                }
                if !ignored.is_empty() {
                    messages.push(format!("Ignored invalid settings in config.toml: {}", ignored.join(", ")));
                }
                config
            }
            Err(error) => {
                let backup = path.with_extension("toml.bak");
                let backed_up = fs::copy(path, &backup).is_ok();
                let mut config = if is_legacy_format(&text) {
                    messages.push("Converted config.toml to the new format".to_string());
                    from_legacy(&text, &get_config_directory().join("words.txt"))
                } else {
                    messages.push(format!("Could not read config.toml, using defaults: {}", error.message()));
                    Config::default()
                };
                config.normalize();
                if backed_up {
                    messages.push("The previous file was kept as config.toml.bak".to_string());
                }
                if let Err(e) = config.save(path) {
                    messages.push(e);
                }
                config
            }
        };
        config.normalize();
        (config, messages)
    }

    /// Writes the settings to `path` as TOML.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let body = toml::to_string_pretty(self).map_err(|e| format!("Error saving config.toml: {}", e))?;
        fs::write(path, format!("{}{}", FILE_HEADER, body)).map_err(|e| format!("Error saving config.toml: {}", e))
    }

    /// Brings values a hand-edited file might have out of range back into range.
    fn normalize(&mut self) {
        self.characters.length = self.characters.length.clamp(MIN_LENGTH, MAX_LENGTH);
        self.segmented.length = self.segmented.length.clamp(MIN_LENGTH, MAX_LENGTH);
        self.grouped.groups = self.grouped.groups.clamp(*GROUPS_RANGE.start(), *GROUPS_RANGE.end());
        self.pronounceable.groups = self
            .pronounceable
            .groups
            .clamp(*PRONOUNCEABLE_GROUPS_RANGE.start(), *PRONOUNCEABLE_GROUPS_RANGE.end());
        self.pin.length = self.pin.length.clamp(*PIN_LENGTH_RANGE.start(), *PIN_LENGTH_RANGE.end());
        self.words.count = self.words.count.clamp(MIN_WORDS, MAX_WORDS);
        self.words.separator = self.words.separator.chars().take(1).collect();
    }
}

/// Reads TOML leniently: every valid entry is used, while invalid values and unknown keys
/// keep their defaults and are returned by name. Fails only if the text isn't TOML at all.
fn from_toml(text: &str) -> Result<(Config, Vec<String>), toml::de::Error> {
    let mut file: toml::Table = toml::from_str(text)?;
    // The previous version had two tabs instead of styles.
    if let Some(toml::Value::String(tab)) = file.remove("default_tab") {
        if let (false, Ok(style)) = (file.contains_key("default_style"), tab.parse::<Style>()) {
            file.insert("default_style".to_string(), toml::Value::try_from(style).expect("a style is a string"));
        }
    }
    let mut merged = default_table();
    let mut ignored = Vec::new();

    for (key, value) in file {
        match value {
            toml::Value::Table(section) if merged.get(&key).is_some_and(toml::Value::is_table) => {
                for (field, value) in section {
                    if !try_set(&mut merged, &[&key, &field], value) {
                        ignored.push(format!("{}.{}", key, field));
                    }
                }
            }
            value => {
                if !try_set(&mut merged, &[&key], value) {
                    ignored.push(key);
                }
            }
        }
    }

    let config = toml::Value::Table(merged).try_into().expect("only valid entries were merged");
    Ok((config, ignored))
}

/// The default settings as a TOML table.
fn default_table() -> toml::Table {
    match toml::Value::try_from(Config::default()) {
        Ok(toml::Value::Table(table)) => table,
        _ => unreachable!("Config always serializes to a table"),
    }
}

/// Sets the entry at `path` (a key, or a section and a key) to `value`, but only if
/// the entry exists and the result is still a valid `Config`. Returns whether it was set.
fn try_set(table: &mut toml::Table, path: &[&str], value: toml::Value) -> bool {
    let mut candidate = table.clone();
    let slot = match path {
        [key] => candidate.get_mut(*key),
        [section, key] => candidate
            .get_mut(*section)
            .and_then(toml::Value::as_table_mut)
            .and_then(|section| section.get_mut(*key)),
        _ => None,
    };
    let Some(slot) = slot else {
        return false;
    };
    *slot = value;

    if toml::Value::Table(candidate.clone()).try_into::<Config>().is_ok() {
        *table = candidate;
        true
    } else {
        false
    }
}

/// Whether `text` looks like the `key=value` format used before config.toml was real TOML.
fn is_legacy_format(text: &str) -> bool {
    text.lines()
        .any(|line| ["char_length=", "word_count=", "lowercase_chars="].iter().any(|key| line.starts_with(key)))
}

/// Reads the `key=value` format used by earlier versions. `old_default_words_path` is the
/// wordlist those versions copied into the config folder; it maps to the built-in list.
fn from_legacy(text: &str, old_default_words_path: &Path) -> Config {
    let values: HashMap<&str, &str> = text.lines().filter_map(|line| line.split_once('=')).collect();
    fn set<T: FromStr>(values: &HashMap<&str, &str>, key: &str, target: &mut T) {
        if let Some(value) = values.get(key).and_then(|value| value.parse().ok()) {
            *target = value;
        }
    }

    let mut config = Config::default();
    set(&values, "theme", &mut config.theme);
    set(&values, "default_tab", &mut config.default_style);
    set(&values, "words_file_path", &mut config.words_file_path);
    if config.words_file_path == old_default_words_path {
        config.words_file_path = PathBuf::new();
    }

    let characters = &mut config.characters;
    set(&values, "char_length", &mut characters.length);
    set(&values, "use_lowercase", &mut characters.use_lowercase);
    set(&values, "use_uppercase", &mut characters.use_uppercase);
    set(&values, "use_numbers", &mut characters.use_numbers);
    set(&values, "use_special", &mut characters.use_special);
    set(&values, "lowercase_chars", &mut characters.lowercase);
    set(&values, "uppercase_chars", &mut characters.uppercase);
    set(&values, "number_chars", &mut characters.numbers);
    set(&values, "special_chars", &mut characters.special);

    let words = &mut config.words;
    set(&values, "word_count", &mut words.count);
    set(&values, "use_separator", &mut words.use_separator);
    set(&values, "separator_char", &mut words.separator);
    set(&values, "use_uppercase_words", &mut words.capitalize);
    set(&values, "include_number", &mut words.include_number);
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty directory for one test.
    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("passgen-test-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn round_trips_through_toml() {
        let config = Config {
            theme: "nord".to_string(),
            default_style: Style::Pronounceable,
            characters: CharacterSettings { special: "\"\\'#=[]".to_string(), ..Default::default() },
            pin: PinSettings { length: 9 },
            words: WordSettings { count: 6, ..Default::default() },
            ..Default::default()
        };
        let text = toml::to_string_pretty(&config).unwrap();
        assert_eq!(from_toml(&text).unwrap(), (config, vec![]));
    }

    #[test]
    fn keeps_valid_entries_and_reports_invalid_ones() {
        let text = r#"
            theme = 5
            colour = 1
            default_tab = "Word"

            [pin]
            length = 99

            [characters]
            length = "long"
            use_special = false

            [words]
            count = 6
        "#;
        let (config, ignored) = from_toml(text).unwrap();
        assert_eq!(config.theme, Config::default().theme);
        assert_eq!(config.default_style, Style::Words, "the old default_tab is migrated");
        assert_eq!(config.pin.length, 99, "only normalize() clamps; from_toml keeps valid numbers");
        assert_eq!(config.characters.length, CharacterSettings::default().length);
        assert!(!config.characters.use_special);
        assert_eq!(config.words.count, 6);
        let mut ignored = ignored;
        ignored.sort();
        assert_eq!(ignored, vec!["characters.length", "colour", "theme"]);
    }

    #[test]
    fn converts_legacy_format() {
        let old_words = Path::new("/home/someone/.config/passgen/words.txt");
        let text = "theme=Light\nwords_file_path=/home/someone/.config/passgen/words.txt\nactive_tab=Word\n\
                    use_special=false\nchar_length=20\nspecial_chars=!@#=\nword_count=5\nseparator_char=_\n\
                    use_uppercase_words=false\n";
        assert!(is_legacy_format(text));
        let config = from_legacy(text, old_words);
        assert_eq!(config.theme, "Light", "theme IDs are matched ignoring case");
        assert_eq!(config.words_file_path, PathBuf::new());
        assert_eq!(config.default_style, Style::Segmented);
        assert!(!config.characters.use_special);
        assert_eq!(config.characters.length, 20);
        assert_eq!(config.characters.special, "!@#=");
        assert_eq!(config.words.count, 5);
        assert_eq!(config.words.separator, "_");
        assert!(!config.words.capitalize);
    }

    #[test]
    fn load_creates_converts_and_clamps() {
        let dir = test_dir("load");
        let path = dir.join(CONFIG_FILE);

        let (config, messages) = Config::load(&path);
        assert_eq!(config, Config::default());
        assert!(path.exists(), "{:?}", messages);

        fs::write(&path, "char_length=300\nword_count=3\ntheme=Light\n").unwrap();
        let (config, _) = Config::load(&path);
        assert_eq!(config.characters.length, MAX_LENGTH);
        assert_eq!(config.words.count, 3);
        assert!(dir.join("config.toml.bak").exists());
        let (reloaded, messages) = Config::load(&path);
        assert_eq!(reloaded, config);
        assert!(messages.is_empty(), "{:?}", messages);

        fs::remove_dir_all(dir).unwrap();
    }
}
