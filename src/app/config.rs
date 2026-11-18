use crate::app::history::History;
use crate::app::state::{CharacterSets, PassGenApp, Tab, Theme};
use crate::app::password::{MAX_WORDS, MIN_WORDS};
use crate::app::utils::{get_config_directory, load_words};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

impl PassGenApp {
    /// Creates a new application instance, loading config from file.
    pub fn new() -> Self {
        let mut app = Self::default(); // Start with hard-coded defaults

        let app_dir = get_config_directory();
        app.config_path = app_dir.join("config.toml");

        let config_path = app.config_path.clone();
        let config_status = app.load_config_from_path(&config_path);

        let words_file_path = app.words_file_path.clone();
        let (words, word_status) = load_words(&words_file_path);
        app.words = words;

        let (history, history_status) = History::load(app_dir.join("history.dat"));
        app.history = history;

        app.status_message = [Some(config_status), Some(word_status), history_status]
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        app.saved_config_state = app.get_current_config_as_string();

        app
    }

    pub fn get_current_config_as_string(&self) -> String {
        format!(
            "theme={:?}\n\
             words_file_path={}\n\
             default_tab={:?}\n\
             use_lowercase={}\n\
             use_uppercase={}\n\
             use_numbers={}\n\
             use_special={}\n\
             char_length={}\n\
             lowercase_chars={}\n\
             uppercase_chars={}\n\
             number_chars={}\n\
             special_chars={}\n\
             word_count={}\n\
             use_separator={}\n\
             separator_char={}\n\
             use_uppercase_words={}\n\
             include_number={}\n",
            self.theme,
            self.words_file_path.to_string_lossy(),
            self.default_tab,
            self.use_lowercase,
            self.use_uppercase,
            self.use_numbers,
            self.use_special,
            self.char_length,
            self.char_sets.lowercase,
            self.char_sets.uppercase,
            self.char_sets.numbers,
            self.char_sets.special,
            self.word_count,
            self.use_separator,
            self.separator_char,
            self.use_uppercase_words,
            self.include_number,
        )
    }

    /// Writes the current settings to config.toml. Called automatically whenever a setting changes.
    pub fn save_config(&mut self) -> Result<(), String> {
        let content = self.get_current_config_as_string();
        self.saved_config_state = content.clone();
        fs::write(&self.config_path, content).map_err(|e| format!("Error saving config.toml: {}", e))
    }

    pub fn load_config_from_path(&mut self, path: &PathBuf) -> String {
        let config = load_config_map(path);
        if config.is_empty() {
            return if path == &self.config_path {
                self.words_file_path = PathBuf::new(); // Built-in wordlist
                self.words_file_path_str = String::new();
                match self.save_config() {
                    Ok(()) => "Created config.toml with default settings".to_string(),
                    Err(e) => e,
                }
            } else {
                format!("Error: Failed to load or parse config from {:?}", path)
            };
        }

        self.theme = config.get("theme").and_then(|s| s.parse().ok()).unwrap_or(Theme::Dark);

        // An empty path means the built-in wordlist. Earlier versions copied a wordlist to
        // <config dir>/words.txt automatically; treat that old default as the built-in list too.
        let old_default_words_path = get_config_directory().join("words.txt");
        self.words_file_path = config
            .get("words_file_path")
            .map(PathBuf::from)
            .filter(|path| *path != old_default_words_path)
            .unwrap_or_default();
        self.words_file_path_str = self.words_file_path.to_string_lossy().to_string();

        self.default_tab = config.get("default_tab").and_then(|s| s.parse().ok()).unwrap_or(Tab::Character);
        self.active_tab = self.default_tab;
        self.use_lowercase = config.get("use_lowercase").and_then(|s| s.parse().ok()).unwrap_or(true);
        self.use_uppercase = config.get("use_uppercase").and_then(|s| s.parse().ok()).unwrap_or(true);
        self.use_numbers = config.get("use_numbers").and_then(|s| s.parse().ok()).unwrap_or(true);
        self.use_special = config.get("use_special").and_then(|s| s.parse().ok()).unwrap_or(true);
        self.char_length = config.get("char_length").and_then(|s| s.parse().ok()).unwrap_or(16);
        self.char_sets.lowercase = config.get("lowercase_chars").cloned().unwrap_or_else(|| CharacterSets::default().lowercase);
        self.char_sets.uppercase = config.get("uppercase_chars").cloned().unwrap_or_else(|| CharacterSets::default().uppercase);
        self.char_sets.numbers = config.get("number_chars").cloned().unwrap_or_else(|| CharacterSets::default().numbers);
        self.char_sets.special = config.get("special_chars").cloned().unwrap_or_else(|| CharacterSets::default().special);
        self.word_count = config
            .get("word_count")
            .and_then(|s| s.parse().ok())
            .unwrap_or(4)
            .clamp(MIN_WORDS, MAX_WORDS);
        self.use_separator = config.get("use_separator").and_then(|s| s.parse().ok()).unwrap_or(true);
        self.separator_char = config.get("separator_char").cloned().unwrap_or_else(|| "-".to_string());
        self.use_uppercase_words = config.get("use_uppercase_words").and_then(|s| s.parse().ok()).unwrap_or(true);
        self.include_number = config.get("include_number").and_then(|s| s.parse().ok()).unwrap_or(false);

        self.saved_config_state = self.get_current_config_as_string();
        format!("Successfully loaded configuration from {:?}", path)
    }
}

pub fn load_config_map(path: &PathBuf) -> HashMap<String, String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, '=');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect()
}
