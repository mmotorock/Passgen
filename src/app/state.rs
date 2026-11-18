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
    about = "A password and passphrase generator.",
    long_about = "Generates strong, random passwords or memorable passphrases.\n\n\
                  Run without arguments to launch the GUI. With -n or -w it works on the command line instead, \
                  using the settings saved by the GUI; the options below override them for one run.",
    after_help = "Examples:\n  \
                  passgen -n 16                    one 16-character password\n  \
                  passgen -n 20 --no-special       letters and numbers only\n  \
                  passgen -w 4 --number            e.g. Mango-42-Otter-Kettle-Robin\n  \
                  passgen -w 5 --separator .       e.g. Mango.Otter.Kettle.Robin.Tulip\n  \
                  passgen -n 16 -c 10 --no-copy    print ten passwords, don't touch the clipboard"
)]
pub struct CliArgs {
    /// Generate a character password of this length (12 to 256)
    #[arg(short = 'n', long = "length", value_name = "LENGTH", group = "mode")]
    pub length: Option<usize>,

    /// Generate a passphrase with this many words (3 to 8)
    #[arg(short = 'w', long = "words", value_name = "COUNT", group = "mode")]
    pub words: Option<usize>,

    /// How many to generate, one per line
    #[arg(short = 'c', long, value_name = "N", default_value_t = 1, requires = "mode",
          value_parser = clap::value_parser!(u32).range(1..=100))]
    pub count: u32,

    /// Don't copy the result to the clipboard
    #[arg(long, requires = "mode")]
    pub no_copy: bool,

    /// Don't add the result to the history
    #[arg(long, requires = "mode")]
    pub no_history: bool,

    /// Leave out lowercase letters
    #[arg(long, requires = "mode", conflicts_with = "words", help_heading = "Character passwords (-n)")]
    pub no_lowercase: bool,

    /// Leave out uppercase letters
    #[arg(long, requires = "mode", conflicts_with = "words", help_heading = "Character passwords (-n)")]
    pub no_uppercase: bool,

    /// Leave out numbers
    #[arg(long, requires = "mode", conflicts_with = "words", help_heading = "Character passwords (-n)")]
    pub no_numbers: bool,

    /// Leave out special characters
    #[arg(long, requires = "mode", conflicts_with = "words", help_heading = "Character passwords (-n)")]
    pub no_special: bool,

    /// Put this character between words
    #[arg(long, value_name = "CHAR", requires = "mode", conflicts_with_all = ["length", "no_separator"],
          help_heading = "Passphrases (-w)")]
    pub separator: Option<char>,

    /// Join the words without a separator
    #[arg(long, requires = "mode", conflicts_with = "length", help_heading = "Passphrases (-w)")]
    pub no_separator: bool,

    /// Insert a two-digit number at a random position
    #[arg(long, requires = "mode", conflicts_with_all = ["length", "no_number"], help_heading = "Passphrases (-w)")]
    pub number: bool,

    /// Don't insert a number
    #[arg(long, requires = "mode", conflicts_with = "length", help_heading = "Passphrases (-w)")]
    pub no_number: bool,

    /// Start every word with an uppercase letter
    #[arg(long, requires = "mode", conflicts_with_all = ["length", "no_capitalize"], help_heading = "Passphrases (-w)")]
    pub capitalize: bool,

    /// Keep every word in lowercase
    #[arg(long, requires = "mode", conflicts_with = "length", help_heading = "Passphrases (-w)")]
    pub no_capitalize: bool,

    /// Use this wordlist (one word per line) instead of the saved one
    #[arg(long, value_name = "FILE", requires = "mode", conflicts_with = "length", help_heading = "Passphrases (-w)")]
    pub wordlist: Option<PathBuf>,
}

/// The main application state for the GUI.
pub struct PassGenApp {
    // General state
    pub active_tab: Tab,
    pub status_message: String,
    pub config_path: PathBuf,
    pub show_settings_window: bool,
    pub fitted_height: f32, // Window height last fitted to the content (0 before the first fit)
    pub generate_error: Option<String>, // Why the last generation failed, shown under the button
    pub clipboard_clear_at: Option<(Instant, String)>, // When to clear the clipboard, and what we copied
    pub copied_flash: Option<(Instant, String)>, // When something was copied, for the ✓ on its button

    // Settings, and the version last written to config.toml
    pub config: Config,
    pub saved_config: Config,
    pub words_file_path_str: String, // Text in the wordlist path box while it is being edited

    // Character generator state
    pub char_length_input: String, // Text in the length box while it is being edited
    pub char_password_output: String,

    // Word generator state
    pub words: Vec<String>,
    pub wordlist_status: String, // Result of loading the wordlist, shown if it failed
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
            generate_error: None,
            clipboard_clear_at: None,
            copied_flash: None,
            config: Config::default(),
            saved_config: Config::default(),
            words_file_path_str: String::new(),
            char_length_input: String::new(),
            char_password_output: String::new(),
            words: Vec::new(),
            wordlist_status: String::new(),
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
        messages.push(word_status.clone());
        messages.extend(history_warning);

        Self {
            active_tab: config.default_tab,
            status_message: messages.join("\n"),
            config_path,
            words_file_path_str: config.words_file_path.to_string_lossy().to_string(),
            saved_config: config.clone(),
            config,
            words,
            wordlist_status: word_status,
            history,
            ..Self::default()
        }
    }
}
