use crate::app::config::{Config, CONFIG_FILE};
use crate::app::history::History;
use crate::app::themes::{self, Theme};
use crate::app::utils::{get_config_directory, load_words};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

/// The kinds of password Passgen can generate. Each has its own options in the GUI.
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, Serialize, Deserialize, clap::ValueEnum)]
pub enum Style {
    /// Every character picked at random from the selected sets
    Random,
    /// Same-case groups of 3 letters, each closed by a number or symbol
    Segmented,
    /// Dash-separated blocks of 6 lowercase letters with one capital and one digit
    Grouped,
    /// Made-up words of easy syllables
    Pronounceable,
    /// Digits only
    Pin,
    /// Common words (a passphrase)
    Words,
}

impl Style {
    /// Every style, in the order shown in the GUI.
    pub const ALL: [Style; 6] = [
        Style::Random,
        Style::Segmented,
        Style::Grouped,
        Style::Pronounceable,
        Style::Pin,
        Style::Words,
    ];

    /// The name shown in the GUI.
    pub fn label(self) -> &'static str {
        match self {
            Style::Random => "Random",
            Style::Segmented => "Segmented",
            Style::Grouped => "Grouped",
            Style::Pronounceable => "Pronounceable",
            Style::Pin => "PIN",
            Style::Words => "Words",
        }
    }
}

/// A key press that switches the style: Tab/Shift+Tab, the arrow keys or h/l, or 1-6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleKey {
    Next,
    Previous,
    /// Jump to the style at this position (0 for the first).
    Jump(usize),
}

impl Style {
    /// The style that `key` switches to from this one. Next and Previous wrap around.
    pub fn after(self, key: StyleKey) -> Style {
        let count = Style::ALL.len();
        let index = Style::ALL.iter().position(|style| *style == self).unwrap_or(0);
        let target = match key {
            StyleKey::Next => (index + 1) % count,
            StyleKey::Previous => (index + count - 1) % count,
            StyleKey::Jump(position) if position < count => position,
            StyleKey::Jump(_) => index,
        };
        Style::ALL[target]
    }
}

impl FromStr for Style {
    type Err = ();
    /// Also accepts the tab names used by earlier versions ("Character" and "Word").
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Character" => Ok(Style::Segmented),
            "Word" => Ok(Style::Words),
            _ => Style::ALL.into_iter().find(|style| format!("{:?}", style) == s).ok_or(()),
        }
    }
}

/// Command-Line Argument Definitions
#[derive(Parser, Debug)]
#[command(
    version,
    about = "A password and passphrase generator.",
    long_about = "Generates strong, random passwords, PINs or memorable passphrases.\n\n\
                  Run without arguments to launch the GUI. With a style or a size option it works on the command \
                  line instead, using the settings saved by the GUI; the options below override them for one run.",
    after_help = "Styles:\n  \
                  random         every character random:           q7@Mdf#2kTae!9xH   (-n 12..256)\n  \
                  segmented      same-case groups of 3 letters:    asd2qwe+YUI6xcv4   (-n 12..256)\n  \
                  grouped        Apple-style blocks of 6:          hupvew-7hiTwa-qasnob (-g 2..5)\n  \
                  pronounceable  made-up words of syllables:       Kavo-temi48-Rupa-zeku (-g 3..8)\n  \
                  pin            digits only:                      583061             (-n 4..12)\n  \
                  words          a passphrase of common words:     Mango-Otter-Kettle-Robin (-w 3..8)\n\n\
                  Examples:\n  \
                  passgen -n 16                     a 16-character password in your default style\n  \
                  passgen -s random -n 20 --no-special\n  \
                  passgen -s pin -n 6               a 6-digit PIN\n  \
                  passgen -s grouped                Apple-style, with your saved number of groups\n  \
                  passgen -w 4 --number             e.g. Mango-42-Otter-Kettle-Robin\n  \
                  passgen -n 16 -c 10 --no-copy     print ten passwords, don't touch the clipboard"
)]
#[command(group(clap::ArgGroup::new("cli").multiple(true).args(["style", "length", "groups", "words"])))]
pub struct CliArgs {
    /// The style to generate (default: the default style from the settings)
    #[arg(short = 's', long, value_enum, value_name = "STYLE")]
    pub style: Option<Style>,

    /// Length, for the random, segmented and pin styles
    #[arg(short = 'n', long = "length", value_name = "LENGTH", group = "size")]
    pub length: Option<usize>,

    /// Number of groups, for the grouped and pronounceable styles
    #[arg(short = 'g', long, value_name = "COUNT", group = "size")]
    pub groups: Option<usize>,

    /// Number of words; on its own, -w also selects the words style
    #[arg(short = 'w', long = "words", value_name = "COUNT", group = "size")]
    pub words: Option<usize>,

    /// How many to generate, one per line
    #[arg(short = 'c', long, value_name = "N", default_value_t = 1, requires = "cli",
          value_parser = clap::value_parser!(u32).range(1..=100))]
    pub count: u32,

    /// Don't copy the result to the clipboard
    #[arg(long, requires = "cli")]
    pub no_copy: bool,

    /// Don't add the result to the history
    #[arg(long, requires = "cli")]
    pub no_history: bool,

    /// Leave out lowercase letters
    #[arg(long, requires = "cli", help_heading = "Random and segmented styles")]
    pub no_lowercase: bool,

    /// Leave out uppercase letters
    #[arg(long, requires = "cli", help_heading = "Random and segmented styles")]
    pub no_uppercase: bool,

    /// Leave out numbers
    #[arg(long, requires = "cli", help_heading = "Random and segmented styles")]
    pub no_numbers: bool,

    /// Leave out special characters
    #[arg(long, requires = "cli", help_heading = "Random and segmented styles")]
    pub no_special: bool,

    /// Include a two-digit number
    #[arg(long, requires = "cli", conflicts_with = "no_number", help_heading = "Pronounceable and words styles")]
    pub number: bool,

    /// Don't include a number
    #[arg(long, requires = "cli", help_heading = "Pronounceable and words styles")]
    pub no_number: bool,

    /// Put this character between words
    #[arg(long, value_name = "CHAR", requires = "cli", conflicts_with = "no_separator",
          help_heading = "Words style")]
    pub separator: Option<char>,

    /// Join the words without a separator
    #[arg(long, requires = "cli", help_heading = "Words style")]
    pub no_separator: bool,

    /// Start every word with an uppercase letter
    #[arg(long, requires = "cli", conflicts_with = "no_capitalize", help_heading = "Words style")]
    pub capitalize: bool,

    /// Keep every word in lowercase
    #[arg(long, requires = "cli", help_heading = "Words style")]
    pub no_capitalize: bool,

    /// Use this wordlist (one word per line) instead of the saved one
    #[arg(long, value_name = "FILE", requires = "cli", help_heading = "Words style")]
    pub wordlist: Option<PathBuf>,
}

impl CliArgs {
    /// Whether any option asks for command-line mode instead of the GUI.
    pub fn wants_cli(&self) -> bool {
        self.style.is_some() || self.length.is_some() || self.groups.is_some() || self.words.is_some()
    }
}

/// The main application state for the GUI.
pub struct PassGenApp {
    // General state
    pub active_style: Style,
    pub status_message: String,
    pub config_path: PathBuf,
    pub show_settings_window: bool,
    pub fitted_height: f32, // Window height last fitted to the content (0 before the first fit)
    pub generate_error: Option<String>, // Why the last generation failed, shown under the button
    pub clipboard_clear_at: Option<(Instant, String)>, // When to clear the clipboard, and what we copied
    pub copied_flash: Option<(Instant, String)>, // When something was copied, for the ✓ on its button
    pub started: bool, // Whether the first password has been generated and copied
    pub style_keys: Vec<StyleKey>, // Style-switching keys pressed since the last frame
    pub unrecorded: Option<(Instant, String)>, // A password shown after an option change, not in the history yet

    // Settings, and the version last written to config.toml
    pub config: Config,
    pub saved_config: Config,
    pub themes: Vec<Theme>, // Built-in themes and those in the themes folder
    pub words_file_path_str: String, // Text in the wordlist path box while it is being edited

    // Generator state
    pub size_input: String, // Text in the length box next to a slider while it is being edited
    pub outputs: HashMap<Style, String>, // The password currently shown for each style
    pub words: Vec<String>,
    pub wordlist_status: String, // Result of loading the wordlist, shown if it failed

    // Recently generated passwords
    pub history: History,
    pub show_history: bool,
}

impl Default for PassGenApp {
    /// Initializes the application with hard-coded default values.
    fn default() -> Self {
        Self {
            active_style: Style::Segmented,
            status_message: "Default settings loaded.".to_string(),
            config_path: PathBuf::new(),
            show_settings_window: false,
            fitted_height: 0.0,
            generate_error: None,
            clipboard_clear_at: None,
            copied_flash: None,
            started: false,
            style_keys: Vec::new(),
            unrecorded: None,
            config: Config::default(),
            saved_config: Config::default(),
            themes: Vec::new(),
            words_file_path_str: String::new(),
            size_input: String::new(),
            outputs: HashMap::new(),
            words: Vec::new(),
            wordlist_status: String::new(),
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

        let themes_dir = themes::themes_directory(&app_dir);
        messages.extend(themes::create_default_themes(&themes_dir));
        let (themes, theme_messages) = themes::load_themes(&themes_dir);
        messages.extend(theme_messages);
        if themes::find(&themes, &config.theme).is_none() {
            messages.push(format!("Theme {:?} not found in the themes folder, using Dark.", config.theme));
        }

        Self {
            active_style: config.default_style,
            status_message: messages.join("\n"),
            config_path,
            words_file_path_str: config.words_file_path.to_string_lossy().to_string(),
            saved_config: config.clone(),
            config,
            themes,
            words,
            wordlist_status: word_status,
            history,
            ..Self::default()
        }
    }
}
