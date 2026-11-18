use crate::app::config::{Config, CONFIG_FILE};
use crate::app::history::History;
use crate::app::utils::{get_config_directory, load_words};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

/// Defines the color scheme for the UI.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
}

impl FromStr for Theme {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Light" => Ok(Theme::Light),
            "Dark" => Ok(Theme::Dark),
            _ => Err(()),
        }
    }
}

/// Defines the active UI tab.
#[derive(PartialEq, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Tab {
    Character,
    Word,
}

impl FromStr for Tab {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Character" => Ok(Tab::Character),
            "Word" => Ok(Tab::Word),
            _ => Err(()),
        }
    }
}

/// Command-Line Argument Definitions
#[derive(Parser, Debug)]
#[command(
    version,
    about = "A versatile password and passphrase generator.",
    long_about = "Generates strong, random passwords or memorable passphrases.\n\n\
                  Run without arguments to launch the GUI.\n\
                  Use either -n for character-based passwords or -w for word-based passphrases in CLI mode."
)]
pub struct CliArgs {
    /// Generates a character-based password of a specific length (12 to 256), using the saved settings.
    #[arg(short, long, group = "mode")]
    pub n: Option<usize>,

    /// Generates a word-based passphrase with a specific number of words (3 to 8), using the saved settings.
    #[arg(short, long, group = "mode", value_name = "COUNT")]
    pub w: Option<usize>,
}

/// The main application state for the GUI.
pub struct PassGenApp {
    // General state
    pub active_tab: Tab,
    pub status_message: String,
    pub config_path: PathBuf,
    pub show_settings_window: bool,
    pub fitted_height: f32, // Window height last fitted to the content (0 before the first fit)
    pub error_message: Option<String>, // For the error dialog
    pub clipboard_clear_at: Option<(Instant, String)>, // When to clear the clipboard, and what we copied

    // Settings, and the version last written to config.toml
    pub config: Config,
    pub saved_config: Config,
    pub words_file_path_str: String, // Text in the wordlist path box while it is being edited

    // Character generator state
    pub char_length_input: String, // Text in the length box while it is being edited
    pub char_password_output: String,

    // Word generator state
    pub words: Vec<String>,
    pub word_password_output: String,

    // Recently generated passwords
    pub history: History,
    pub show_history: bool,
}

impl Default for PassGenApp {
    /// Initializes the application with hard-coded default values.
    fn default() -> Self {
        Self {
            active_tab: Tab::Character,
            status_message: "Default settings loaded.".to_string(),
            config_path: PathBuf::new(),
            show_settings_window: false,
            fitted_height: 0.0,
            error_message: None,
            clipboard_clear_at: None,
            config: Config::default(),
            saved_config: Config::default(),
            words_file_path_str: String::new(),
            char_length_input: String::new(),
            char_password_output: String::new(),
            words: Vec::new(),
            word_password_output: String::new(),
            history: History::default(),
            show_history: false,
        }
    }
}

impl PassGenApp {
    /// Creates a new application instance, loading the settings, wordlist and history.
    pub fn new() -> Self {
        let app_dir = get_config_directory();
        let config_path = app_dir.join(CONFIG_FILE);
        let (config, mut messages) = Config::load(&config_path);
        let (words, word_status) = load_words(&config.words_file_path);
        let (history, history_warning) = History::load(app_dir.join("history.dat"));
        messages.push(word_status);
        messages.extend(history_warning);

        Self {
            active_tab: config.default_tab,
            status_message: messages.join("\n"),
            config_path,
            words_file_path_str: config.words_file_path.to_string_lossy().to_string(),
            saved_config: config.clone(),
            config,
            words,
            history,
            ..Self::default()
        }
    }
}
