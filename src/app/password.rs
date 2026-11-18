use crate::app::config::{CharacterSettings, WordSettings};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::collections::HashMap;

/// The shortest and longest character passwords.
pub const MIN_LENGTH: usize = 12;
pub const MAX_LENGTH: usize = 256;

/// Generates a password from random characters.
///
/// Every character is picked independently from the combined pool of selected sets.
/// No character appears twice in a row, and no character appears more than
/// `max_repeats_for` times, so passwords of any length can be generated.
pub fn generate_char_password(settings: &CharacterSettings) -> Result<String, String> {
    let length = settings.length;
    let selected_sets = selected_sets(settings);

    if selected_sets.is_empty() {
        return Err("You must select at least one character set.".to_string());
    }
    if length < selected_sets.len() {
        return Err(format!(
            "Password length must be at least {} to include one of each selected type.",
            selected_sets.len()
        ));
    }

    let pool = character_pool(&selected_sets);
    let max_repeats = max_repeats_for(length, pool.len());

    let mut rng = thread_rng();
    for _ in 0..1000 {
        let Some(candidate) = build_candidate(length, &pool, max_repeats, &mut rng) else {
            continue;
        };
        let has_every_set = selected_sets
            .iter()
            .all(|set| candidate.iter().any(|c| set.contains(c)));
        if has_every_set {
            return Ok(candidate.into_iter().collect());
        }
    }

    Err("Failed to generate a valid password after multiple attempts. Try adding more characters to the sets.".to_string())
}

/// The enabled, non-empty character sets.
fn selected_sets(settings: &CharacterSettings) -> Vec<Vec<char>> {
    [
        (settings.use_lowercase, &settings.lowercase),
        (settings.use_uppercase, &settings.uppercase),
        (settings.use_numbers, &settings.numbers),
        (settings.use_special, &settings.special),
    ]
    .into_iter()
    .filter(|(enabled, set)| *enabled && !set.is_empty())
    .map(|(_, set)| set.chars().collect())
    .collect()
}

/// All distinct characters across the selected sets.
/// Deduplicated so every character is equally likely, even if sets overlap.
fn character_pool(selected_sets: &[Vec<char>]) -> Vec<char> {
    let mut pool = selected_sets.concat();
    pool.sort_unstable();
    pool.dedup();
    pool
}

/// The most times a single character may appear. Short passwords keep the limit of 3;
/// longer ones raise it just enough that the pool can always fill the requested length.
fn max_repeats_for(length: usize, pool_size: usize) -> usize {
    3.max(length.div_ceil(pool_size) + 1)
}

/// Builds one random password, or `None` if the rules left no valid character to pick.
fn build_candidate(length: usize, pool: &[char], max_repeats: usize, rng: &mut impl Rng) -> Option<Vec<char>> {
    let mut counts: HashMap<char, usize> = HashMap::new();
    let mut password = Vec::with_capacity(length);

    for _ in 0..length {
        let previous = password.last().copied();
        let allowed: Vec<char> = pool
            .iter()
            .copied()
            .filter(|c| pool.len() == 1 || Some(*c) != previous)
            .filter(|c| counts.get(c).copied().unwrap_or(0) < max_repeats)
            .collect();
        let &next = allowed.choose(rng)?;
        *counts.entry(next).or_insert(0) += 1;
        password.push(next);
    }

    Some(password)
}

/// The fewest and most words a passphrase can have.
pub const MIN_WORDS: usize = 3;
pub const MAX_WORDS: usize = 8;

/// The range of the optional number inserted into a passphrase.
const PASSPHRASE_NUMBERS: std::ops::RangeInclusive<u32> = 10..=99;

/// Generates a passphrase from a list of words, respecting length constraints.
/// With `include_number`, a two-digit number is inserted at a random position.
pub fn generate_word_password(settings: &WordSettings, words: &[String]) -> Result<String, String> {
    let count = settings.count;
    if !(MIN_WORDS..=MAX_WORDS).contains(&count) {
        return Err(format!("Word count must be between {} and {}.", MIN_WORDS, MAX_WORDS));
    }
    if words.len() < count {
        return Err(format!(
            "Not enough words in the wordlist (found {}, need at least {}).",
            words.len(),
            count
        ));
    }

    // Keeps passphrases from unusually long custom wordlists manageable;
    // the built-in list never comes close to this.
    let max_length = count * 10 + if settings.include_number { 3 } else { 0 };
    let mut rng = thread_rng();

    for _ in 0..100 {
        let mut parts: Vec<String> = words
            .choose_multiple(&mut rng, count)
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        if settings.capitalize {
                            first.to_uppercase().to_string() + chars.as_str()
                        } else {
                            first.to_lowercase().to_string() + chars.as_str()
                        }
                    }
                }
            })
            .collect();

        if settings.include_number {
            let position = rng.gen_range(0..=parts.len());
            parts.insert(position, rng.gen_range(PASSPHRASE_NUMBERS).to_string());
        }

        let separator_str = if settings.use_separator { settings.separator.as_str() } else { "" };
        let passphrase = parts.join(separator_str);

        if passphrase.chars().count() <= max_length {
            return Ok(passphrase);
        }
    }

    Err(format!(
        "Could not generate a passphrase under {} characters. Check the wordlist for long words.",
        max_length
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate(length: usize) -> String {
        generate_char_password(&CharacterSettings { length, ..Default::default() }).unwrap()
    }

    fn words(count: usize, capitalize: bool, include_number: bool) -> WordSettings {
        WordSettings { count, capitalize, include_number, ..Default::default() }
    }

    #[test]
    fn produces_requested_length_up_to_256() {
        for length in [12, 16, 64, 92, 93, 128, 200, 256] {
            for _ in 0..50 {
                assert_eq!(generate(length).chars().count(), length);
            }
        }
    }

    #[test]
    fn never_repeats_a_character_back_to_back() {
        for _ in 0..200 {
            let password: Vec<char> = generate(256).chars().collect();
            assert!(password.windows(2).all(|pair| pair[0] != pair[1]), "{:?}", password);
        }
    }

    #[test]
    fn respects_repeat_limit() {
        let pool_size = character_pool(&selected_sets(&CharacterSettings::default())).len();
        for length in [16, 128, 256] {
            let limit = max_repeats_for(length, pool_size);
            for _ in 0..100 {
                let mut counts = HashMap::new();
                for c in generate(length).chars() {
                    *counts.entry(c).or_insert(0) += 1;
                }
                assert!(counts.values().all(|&n| n <= limit));
            }
        }
        assert_eq!(max_repeats_for(16, pool_size), 3);
    }

    #[test]
    fn default_sets_avoid_hard_to_read_characters() {
        let defaults = CharacterSettings::default();
        let all = format!("{}{}{}{}", defaults.lowercase, defaults.uppercase, defaults.numbers, defaults.special);
        for c in "0Oo1lIB'\"`|;:,._^~\\/()[]{}<>CKPSUVWXZ".chars() {
            assert!(!all.contains(c), "default sets should not contain {:?}", c);
        }
    }

    #[test]
    fn includes_every_selected_set() {
        let sets = CharacterSettings::default();
        for _ in 0..500 {
            let password = generate(12);
            for set in [&sets.lowercase, &sets.uppercase, &sets.numbers, &sets.special] {
                assert!(password.chars().any(|c| set.contains(c)), "{} lacks {}", password, set);
            }
        }
    }

    #[test]
    fn small_custom_set_still_works() {
        let settings = CharacterSettings {
            length: 40,
            use_lowercase: false,
            use_uppercase: false,
            use_special: false,
            numbers: "01".to_string(),
            ..Default::default()
        };
        let password = generate_char_password(&settings).unwrap();
        assert_eq!(password.len(), 40);
    }

    fn wordlist() -> Vec<String> {
        ["apple", "otter", "kettle", "robin", "cactus", "tulip", "mango", "piano", "comet", "maple"]
            .iter()
            .map(|w| w.to_string())
            .collect()
    }

    #[test]
    fn passphrase_has_requested_words() {
        for count in MIN_WORDS..=MAX_WORDS {
            let passphrase = generate_word_password(&words(count, true, false), &wordlist()).unwrap();
            let parts: Vec<&str> = passphrase.split('-').collect();
            assert_eq!(parts.len(), count);
            assert!(parts.iter().all(|p| p.chars().next().unwrap().is_uppercase()));
        }
    }

    #[test]
    fn passphrase_includes_one_number_when_asked() {
        for _ in 0..200 {
            let passphrase = generate_word_password(&words(4, false, true), &wordlist()).unwrap();
            let parts: Vec<&str> = passphrase.split('-').collect();
            assert_eq!(parts.len(), 5);
            let numbers: Vec<u32> = parts.iter().filter_map(|p| p.parse().ok()).collect();
            assert_eq!(numbers.len(), 1, "{}", passphrase);
            assert!(PASSPHRASE_NUMBERS.contains(&numbers[0]));
        }
    }

    #[test]
    fn passphrase_rejects_bad_word_counts() {
        assert!(generate_word_password(&words(MIN_WORDS - 1, true, false), &wordlist()).is_err());
        assert!(generate_word_password(&words(MAX_WORDS + 1, true, false), &wordlist()).is_err());
    }

    #[test]
    fn rejects_no_sets() {
        let settings = CharacterSettings {
            use_lowercase: false,
            use_uppercase: false,
            use_numbers: false,
            use_special: false,
            ..Default::default()
        };
        assert!(generate_char_password(&settings).is_err());
    }
}
