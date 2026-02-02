use crate::tts::normalize;
use crate::tts::vocab::VOCAB;
use espeak_rs::text_to_phonemes;
use fancy_regex::Regex;
use lazy_static::lazy_static;
use regex;
use misaki_rs::G2P;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct PhonemeMap {
    pub map: HashMap<String, String>,
    pub combined_re: Regex,
}

impl PhonemeMap {
    pub fn new(map: HashMap<String, String>) -> Self {
        let mut entries: Vec<_> = map.keys().collect();
        // Sort by length descending to match longer phrases first
        entries.sort_by(|a, b| b.len().cmp(&a.len()));

        let dict_pattern = if entries.is_empty() {
            "a^".to_string() // Matches nothing
        } else {
            let escaped_entries: Vec<String> = entries.iter().map(|&s| regex::escape(s)).collect();
            format!(r"\b({})\b", escaped_entries.join("|"))
        };

        // Combine inline override and dictionary matching into a single-pass regex.
        // Group 1 & 2: inline override word and phonemes.
        // Group "dict": dictionary word match.
        let combined_pattern =
            format!(r"\[([^\]]+)\]\(/([^/]+)/\)|(?P<dict>{})", dict_pattern);
        let combined_re = Regex::new(&combined_pattern).unwrap();

        Self { map, combined_re }
    }
}

lazy_static! {
    static ref PHONEME_PATTERNS: Regex = Regex::new(r"(?<=[a-zɹː])(?=hˈʌndɹɪd)").unwrap();
    static ref Z_PATTERN: Regex = Regex::new(r#" z(?=[;:,.!?¡¿—…"«»"" ]|$)"#).unwrap();
    static ref NINETY_PATTERN: Regex = Regex::new(r"(?<=nˈaɪn)ti(?!ː)").unwrap();
    pub static ref OVERRIDE_RE: Regex = Regex::new(r"\[([^\]]+)\]\(/([^/]+)/\)").unwrap();
    pub static ref ESPEAK_MUTEX: Mutex<()> = Mutex::new(());
    static ref MISAKI_EN_US: G2P = G2P::new(false);
    static ref MISAKI_EN_GB: G2P = G2P::new(true);
}

pub struct Phonemizer {
    lang: String,
    pub phoneme_map: Option<Arc<PhonemeMap>>,
}

impl Phonemizer {
    pub fn new(lang: &str) -> Self {
        Phonemizer {
            lang: lang.to_string(),
            phoneme_map: None,
        }
    }

    pub fn with_phoneme_map(mut self, map: Arc<PhonemeMap>) -> Self {
        self.phoneme_map = Some(map);
        self
    }

    pub fn phonemize(&self, text: &str, normalize: bool) -> String {
        let text = if normalize {
            normalize::normalize_text(text)
        } else {
            text.to_string()
        };

        let mut ps = String::new();
        let mut last_match = 0;

        // Use the combined regex if a map is present, otherwise just OVERRIDE_RE
        let re = match &self.phoneme_map {
            Some(map) => &map.combined_re,
            None => &OVERRIDE_RE,
        };

        for mat_res in re.find_iter(&text) {
            if let Ok(mat) = mat_res {
                let start = mat.start();
                let end = mat.end();

                // Phonemize preceding plain text
                if start > last_match {
                    let segment = &text[last_match..start];
                    ps.push_str(&self.phonemize_segment(segment));
                }

                let caps = re.captures(&text[start..end]).unwrap().unwrap();
                if let Some(dict_match) = caps.name("dict") {
                    // Dictionary match
                    let word = dict_match.as_str();
                    if let Some(map) = &self.phoneme_map {
                        if let Some(phonemes) = map.map.get(word) {
                            ps.push_str(phonemes);
                        }
                    }
                } else {
                    // Inline override match: [word](/phonemes/)
                    // Group 1: word, Group 2: phonemes
                    if let Some(phonemes_cap) = caps.get(2) {
                        ps.push_str(phonemes_cap.as_str());
                    }
                }

                last_match = end;
            }
        }

        // Phonemize remaining plain text
        if last_match < text.len() {
            let segment = &text[last_match..];
            ps.push_str(&self.phonemize_segment(segment));
        }

        self.phonemize_postprocess(ps)
    }

    fn phonemize_segment(&self, text: &str) -> String {
        if text.trim().is_empty() {
            return text.to_string();
        }

        match self.lang.as_str() {
            "a" | "en-us" => MISAKI_EN_US.g2p(text).0,
            "b" | "en-gb" => MISAKI_EN_GB.g2p(text).0,
            _ => {
                let _guard = ESPEAK_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
                text_to_phonemes(text, &self.lang, None, true, false)
                    .unwrap_or_default()
                    .join("")
            }
        }
    }

    pub fn phonemize_postprocess(&self, mut ps: String) -> String {
        // Apply kokoro-specific replacements
        ps = ps
            .replace("kəkˈoːɹoʊ", "kˈoʊkəɹoʊ")
            .replace("kəkˈɔːɹəʊ", "kˈəʊkəɹəʊ");

        // Apply character replacements
        ps = ps
            .replace("ʲ", "j")
            .replace("r", "ɹ")
            .replace("x", "k")
            .replace("ɬ", "l");

        // Apply regex patterns
        ps = PHONEME_PATTERNS.replace_all(&ps, " ").to_string();
        ps = Z_PATTERN.replace_all(&ps, "z").to_string();

        if self.lang == "a" || self.lang == "en-us" {
            ps = NINETY_PATTERN.replace_all(&ps, "di").to_string();
        }

        // Filter characters present in vocabulary
        ps = ps.chars().filter(|&c| VOCAB.contains_key(&c)).collect();

        ps.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phonemizer_en_us() {
        let phonemizer = Phonemizer::new("en-us");
        let ps = phonemizer.phonemize("Hello world", false);
        println!("US Phonemes: {}", ps);
        assert!(!ps.is_empty());
        assert!(ps.contains("həlˈoʊ"));
    }

    #[test]
    fn test_phonemizer_en_gb() {
        let phonemizer = Phonemizer::new("en-gb");
        let ps = phonemizer.phonemize("Hello world", false);
        println!("GB Phonemes: {}", ps);
        assert!(!ps.is_empty());
        assert!(ps.contains("həlˈəʊ"));
    }

    #[test]
    fn test_phonemizer_fallback() {
        let phonemizer = Phonemizer::new("fr");
        let ps = phonemizer.phonemize("Bonjour le monde", false);
        println!("FR Phonemes: {}", ps);
        assert!(!ps.is_empty());
    }

    #[test]
    fn test_phonemizer_overrides() {
        let phonemizer = Phonemizer::new("en-us");
        let ps = phonemizer.phonemize("Hello [world](/wˈɜːld/)!", false);
        println!("Override Phonemes: {}", ps);
        assert!(ps.contains("wˈɜːld"));
        assert!(ps.contains("həlˈo"));
    }

    #[test]
    fn test_phonemizer_dictionary() {
        let mut map = HashMap::new();
        map.insert("world".to_string(), "wˈɜːld".to_string());
        let phoneme_map = Arc::new(PhonemeMap::new(map));
        let phonemizer = Phonemizer::new("en-us").with_phoneme_map(phoneme_map);
        let ps = phonemizer.phonemize("Hello world!", false);
        println!("Dictionary Phonemes: {}", ps);
        assert!(ps.contains("wˈɜːld"));
        assert!(ps.contains("həlˈo"));
    }

    #[test]
    fn test_phonemizer_nested_prevention() {
        let mut map = HashMap::new();
        map.insert("York".to_string(), "wrong".to_string());
        let phoneme_map = Arc::new(PhonemeMap::new(map));
        let phonemizer = Phonemizer::new("en-us").with_phoneme_map(phoneme_map);
        // The inline override should take precedence and "York" inside it should NOT be replaced
        let ps = phonemizer.phonemize("Welcome to [New York](/nˈuːjˈɔːɹk/)!", false);
        println!("Nested Prevention Phonemes: {}", ps);
        assert!(ps.contains("nˈuːjˈɔːɹk"));
        assert!(!ps.contains("wrong"));
    }
}
