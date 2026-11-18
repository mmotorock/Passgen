use crate::app::state::CharacterSets;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::collections::HashMap;

/// Generates a password from random characters.
///
/// Every character is picked independently from the combined pool of selected sets.
/// No character appears twice in a row, and no character appears more than
/// `max_repeats_for` times, so passwords of any length can be generated.
pub fn generate_char_password(
    length: usize,
    sets: &CharacterSets,
    use_lowercase: bool,
    use_uppercase: bool,
    use_numbers: bool,
    use_special: bool,
) -> Result<String, String> {
    let selected_sets = selected_sets(sets, use_lowercase, use_uppercase, use_numbers, use_special);

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

/// Estimates the strength of a character password, in bits of randomness.
/// The first character can be any in the pool; each later one any except the previous character.
pub fn char_entropy_bits(
    length: usize,
    sets: &CharacterSets,
    use_lowercase: bool,
    use_uppercase: bool,
    use_numbers: bool,
    use_special: bool,
) -> f64 {
    let selected = selected_sets(sets, use_lowercase, use_uppercase, use_numbers, use_special);
    let pool_size = character_pool(&selected).len() as f64;
    if length == 0 || pool_size < 2.0 {
        return 0.0;
    }
    pool_size.log2() + (length - 1) as f64 * (pool_size - 1.0).log2()
}

/// Estimates the strength of a passphrase, in bits of randomness.
pub fn word_entropy_bits(count: usize, wordlist_size: usize) -> f64 {
    if wordlist_size < 2 {
        return 0.0;
    }
    count as f64 * (wordlist_size as f64).log2()
}

/// Guesses per second assumed for an attacker: a GPU cracking rig working on
/// a leaked database of fast password hashes. A deliberately pessimistic figure.
pub const GUESSES_PER_SECOND: f64 = 1e11;

/// Seconds an attacker needs on average to guess a password of the given strength
/// (half of all possibilities), at `GUESSES_PER_SECOND`.
pub fn crack_time_seconds(bits: f64) -> f64 {
    2f64.powf(bits - 1.0) / GUESSES_PER_SECOND
}

/// Describes a crack time in plain words, e.g. "about 3 hours" or "about 24 thousand years".
pub fn describe_crack_time(seconds: f64) -> String {
    const MINUTE: f64 = 60.0;
    const HOUR: f64 = 60.0 * MINUTE;
    const DAY: f64 = 24.0 * HOUR;
    const YEAR: f64 = 365.25 * DAY;
    const AGE_OF_UNIVERSE_YEARS: f64 = 1.38e10;

    let plural = |value: f64, unit: &str| {
        let rounded = value.round() as u64;
        format!("about {} {}{}", rounded, unit, if rounded == 1 { "" } else { "s" })
    };

    if seconds < 1.0 {
        return "less than a second".to_string();
    }
    if seconds < MINUTE {
        return plural(seconds, "second");
    }
    if seconds < HOUR {
        return plural(seconds / MINUTE, "minute");
    }
    if seconds < DAY {
        return plural(seconds / HOUR, "hour");
    }
    if seconds < YEAR {
        return plural(seconds / DAY, "day");
    }

    let years = seconds / YEAR;
    if years >= AGE_OF_UNIVERSE_YEARS {
        return "longer than the age of the universe".to_string();
    }
    for (scale, name) in [(1e9, "billion"), (1e6, "million"), (1e3, "thousand")] {
        if years >= scale {
            return format!("about {} {} years", (years / scale).round() as u64, name);
        }
    }
    plural(years, "year")
}

/// The enabled, non-empty character sets.
fn selected_sets(
    sets: &CharacterSets,
    use_lowercase: bool,
    use_uppercase: bool,
    use_numbers: bool,
    use_special: bool,
) -> Vec<Vec<char>> {
    [
        (use_lowercase, &sets.lowercase),
        (use_uppercase, &sets.uppercase),
        (use_numbers, &sets.numbers),
        (use_special, &sets.special),
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

/// Generates a passphrase from a list of words, respecting length constraints.
pub fn generate_word_password(
    count: usize,
    words: &[String],
    use_separator: bool,
    separator: &str,
    use_uppercase: bool,
) -> Result<String, String> {
    if words.len() < count {
        return Err(format!(
            "Not enough words in words.txt (found {}, need at least {}).",
            words.len(),
            count
        ));
    }

    let max_length = match count {
        3 => 30,
        4 => 35,
        5 => 40,
        _ => return Err("Invalid word count.".to_string()),
    };

    for _ in 0..100 {
        let chosen_words: Vec<&String> = words.choose_multiple(&mut thread_rng(), count).collect();

        let processed_words: Vec<String> = chosen_words
            .iter()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        if use_uppercase {
                            first.to_uppercase().to_string() + &word[first.len_utf8()..]
                        } else {
                            first.to_lowercase().to_string() + &word[first.len_utf8()..]
                        }
                    }
                }
            })
            .collect();

        let separator_str = if use_separator { separator } else { "" };
        let passphrase = processed_words.join(separator_str);

        if passphrase.len() <= max_length {
            return Ok(passphrase);
        }
    }

    Err(format!(
        "Could not generate a passphrase under {} characters. Check words.txt for long words.",
        max_length
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate(length: usize) -> String {
        generate_char_password(length, &CharacterSets::default(), true, true, true, true).unwrap()
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
        for length in [16, 128, 256] {
            let limit = max_repeats_for(length, 88);
            for _ in 0..100 {
                let mut counts = HashMap::new();
                for c in generate(length).chars() {
                    *counts.entry(c).or_insert(0) += 1;
                }
                assert!(counts.values().all(|&n| n <= limit));
            }
        }
        assert_eq!(max_repeats_for(16, 88), 3);
    }

    #[test]
    fn includes_every_selected_set() {
        let sets = CharacterSets::default();
        for _ in 0..500 {
            let password = generate(12);
            for set in [&sets.lowercase, &sets.uppercase, &sets.numbers, &sets.special] {
                assert!(password.chars().any(|c| set.contains(c)), "{} lacks {}", password, set);
            }
        }
    }

    #[test]
    fn small_custom_set_still_works() {
        let sets = CharacterSets { numbers: "01".to_string(), ..CharacterSets::default() };
        let password = generate_char_password(40, &sets, false, false, true, false).unwrap();
        assert_eq!(password.len(), 40);
    }

    #[test]
    fn estimates_strength() {
        let sets = CharacterSets::default();
        let bits = char_entropy_bits(16, &sets, true, true, true, true);
        assert!((bits - (88f64.log2() + 15.0 * 87f64.log2())).abs() < 1e-9);
        assert_eq!(char_entropy_bits(16, &sets, false, false, false, false), 0.0);
        assert!((word_entropy_bits(4, 7776) - 4.0 * 7776f64.log2()).abs() < 1e-9);
    }

    #[test]
    fn crack_time_follows_character_variety() {
        let sets = CharacterSets::default();
        let time = |lower, upper, numbers, special| {
            crack_time_seconds(char_entropy_bits(12, &sets, lower, upper, numbers, special))
        };
        let numbers_only = time(false, false, true, false);
        let lowercase_only = time(true, false, false, false);
        let mixed_case = time(true, true, false, false);
        let everything = time(true, true, true, true);
        assert!(numbers_only < 60.0, "12 digits should fall within a minute");
        assert!(numbers_only < lowercase_only && lowercase_only < mixed_case && mixed_case < everything);
    }

    #[test]
    fn describes_crack_times() {
        assert_eq!(describe_crack_time(0.2), "less than a second");
        assert_eq!(describe_crack_time(1.0), "about 1 second");
        assert_eq!(describe_crack_time(3.0 * 3600.0), "about 3 hours");
        assert_eq!(describe_crack_time(24_000.0 * 365.25 * 86400.0), "about 24 thousand years");
        assert_eq!(describe_crack_time(1e30), "longer than the age of the universe");
    }

    #[test]
    fn rejects_no_sets() {
        assert!(generate_char_password(16, &CharacterSets::default(), false, false, false, false).is_err());
    }
}
