use crate::app::config::{CharacterSettings, Config, WordSettings, CONFIG_FILE};
use crate::app::history::History;
use crate::app::password::{
    generate_char_password, generate_word_password, MAX_LENGTH, MAX_WORDS, MIN_LENGTH, MIN_WORDS,
};
use crate::app::state::CliArgs;
use crate::app::utils::{get_config_directory, load_words};
use arboard::Clipboard;
use std::process::ExitCode;

/// Runs the command-line interface: generates `args.count` passwords or passphrases from the
/// saved settings, with any options in `args` applied on top, and prints one per line.
pub fn run_cli(args: &CliArgs) -> ExitCode {
    let config = load_config();
    let generate: Box<dyn Fn() -> Result<String, String>> = if let Some(length) = args.length {
        if !(MIN_LENGTH..=MAX_LENGTH).contains(&length) {
            return fail(&format!("Length for -n must be between {} and {}.", MIN_LENGTH, MAX_LENGTH));
        }
        let settings = character_settings(length, config.characters, args);
        Box::new(move || generate_char_password(&settings))
    } else if let Some(count) = args.words {
        if !(MIN_WORDS..=MAX_WORDS).contains(&count) {
            return fail(&format!("Word count for -w must be between {} and {}.", MIN_WORDS, MAX_WORDS));
        }
        let path = args.wordlist.clone().unwrap_or(config.words_file_path);
        let (words, word_status) = load_words(&path);
        if words.is_empty() {
            return fail(&word_status);
        }
        let settings = word_settings(count, config.words, args);
        Box::new(move || generate_word_password(&settings, &words))
    } else {
        unreachable!("run_cli is only called with -n or -w");
    };

    let mut passwords = Vec::new();
    for _ in 0..args.count {
        match generate() {
            Ok(password) => passwords.push(password),
            Err(e) => return fail(&e),
        }
    }
    for password in &passwords {
        println!("{}", password);
    }

    if !args.no_history {
        remember(&passwords);
    }
    if !args.no_copy {
        copy(&passwords.join("\n"), passwords.len());
    }
    ExitCode::SUCCESS
}

/// The saved character settings with the command-line overrides applied.
fn character_settings(length: usize, saved: CharacterSettings, args: &CliArgs) -> CharacterSettings {
    CharacterSettings {
        length,
        use_lowercase: saved.use_lowercase && !args.no_lowercase,
        use_uppercase: saved.use_uppercase && !args.no_uppercase,
        use_numbers: saved.use_numbers && !args.no_numbers,
        use_special: saved.use_special && !args.no_special,
        ..saved
    }
}

/// The saved passphrase settings with the command-line overrides applied.
fn word_settings(count: usize, saved: WordSettings, args: &CliArgs) -> WordSettings {
    let mut settings = WordSettings { count, ..saved };
    if let Some(separator) = args.separator {
        settings.use_separator = true;
        settings.separator = separator.to_string();
    }
    if args.no_separator {
        settings.use_separator = false;
    }
    if args.number || args.no_number {
        settings.include_number = args.number;
    }
    if args.capitalize || args.no_capitalize {
        settings.capitalize = args.capitalize;
    }
    settings
}

/// Loads the shared settings, reporting anything noteworthy on stderr.
fn load_config() -> Config {
    let (config, messages) = Config::load(&get_config_directory().join(CONFIG_FILE));
    for message in messages {
        eprintln!("{}", message);
    }
    config
}

/// Reports an error on stderr.
fn fail(message: &str) -> ExitCode {
    eprintln!("Error: {}", message);
    ExitCode::FAILURE
}

/// Adds CLI-generated passwords to the shared history, reporting problems on stderr.
fn remember(passwords: &[String]) {
    let (mut history, warning) = History::load(get_config_directory().join("history.dat"));
    if let Some(warning) = warning {
        eprintln!("{}", warning);
    }
    for password in passwords {
        if let Err(e) = history.add(password) {
            eprintln!("Saving history failed: {}", e);
            return;
        }
    }
}

/// Copies `text` (one or more passwords, one per line) to the clipboard.
fn copy(text: &str, count: usize) {
    let copied = Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text));
    match copied {
        Ok(()) if count == 1 => eprintln!("Copied to clipboard."),
        Ok(()) => eprintln!("Copied all {} to clipboard.", count),
        Err(e) => eprintln!("Could not copy to clipboard: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};

    fn parse(args: &str) -> CliArgs {
        CliArgs::try_parse_from(std::iter::once("passgen").chain(args.split_whitespace())).unwrap()
    }

    #[test]
    fn argument_definitions_are_consistent() {
        CliArgs::command().debug_assert();
    }

    #[test]
    fn character_flags_only_turn_sets_off() {
        let saved = CharacterSettings { use_numbers: false, ..Default::default() };
        let settings = character_settings(20, saved, &parse("-n 20 --no-special"));
        assert_eq!(settings.length, 20);
        assert!(settings.use_lowercase && settings.use_uppercase);
        assert!(!settings.use_numbers, "a set turned off in the settings stays off");
        assert!(!settings.use_special);
    }

    #[test]
    fn word_flags_override_saved_settings() {
        let saved = WordSettings { include_number: true, ..Default::default() };
        let settings = word_settings(5, saved.clone(), &parse("-w 5 --separator . --no-number --no-capitalize"));
        assert_eq!((settings.count, settings.separator.as_str()), (5, "."));
        assert!(settings.use_separator && !settings.include_number && !settings.capitalize);

        let unchanged = word_settings(4, saved.clone(), &parse("-w 4"));
        assert_eq!(unchanged, WordSettings { count: 4, ..saved });
    }

    #[test]
    fn rejects_options_for_the_other_mode() {
        for args in ["-n 16 --number", "-w 4 --no-special", "--number", "-c 3", "-n 16 -w 4", "-w 4 --number --no-number"] {
            let parsed = CliArgs::try_parse_from(std::iter::once("passgen").chain(args.split_whitespace()));
            assert!(parsed.is_err(), "{:?} should be rejected", args);
        }
    }
}
