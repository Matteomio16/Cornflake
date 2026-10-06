use std::time::Instant;

#[cfg(windows)]
#[link(name = "advapi32")]
extern "C" {}

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

fn read_wav(name: &str) -> Vec<f32> {
    let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    let bytes = std::fs::read(path).unwrap();
    let at = bytes.windows(4).position(|w| w == b"data").unwrap() + 8;
    bytes[at..]
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
        .collect()
}

fn transcribe(ctx: &WhisperContext, audio: &[f32]) -> String {
    let mut state = ctx.create_state().unwrap();
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_print_progress(false);
    params.set_print_realtime(false);
    state.full(params, audio).unwrap();
    (0..state.full_n_segments().unwrap())
        .map(|i| state.full_get_segment_text(i).unwrap())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn transcribes_fixture_speech() {
    let Ok(model) = std::env::var("OC_TEST_MODEL") else {
        eprintln!("OC_TEST_MODEL not set, skipping");
        return;
    };
    let ctx = WhisperContext::new_with_params(&model, WhisperContextParameters::default()).unwrap();
    for (file, expected) in [("me.wav", ["thursday", "deck"]), ("them.wav", ["wednesday", "valuation"])] {
        let audio = read_wav(file);
        let secs = audio.len() as f32 / 16000.0;
        let start = Instant::now();
        let text = transcribe(&ctx, &audio).to_lowercase();
        let took = start.elapsed().as_secs_f32();
        eprintln!("{file}: {secs:.1}s audio in {took:.1}s (x{:.1} realtime): {text}", secs / took);
        for word in expected {
            assert!(text.contains(word), "{file}: missing '{word}' in '{text}'");
        }
    }
}

#[test]
fn transcribe_given_file() {
    let (Ok(model), Ok(wav)) = (std::env::var("OC_TEST_MODEL"), std::env::var("OC_TEST_WAV")) else {
        return;
    };
    let ctx = WhisperContext::new_with_params(&model, WhisperContextParameters::default()).unwrap();
    let bytes = std::fs::read(wav).unwrap();
    let at = bytes.windows(4).position(|w| w == b"data").unwrap() + 8;
    let audio: Vec<f32> = bytes[at..].chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0).collect();
    eprintln!("TEXT: {}", transcribe(&ctx, &audio));
}
