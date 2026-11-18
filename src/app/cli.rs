use crate::app::config::load_config_map;
use crate::app::history::History;
use crate::app::password::{generate_char_password, generate_word_password, MAX_WORDS, MIN_WORDS};
use crate::app::state::CharacterSets;
use crate::app::utils::{get_config_directory, load_words};
use arboard::Clipboard;
use std::path::Path;

/// Launches the command-line interface for character passwords.
pub fn run_char_cli_mode(length: usize) {
    let app_dir = get_config_directory();
    let config_path = app_dir.join("config.toml");
    let config = load_config_map(&config_path);
    let defaults = CharacterSets::default();
    let sets = CharacterSets {
        lowercase: config.get("lowercase_chars").cloned().unwrap_or(defaults.lowercase),
        uppercase: config.get("uppercase_chars").cloned().unwrap_or(defaults.uppercase),
        numbers: config.get("number_chars").cloned().unwrap_or(defaults.numbers),
        special: config.get("special_chars").cloned().unwrap_or(defaults.special),
    };

    match generate_char_password(length, &sets, true, true, true, true) {
        Ok(password) => {
            println!("{}", password);
            remember(&password);
            if let Ok(mut clipboard) = Clipboard::new() {
                if clipboard.set_text(password).is_ok() {
                    eprintln!("Password copied to clipboard.");
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

/// Launches the command-line interface for word passphrases.
pub fn run_word_cli_mode(count: usize) {
    if !(MIN_WORDS..=MAX_WORDS).contains(&count) {
        eprintln!("Error: Word count for -w flag must be between {} and {}.", MIN_WORDS, MAX_WORDS);
        return;
    }

    let (words, _) = load_words(Path::new(""));

    match generate_word_password(count, &words, true, "-", true, false) {
        Ok(password) => {
            println!("{}", password);
            remember(&password);
            if let Ok(mut clipboard) = Clipboard::new() {
                if clipboard.set_text(password).is_ok() {
                    eprintln!("Passphrase copied to clipboard.");
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

/// Adds a CLI-generated password to the shared history, reporting problems on stderr.
fn remember(password: &str) {
    let (mut history, warning) = History::load(get_config_directory().join("history.dat"));
    if let Some(warning) = warning {
        eprintln!("{}", warning);
    }
    if let Err(e) = history.add(password) {
        eprintln!("Saving history failed: {}", e);
    }
}
