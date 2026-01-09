use crate::tts::normalize;
use crate::tts::vocab::VOCAB;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref PHONEME_PATTERNS: Regex = Regex::new(r"(?<=[a-zɹː])(?=hˈʌndɹɪd)").unwrap();
    static ref Z_PATTERN: Regex = Regex::new(r#" z(?=[;:,.!?¡¿—…"«»"" ]|$)"#).unwrap();
    static ref NINETY_PATTERN: Regex = Regex::new(r"(?<=nˈaɪn)ti(?!ː)").unwrap();
}

use std::{error::Error as StdError, fmt};

#[derive(Debug)]
pub enum BackendError {
    UnsupportedLanguage(String),
    NoEspeakForLanguage(String),
    EspeakInitFailed,
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::UnsupportedLanguage(lang) => write!(f, "Unsupported language: {lang}"),
            BackendError::NoEspeakForLanguage(lang) => {
                write!(
                    f,
                    "Espeak backend not used for language: {lang} (Chinese/Japanese)"
                )
            }
            BackendError::EspeakInitFailed => write!(f, "Failed to initialize Espeak backend"),
        }
    }
}

impl StdError for BackendError {}

// Placeholder for the EspeakBackend struct
struct EspeakBackend {
    language: String,
    preserve_punctuation: bool,
    with_stress: bool,
}

impl EspeakBackend {
    fn new(language: &str, preserve_punctuation: bool, with_stress: bool) -> Self {
        EspeakBackend {
            language: language.to_string(),
            preserve_punctuation,
            with_stress,
        }
    }

    fn phonemize(&self, _text: &[String]) -> Option<Vec<String>> {
        // Implementation would go here
        // This is where you'd integrate with actual espeak bindings
        todo!("Implement actual phonemization")
    }
}

pub struct Phonemizer {
    lang: String,
    backend: EspeakBackend,
}

impl Phonemizer {
    pub fn new(lang: &str) -> Result<Self, BackendError> {
        let backend = Self::build_backend(lang)?;

        Ok(Phonemizer {
            lang: lang.to_string(),
            backend,
        })
    }

    fn build_backend(lang: &str) -> Result<EspeakBackend, BackendError> {
        let lang_code =
            Self::lang_code(lang).ok_or(BackendError::UnsupportedLanguage(lang.to_string()))?;
        Ok(EspeakBackend::new(lang_code, true, true))
    }

    fn lang_code(lang: &str) -> Option<&'static str> {
        match lang {
            "a" => Some("en-us"),
            "b" => Some("en-gb"),
            "e" => Some("es"),
            "f" => Some("fr-fr"),
            "h" => Some("hi"),
            "i" => Some("it"),
            "p" => Some("pt-br"),
            _ => None,
        }
    }

    pub fn phonemize(&self, text: &str, normalize: bool) -> String {
        let text = if normalize {
            normalize::normalize_text(text)
        } else {
            text.to_string()
        };

        // Assume phonemize returns Option<String>
        let mut ps = match self.backend.phonemize(&[text]) {
            Some(phonemes) => phonemes[0].clone(),
            None => String::new(),
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

        if self.lang == "a" {
            ps = NINETY_PATTERN.replace_all(&ps, "di").to_string();
        }

        // Filter characters present in vocabulary
        ps = ps.chars().filter(|&c| VOCAB.contains_key(&c)).collect();

        ps.trim().to_string()
    }
}
