use crate::tts::normalize;
use crate::tts::vocab::VOCAB;
use espeak_rs::text_to_phonemes;
use fancy_regex::Regex;
use lazy_static::lazy_static;
use misaki_rs::G2P;
use std::sync::Mutex;

lazy_static! {
    static ref PHONEME_PATTERNS: Regex = Regex::new(r"(?<=[a-zɹː])(?=hˈʌndɹɪd)").unwrap();
    static ref Z_PATTERN: Regex = Regex::new(r#" z(?=[;:,.!?¡¿—…"«»"" ]|$)"#).unwrap();
    static ref NINETY_PATTERN: Regex = Regex::new(r"(?<=nˈaɪn)ti(?!ː)").unwrap();
    pub static ref ESPEAK_MUTEX: Mutex<()> = Mutex::new(());
    static ref MISAKI_EN_US: G2P = G2P::new(false);
    static ref MISAKI_EN_GB: G2P = G2P::new(true);
}

pub struct Phonemizer {
    lang: String,
}

impl Phonemizer {
    pub fn new(lang: &str) -> Self {
        Phonemizer {
            lang: lang.to_string(),
        }
    }

    pub fn phonemize(&self, text: &str, normalize: bool) -> String {
        let text = if normalize {
            normalize::normalize_text(text)
        } else {
            text.to_string()
        };

        let mut ps = match self.lang.as_str() {
            "a" | "en-us" => MISAKI_EN_US.g2p(&text).0,
            "b" | "en-gb" => MISAKI_EN_GB.g2p(&text).0,
            _ => {
                let _guard = ESPEAK_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
                text_to_phonemes(&text, &self.lang, None, true, false)
                    .unwrap_or_default()
                    .join("")
            }
        };

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
}
