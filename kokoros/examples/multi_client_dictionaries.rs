use kokoros::tts::koko::TTSKoko;
use kokoros::tts::phonemizer::PhonemeMap;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup global TTS with a shared dictionary
    let mut global_dict = HashMap::new();
    global_dict.insert("Kokoro".to_string(), "kˈoʊkəɹoʊ".to_string());
    let global_map = Arc::new(PhonemeMap::new(global_dict));

    // Initializing TTSKoko once is expensive, but cloning it later is cheap.
    let tts = TTSKoko::new("checkpoints/kokoro-v1.0.onnx", "data/voices-v1.0.bin")
        .await
        .with_phoneme_map(global_map);

    // 2. Simulate Client A with its own dictionary
    let mut client_a_dict = HashMap::new();
    client_a_dict.insert("Rust".to_string(), "ɹˈʌst".to_string());

    // To efficiently handle per-client dictionaries:
    // a) Merge with global dict if needed
    let mut merged_a = tts.phoneme_map.as_ref().map(|m| m.map.clone()).unwrap_or_default();
    merged_a.extend(client_a_dict);
    let map_a = Arc::new(PhonemeMap::new(merged_a));

    // b) Create a lightweight wrapper for this request
    let tts_a = tts.clone().with_phoneme_map(map_a);

    // 3. Simulate Client B with its own dictionary
    let mut client_b_dict = HashMap::new();
    client_b_dict.insert("Fast".to_string(), "fˈæst".to_string());

    let mut merged_b = tts.phoneme_map.as_ref().map(|m| m.map.clone()).unwrap_or_default();
    merged_b.extend(client_b_dict);
    let map_b = Arc::new(PhonemeMap::new(merged_b));

    let tts_b = tts.clone().with_phoneme_map(map_b);

    // Now tts_a and tts_b share the same model and style data (Arc-wrapped),
    // but have different phoneme maps.

    println!("Client A map entries: {:?}", tts_a.phoneme_map.as_ref().unwrap().map.keys());
    println!("Client B map entries: {:?}", tts_b.phoneme_map.as_ref().unwrap().map.keys());

    // Synthesis calls would go here:
    // tts_a.tts_raw_audio("Kokoro is written in Rust", ...);
    // tts_b.tts_raw_audio("Kokoro is insanely Fast", ...);

    Ok(())
}
