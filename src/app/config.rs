use crate::app::state::{CharacterSets, PassGenApp, Tab, Theme};
use crate::app::utils::{get_app_directory, load_words_from_file};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

impl PassGenApp {
    /// Creates a new application instance, loading config from file.
    pub fn new() -> Self {
        let mut app = Self::default(); // Start with hard-coded defaults

        let app_dir = get_app_directory();
        app.config_path = app_dir.join("config.toml");
        app.password_file_path = app_dir.join("password.txt");

        let config_path = app.config_path.clone();
        let config_status = app.load_config_from_path(&config_path);

        let words_file_path = app.words_file_path.clone();
        let (words, word_status) = load_words_from_file(&words_file_path);
        app.words = words;

        app.status_message = format!("{}\n{}", config_status, word_status);
        app.saved_config_state = app.get_current_config_as_string();

        app
    }

    pub fn get_current_config_as_string(&self) -> String {
        format!(
            "theme={:?}\n\
             words_file_path={}\n\
             active_tab={:?}\n\
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
             use_uppercase_words={}\n",
            self.theme,
            self.words_file_path.to_string_lossy(),
            self.active_tab,
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
        )
    }

    pub fn save_config(&mut self) -> String {
        self.words_file_path = PathBuf::from(&self.words_file_path_str);
        let content = self.get_current_config_as_string();
        self.saved_config_state = content.clone();
        match fs::write(&self.config_path, content) {
            Ok(_) => "Configuration saved to config.toml".to_string(),
            Err(e) => format!("Error saving config.toml: {}", e),
        }
    }

    pub fn load_config_from_path(&mut self, path: &PathBuf) -> String {
        let config = load_config_map(path);
        if config.is_empty() {
            return if path == &self.config_path {
                self.words_file_path = get_app_directory().join("words.txt");
                self.words_file_path_str = self.words_file_path.to_string_lossy().to_string();
                self.save_config()
            } else {
                format!("Error: Failed to load or parse config from {:?}", path)
            };
        }

        self.theme = config.get("theme").and_then(|s| s.parse().ok()).unwrap_or(Theme::Dark);

        let default_words_path = get_app_directory().join("words.txt");
        self.words_file_path = config.get("words_file_path").map(PathBuf::from).unwrap_or(default_words_path);
        self.words_file_path_str = self.words_file_path.to_string_lossy().to_string();

        self.active_tab = config.get("active_tab").and_then(|s| s.parse().ok()).unwrap_or(Tab::Character);
        self.use_lowercase = config.get("use_lowercase").and_then(|s| s.parse().ok()).unwrap_or(true);
        self.use_uppercase = config.get("use_uppercase").and_then(|s| s.parse().ok()).unwrap_or(true);
        self.use_numbers = config.get("use_numbers").and_then(|s| s.parse().ok()).unwrap_or(true);
        self.use_special = config.get("use_special").and_then(|s| s.parse().ok()).unwrap_or(true);
        self.char_length = config.get("char_length").and_then(|s| s.parse().ok()).unwrap_or(16);
        self.char_sets.lowercase = config.get("lowercase_chars").cloned().unwrap_or_else(|| CharacterSets::default().lowercase);
        self.char_sets.uppercase = config.get("uppercase_chars").cloned().unwrap_or_else(|| CharacterSets::default().uppercase);
        self.char_sets.numbers = config.get("number_chars").cloned().unwrap_or_else(|| CharacterSets::default().numbers);
        self.char_sets.special = config.get("special_chars").cloned().unwrap_or_else(|| CharacterSets::default().special);
        self.word_count = config.get("word_count").and_then(|s| s.parse().ok()).unwrap_or(3);
        self.use_separator = config.get("use_separator").and_then(|s| s.parse().ok()).unwrap_or(true);
        self.separator_char = config.get("separator_char").cloned().unwrap_or_else(|| "-".to_string());
        self.use_uppercase_words = config.get("use_uppercase_words").and_then(|s| s.parse().ok()).unwrap_or(true);

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
