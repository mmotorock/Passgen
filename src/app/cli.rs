use crate::app::config::load_config_map;
use crate::app::history::History;
use crate::app::password::{generate_char_password, generate_word_password};
use crate::app::state::CharacterSets;
use crate::app::utils::{create_default_words_file, get_config_directory, load_words_from_file};
use arboard::Clipboard;

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
    if !(3..=5).contains(&count) {
        eprintln!("Error: Word count for -w flag must be 3, 4, or 5.");
        return;
    }

    let app_dir = get_config_directory();
    let words_file_path = app_dir.join("words.txt");
    create_default_words_file(&words_file_path);
    let (words, _) = load_words_from_file(&words_file_path);

    match generate_word_password(count, &words, true, "-", true) {
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
    let (mut history, status) = History::load(get_config_directory().join("history.dat"));
    if !status.is_empty() && !status.starts_with("Loaded") {
        eprintln!("{}", status);
    }
    let status = history.add(password);
    if !status.ends_with("added to history.") {
        eprintln!("{}", status);
    }
}
