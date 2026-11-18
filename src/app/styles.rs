//! The password styles other than Random and Words, and one entry point for all styles.

use crate::app::config::{CharacterSettings, Config, GroupedSettings, PinSettings, PronounceableSettings};
use crate::app::password::{
    char_settings_problem, generate_char_password, generate_word_password, max_repeats_for, word_settings_problem,
};
use crate::app::state::Style;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::ops::RangeInclusive;

/// How many dash-separated blocks a Grouped password can have.
pub const GROUPS_RANGE: RangeInclusive<usize> = 2..=5;
/// How many made-up words a Pronounceable password can have.
pub const PRONOUNCEABLE_GROUPS_RANGE: RangeInclusive<usize> = 3..=8;
/// How many digits a PIN can have.
pub const PIN_LENGTH_RANGE: RangeInclusive<usize> = 4..=12;

/// Letters per segment in the Segmented style (longer passwords get some 4-letter segments).
const SEGMENT_LETTERS: usize = 3;
/// In the Segmented style, the share of segments closed by a symbol rather than a number.
const SYMBOL_CLOSER_SHARE: f64 = 0.25;
/// Characters per block in the Grouped style.
const GROUP_SIZE: usize = 6;
/// Consonants and vowels for the Pronounceable style: easy to say and to spell out.
const CONSONANTS: &str = "bdfghjkmnprstvz";
const VOWELS: &str = "aeiou";
/// Syllables (consonant + vowel) per made-up word in the Pronounceable style.
const SYLLABLES_PER_GROUP: usize = 2;

/// Generates a password in `style` with the settings in `config`.
/// `words` is the loaded wordlist, used by the Words style.
pub fn generate(style: Style, config: &Config, words: &[String]) -> Result<String, String> {
    match style {
        Style::Random => generate_char_password(&config.characters),
        Style::Segmented => generate_segmented(&config.characters, config.segmented.length),
        Style::Grouped => generate_grouped(&config.grouped, &config.characters),
        Style::Pronounceable => generate_pronounceable(&config.pronounceable, &config.characters),
        Style::Pin => generate_pin(&config.pin),
        Style::Words => generate_word_password(&config.words, words),
    }
}

/// Explains why no password can be generated in `style` with these settings, if that's the case.
pub fn problem(style: Style, config: &Config, wordlist_size: usize) -> Option<String> {
    match style {
        Style::Random => char_settings_problem(&config.characters),
        Style::Segmented => char_settings_problem(&with_length(&config.characters, config.segmented.length)),
        Style::Grouped => grouped_problem(&config.grouped, &config.characters),
        Style::Pronounceable => pronounceable_problem(&config.pronounceable, &config.characters),
        Style::Pin => range_problem("A PIN", "digits", config.pin.length, &PIN_LENGTH_RANGE),
        Style::Words => word_settings_problem(&config.words, wordlist_size),
    }
}

/// The character settings with a different length.
fn with_length(sets: &CharacterSettings, length: usize) -> CharacterSettings {
    CharacterSettings { length, ..sets.clone() }
}

fn range_problem(what: &str, unit: &str, value: usize, range: &RangeInclusive<usize>) -> Option<String> {
    (!range.contains(&value))
        .then(|| format!("{} must have between {} and {} {}.", what, range.start(), range.end(), unit))
}

/// Distinct characters of a set, so every one is equally likely.
fn distinct(set: &str) -> Vec<char> {
    let mut chars: Vec<char> = set.chars().collect();
    chars.sort_unstable();
    chars.dedup();
    chars
}

/// Picks a character from `set` that differs from `previous` and hasn't reached `max_repeats`.
fn pick(
    set: &[char],
    previous: Option<char>,
    counts: &mut HashMap<char, usize>,
    max_repeats: usize,
    rng: &mut impl Rng,
) -> Option<char> {
    let allowed: Vec<char> = set
        .iter()
        .copied()
        .filter(|c| Some(*c) != previous && counts.get(c).copied().unwrap_or(0) < max_repeats)
        .collect();
    let &c = allowed.choose(rng)?;
    *counts.entry(c).or_insert(0) += 1;
    Some(c)
}

// --- Segmented -------------------------------------------------------------------------

/// Generates a Segmented password such as `asd2qwe+YUI6xcv4`: runs of 3 letters of the same
/// case, each closed by a number (usually) or a symbol, always ending with a closer. Every
/// selected set appears at least once. If no letter set or no number/symbol set is selected,
/// there's nothing to segment, so it falls back to the Random style.
pub fn generate_segmented(sets: &CharacterSettings, length: usize) -> Result<String, String> {
    let settings = with_length(sets, length);
    if let Some(problem) = char_settings_problem(&settings) {
        return Err(problem);
    }
    let selected = |enabled: bool, set: &str| if enabled { Some(distinct(set)) } else { None };
    let letter_sets: Vec<Vec<char>> = [
        selected(sets.use_lowercase, &sets.lowercase),
        selected(sets.use_uppercase, &sets.uppercase),
    ]
    .into_iter()
    .flatten()
    .collect();
    let numbers = selected(sets.use_numbers, &sets.numbers);
    let symbols = selected(sets.use_special, &sets.special);
    if letter_sets.is_empty() || (numbers.is_none() && symbols.is_none()) {
        return generate_char_password(&settings);
    }

    // One closer per segment; the letters are shared out as evenly as possible.
    let segments = length / (SEGMENT_LETTERS + 1);
    let letters = length - segments;
    let segment_sizes: Vec<usize> =
        (0..segments).map(|i| letters / segments + usize::from(i < letters % segments)).collect();
    let pool_size = letter_sets.iter().chain(&numbers).chain(&symbols).map(Vec::len).sum();
    let max_repeats = max_repeats_for(length, pool_size);

    // With both numbers and symbols, a fixed share of the closers are symbols (at least one),
    // so numbers dominate without ever leaving symbols out: 16 characters get 3 numbers, 1 symbol.
    let symbol_count = match (&numbers, &symbols) {
        (Some(_), Some(_)) => (((segments as f64) * SYMBOL_CLOSER_SHARE).round() as usize).clamp(1, segments - 1),
        (None, Some(_)) => segments,
        _ => 0,
    };

    let mut rng = thread_rng();
    'attempt: for _ in 0..1000 {
        // Every selected letter case must appear somewhere.
        let cases: Vec<usize> = (0..segments).map(|_| rng.gen_range(0..letter_sets.len())).collect();
        if !(0..letter_sets.len()).all(|set| cases.contains(&set)) {
            continue;
        }
        let mut closer_is_symbol: Vec<bool> = (0..segments).map(|i| i < symbol_count).collect();
        closer_is_symbol.shuffle(&mut rng);

        let mut counts = HashMap::new();
        let mut password = String::with_capacity(length);
        let mut previous = None;
        for segment in 0..segments {
            for _ in 0..segment_sizes[segment] {
                let Some(c) = pick(&letter_sets[cases[segment]], previous, &mut counts, max_repeats, &mut rng) else {
                    continue 'attempt;
                };
                password.push(c);
                previous = Some(c);
            }
            let closers = if closer_is_symbol[segment] { &symbols } else { &numbers };
            let closers = closers.as_ref().expect("only sets that are selected are used");
            let Some(c) = pick(closers, previous, &mut counts, max_repeats, &mut rng) else {
                continue 'attempt;
            };
            password.push(c);
            previous = Some(c);
        }
        return Ok(password);
    }
    Err("Failed to generate a segmented password. Try adding more characters to the sets.".to_string())
}

// --- Grouped -----------------------------------------------------------------------------

fn grouped_problem(settings: &GroupedSettings, sets: &CharacterSettings) -> Option<String> {
    range_problem("A grouped password", "groups", settings.groups, &GROUPS_RANGE).or_else(|| {
        if distinct(&sets.lowercase).len() < 2 || sets.uppercase.is_empty() || sets.numbers.is_empty() {
            Some("The Grouped style needs lowercase (at least 2), uppercase and number characters.".to_string())
        } else {
            None
        }
    })
}

/// Generates a Grouped password such as `hupvew-7hiTwa-qasnob`, like Apple's suggested
/// passwords: blocks of 6 lowercase letters joined by dashes, with exactly one uppercase
/// letter and one digit at random positions. Uses the character sets from the settings.
pub fn generate_grouped(settings: &GroupedSettings, sets: &CharacterSettings) -> Result<String, String> {
    if let Some(problem) = grouped_problem(settings, sets) {
        return Err(problem);
    }
    let (lowercase, uppercase, numbers) = (distinct(&sets.lowercase), distinct(&sets.uppercase), distinct(&sets.numbers));
    let total = settings.groups * GROUP_SIZE;
    let mut rng = thread_rng();

    let mut positions: Vec<usize> = (0..total).collect();
    positions.shuffle(&mut rng);
    let (upper_at, digit_at) = (positions[0], positions[1]);

    let mut counts = HashMap::new();
    let max_repeats = max_repeats_for(total, lowercase.len());
    let mut chars = Vec::with_capacity(total);
    for i in 0..total {
        // Only compare neighbours within a block; the dash separates blocks.
        let previous = if i % GROUP_SIZE == 0 { None } else { chars.last().copied() };
        let set = if i == upper_at {
            &uppercase
        } else if i == digit_at {
            &numbers
        } else {
            &lowercase
        };
        let c = pick(set, previous, &mut counts, max_repeats, &mut rng)
            .or_else(|| set.choose(&mut rng).copied())
            .expect("sets are not empty");
        chars.push(c);
    }

    let blocks: Vec<String> = chars.chunks(GROUP_SIZE).map(|block| block.iter().collect()).collect();
    Ok(blocks.join("-"))
}

// --- Pronounceable -----------------------------------------------------------------------

fn pronounceable_problem(settings: &PronounceableSettings, sets: &CharacterSettings) -> Option<String> {
    range_problem("A pronounceable password", "groups", settings.groups, &PRONOUNCEABLE_GROUPS_RANGE).or_else(|| {
        (settings.include_number && sets.numbers.is_empty())
            .then(|| "The number set is empty, so no number can be included.".to_string())
    })
}

/// Generates a Pronounceable password such as `Kavo-temi48-Rupa-zeku`: made-up words of easy
/// consonant-vowel syllables, some starting with a capital (at least one), optionally with a
/// two-digit number from the number set after one of the words.
pub fn generate_pronounceable(settings: &PronounceableSettings, sets: &CharacterSettings) -> Result<String, String> {
    if let Some(problem) = pronounceable_problem(settings, sets) {
        return Err(problem);
    }
    let consonants: Vec<char> = CONSONANTS.chars().collect();
    let vowels: Vec<char> = VOWELS.chars().collect();
    let numbers = distinct(&sets.numbers);
    let mut rng = thread_rng();

    let mut capitals: Vec<bool> = (0..settings.groups).map(|_| rng.gen_bool(0.5)).collect();
    if !capitals.contains(&true) {
        capitals[rng.gen_range(0..settings.groups)] = true;
    }
    let number_after = settings.include_number.then(|| rng.gen_range(0..settings.groups));

    let groups: Vec<String> = (0..settings.groups)
        .map(|group| {
            let mut word = String::new();
            for _ in 0..SYLLABLES_PER_GROUP {
                word.push(*consonants.choose(&mut rng).expect("not empty"));
                word.push(*vowels.choose(&mut rng).expect("not empty"));
            }
            if capitals[group] {
                word = word[..1].to_uppercase() + &word[1..];
            }
            if number_after == Some(group) {
                for _ in 0..2 {
                    word.push(*numbers.choose(&mut rng).expect("checked above"));
                }
            }
            word
        })
        .collect();
    Ok(groups.join("-"))
}

// --- PIN ---------------------------------------------------------------------------------

/// Generates a PIN of random digits 0-9, avoiding the most commonly guessed patterns:
/// one repeated digit (`111111`) and straight runs (`123456`, `654321`).
pub fn generate_pin(settings: &PinSettings) -> Result<String, String> {
    if let Some(problem) = range_problem("A PIN", "digits", settings.length, &PIN_LENGTH_RANGE) {
        return Err(problem);
    }
    let mut rng = thread_rng();
    loop {
        let digits: Vec<u8> = (0..settings.length).map(|_| rng.gen_range(0..10)).collect();
        if !is_obvious_pin(&digits) {
            return Ok(digits.iter().map(|d| char::from(b'0' + d)).collect());
        }
    }
}

/// Whether the digits are all the same or a straight ascending or descending run.
fn is_obvious_pin(digits: &[u8]) -> bool {
    let steps: Vec<i8> = digits.windows(2).map(|pair| pair[1] as i8 - pair[0] as i8).collect();
    steps.iter().all(|&step| step == 0) || steps.iter().all(|&step| step == 1) || steps.iter().all(|&step| step == -1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sets() -> CharacterSettings {
        CharacterSettings::default()
    }

    /// Splits a segmented password into (letters, closer) pairs.
    fn segments(password: &str) -> Vec<(String, char)> {
        let mut result = Vec::new();
        let mut letters = String::new();
        for c in password.chars() {
            if c.is_ascii_alphabetic() {
                letters.push(c);
            } else {
                result.push((std::mem::take(&mut letters), c));
            }
        }
        assert!(letters.is_empty(), "{} should end with a number or symbol", password);
        result
    }

    #[test]
    fn segmented_has_same_case_segments_closed_by_a_number_or_symbol() {
        for length in [12, 13, 14, 15, 16, 18, 20, 32, 64, 256] {
            for _ in 0..50 {
                let password = generate_segmented(&sets(), length).unwrap();
                assert_eq!(password.chars().count(), length);
                for (letters, _) in segments(&password) {
                    assert!((3..=4).contains(&letters.len()), "{}: segment {:?}", password, letters);
                    let lower = letters.chars().all(|c| c.is_ascii_lowercase());
                    let upper = letters.chars().all(|c| c.is_ascii_uppercase());
                    assert!(lower || upper, "{}: mixed case in {:?}", password, letters);
                }
            }
        }
    }

    #[test]
    fn segmented_includes_every_selected_kind_and_prefers_numbers() {
        let s = sets();
        for (length, expected_symbols) in [(12, 1), (16, 1), (20, 1), (24, 2), (32, 2)] {
            for _ in 0..200 {
                let password = generate_segmented(&s, length).unwrap();
                for set in [&s.lowercase, &s.uppercase, &s.numbers, &s.special] {
                    assert!(password.chars().any(|c| set.contains(c)), "{} lacks {}", password, set);
                }
                let symbols = segments(&password).iter().filter(|(_, closer)| !closer.is_ascii_digit()).count();
                assert_eq!(symbols, expected_symbols, "{}", password);
            }
        }
    }

    #[test]
    fn segmented_falls_back_to_random_without_closers() {
        let letters_only = CharacterSettings { use_numbers: false, use_special: false, ..sets() };
        let password = generate_segmented(&letters_only, 16).unwrap();
        assert_eq!(password.len(), 16);
        assert!(password.chars().all(|c| c.is_ascii_alphabetic()));
    }

    #[test]
    fn grouped_has_blocks_with_one_capital_and_one_digit() {
        for groups in GROUPS_RANGE {
            for _ in 0..100 {
                let password = generate_grouped(&GroupedSettings { groups }, &sets()).unwrap();
                let blocks: Vec<&str> = password.split('-').collect();
                assert_eq!(blocks.len(), groups);
                assert!(blocks.iter().all(|b| b.len() == GROUP_SIZE), "{}", password);
                assert_eq!(password.chars().filter(char::is_ascii_uppercase).count(), 1, "{}", password);
                assert_eq!(password.chars().filter(char::is_ascii_digit).count(), 1, "{}", password);
            }
        }
    }

    #[test]
    fn pronounceable_alternates_consonants_and_vowels() {
        for include_number in [false, true] {
            for _ in 0..100 {
                let settings = PronounceableSettings { groups: 4, include_number };
                let password = generate_pronounceable(&settings, &sets()).unwrap();
                let groups: Vec<&str> = password.split('-').collect();
                assert_eq!(groups.len(), 4);
                assert!(password.chars().any(|c| c.is_ascii_uppercase()), "{}", password);
                assert_eq!(password.chars().filter(char::is_ascii_digit).count(), if include_number { 2 } else { 0 });
                for group in groups {
                    let letters: Vec<char> = group.chars().filter(|c| c.is_alphabetic()).map(|c| c.to_ascii_lowercase()).collect();
                    assert_eq!(letters.len(), SYLLABLES_PER_GROUP * 2);
                    for pair in letters.chunks(2) {
                        assert!(CONSONANTS.contains(pair[0]) && VOWELS.contains(pair[1]), "{}", password);
                    }
                }
            }
        }
    }

    #[test]
    fn pin_has_digits_only_and_avoids_obvious_patterns() {
        for length in PIN_LENGTH_RANGE {
            for _ in 0..100 {
                let pin = generate_pin(&PinSettings { length }).unwrap();
                assert_eq!(pin.len(), length);
                assert!(pin.chars().all(|c| c.is_ascii_digit()));
            }
        }
        assert!(is_obvious_pin(&[4, 4, 4, 4]));
        assert!(is_obvious_pin(&[1, 2, 3, 4, 5, 6]));
        assert!(is_obvious_pin(&[9, 8, 7, 6]));
        assert!(!is_obvious_pin(&[5, 8, 3, 0, 6, 1]));
        assert!(generate_pin(&PinSettings { length: 3 }).is_err());
        assert!(generate_pin(&PinSettings { length: 13 }).is_err());
    }

    #[test]
    fn only_pin_allows_short_lengths() {
        let mut config = Config::default();
        config.pin.length = 4;
        assert_eq!(problem(Style::Pin, &config, 0), None);
        config.segmented.length = 4;
        assert!(problem(Style::Segmented, &config, 0).is_some());
        config.characters.length = 4;
        assert!(problem(Style::Random, &config, 0).is_some());
    }

    #[test]
    fn every_style_generates_with_defaults() {
        let config = Config::default();
        let words: Vec<String> = ["apple", "otter", "kettle", "robin", "cactus"].iter().map(|w| w.to_string()).collect();
        for style in Style::ALL {
            assert_eq!(problem(style, &config, words.len()), None, "{:?}", style);
            assert!(generate(style, &config, &words).is_ok(), "{:?}", style);
        }
    }
}
