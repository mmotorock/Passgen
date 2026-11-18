use crate::app::history::History;
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

/// Defines the color scheme for the UI.
#[derive(Debug, PartialEq, Clone, Copy)]
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
#[derive(PartialEq, Debug, Clone, Copy)]
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
    /// Generates a character-based password of a specific length.
    #[arg(short, long, group = "mode")]
    pub n: Option<usize>,

    /// Generates a word-based passphrase with a specific number of words (3, 4, or 5).
    #[arg(short, long, group = "mode", value_name = "COUNT")]
    pub w: Option<usize>,
}

/// Represents the character sets for password generation.
#[derive(Clone, Debug)]
pub struct CharacterSets {
    pub lowercase: String,
    pub uppercase: String,
    pub numbers: String,
    pub special: String,
}

impl CharacterSets {
    /// Creates a new instance with default character sets.
    pub fn default() -> Self {
        Self {
            lowercase: "abcdefghijklmnopqrstuvwxyz".to_string(),
            uppercase: "ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string(),
            numbers: "0123456789".to_string(),
            special: "!@#$%^&*()-_=+[]{}|;:,.<>?".to_string(),
        }
    }
}

/// The main application state for the GUI.
pub struct PassGenApp {
    // General state
    pub active_tab: Tab,
    pub status_message: String,
    pub config_path: PathBuf,
    pub saved_config_state: String,
    pub show_settings_window: bool,
    pub error_message: Option<String>, // For the error dialog
    pub clipboard_clear_at: Option<(Instant, String)>, // When to clear the clipboard, and what we copied

    // Settings
    pub theme: Theme,
    pub default_tab: Tab,
    pub words_file_path_str: String,

    // Character generator state
    pub char_sets: CharacterSets,
    pub use_lowercase: bool,
    pub use_uppercase: bool,
    pub use_numbers: bool,
    pub use_special: bool,
    pub char_length: usize,
    pub char_length_input: String, // Text in the length box while it is being edited
    pub char_password_output: String,
    pub char_strength_bits: Option<f64>,

    // Word generator state
    pub words: Vec<String>,
    pub word_count: usize,
    pub word_password_output: String,
    pub word_strength_bits: Option<f64>,
    pub words_file_path: PathBuf,
    pub use_separator: bool,
    pub separator_char: String,
    pub use_uppercase_words: bool,

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
            saved_config_state: String::new(),
            show_settings_window: false,
            error_message: None,
            clipboard_clear_at: None,
            theme: Theme::Dark,
            default_tab: Tab::Character,
            words_file_path_str: String::new(),
            char_sets: CharacterSets::default(),
            use_lowercase: true,
            use_uppercase: true,
            use_numbers: true,
            use_special: true,
            char_length: 16,
            char_length_input: String::new(),
            char_password_output: String::new(),
            char_strength_bits: None,
            words: Vec::new(),
            word_count: 3,
            word_password_output: String::new(),
            word_strength_bits: None,
            words_file_path: PathBuf::new(),
            use_separator: true,
            separator_char: "-".to_string(),
            use_uppercase_words: true,
            history: History::default(),
            show_history: false,
        }
    }
}
