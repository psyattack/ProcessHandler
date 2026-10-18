use base64::prelude::*;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use crate::models::Rule;

pub const SHARE_PREFIX: &str = "PH1:";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum SharedAutomation {
    Single(Rule),
    Batch(Vec<Rule>),
}

/// Compress and base64-encode single or batch automations with "PH1:" prefix
pub fn compress_and_encode(payload: &SharedAutomation) -> Result<String, String> {
    let json_bytes = serde_json::to_vec(payload).map_err(|e| format!("JSON serialize error: {e}"))?;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&json_bytes).map_err(|e| format!("Compression error: {e}"))?;
    let compressed = encoder.finish().map_err(|e| format!("Compression finish error: {e}"))?;
    let b64 = BASE64_STANDARD.encode(compressed);
    Ok(format!("{}{}", SHARE_PREFIX, b64))
}

/// Decode and decompress automation payload, returning a list of rules
pub fn decode_and_decompress(raw_input: &str) -> Result<Vec<Rule>, String> {
    let clean = raw_input.trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim();

    if clean.is_empty() {
        return Err("Input is empty".into());
    }

    let encoded = if let Some(stripped) = clean.strip_prefix(SHARE_PREFIX) {
        stripped
    } else if let Some(stripped) = clean.strip_prefix("PH_AUTO:") {
        stripped
    } else {
        clean
    };

    // 1. Try Base64 decode + Zlib decompress
    if let Ok(compressed_bytes) = BASE64_STANDARD.decode(encoded.trim()) {
        let mut decoder = ZlibDecoder::new(&compressed_bytes[..]);
        let mut json_str = String::new();
        if decoder.read_to_string(&mut json_str).is_ok() {
            if let Ok(shared) = serde_json::from_str::<SharedAutomation>(&json_str) {
                return Ok(match shared {
                    SharedAutomation::Single(r) => vec![r],
                    SharedAutomation::Batch(v) => v,
                });
            }
            if let Ok(single) = serde_json::from_str::<Rule>(&json_str) {
                return Ok(vec![single]);
            }
            if let Ok(batch) = serde_json::from_str::<Vec<Rule>>(&json_str) {
                return Ok(batch);
            }
        }
    }

    // 2. Fallback: Direct JSON parsing
    if let Ok(shared) = serde_json::from_str::<SharedAutomation>(clean) {
        return Ok(match shared {
            SharedAutomation::Single(r) => vec![r],
            SharedAutomation::Batch(v) => v,
        });
    }
    if let Ok(single) = serde_json::from_str::<Rule>(clean) {
        return Ok(vec![single]);
    }
    if let Ok(batch) = serde_json::from_str::<Vec<Rule>>(clean) {
        return Ok(batch);
    }

    Err("Invalid automation data: could not decode or decompress payload".into())
}

/// Copy text to system clipboard via both egui context and arboard for maximum reliability
pub fn copy_text_to_clipboard(ctx: &egui::Context, text: &str) {
    ctx.copy_text(text.to_string());
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text);
    }
}

/// Read text from system clipboard
pub fn read_text_from_clipboard() -> Result<String, String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {e}"))?;
    cb.get_text().map_err(|e| format!("Clipboard empty or non-text: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Action, ProcessMatchMode, Trigger};

    #[test]
    fn test_single_rule_roundtrip() {
        let rule = Rule {
            id: "test_rule_1".into(),
            name: "Test Launch Rule".into(),
            description: "Launches game and kills bloat".into(),
            is_enabled: true,
            cooldown_seconds: 5.0,
            trigger: Some(Trigger::Process {
                process_name: "game.exe".into(),
                match_mode: ProcessMatchMode::Exact,
                single_instance: true,
            }),
            actions: vec![
                Action::StartProcess {
                    file_path: "C:\\tools\\companion.exe".into(),
                    arguments: "-silent".into(),
                    working_directory: None,
                    run_as_admin: false,
                    only_if_not_running: true,
                    run_hidden: true,
                    close_on_exit: true,
                },
            ],
            on_exit_actions: vec![
                Action::Notification {
                    title: "Game Exited".into(),
                    message: "Cleaning up".into(),
                },
            ],
            execution_count: 0,
            custom_icon_path: Some("C:\\games\\game.exe".into()),
            auto_process_icon: true,
        };

        let encoded = compress_and_encode(&SharedAutomation::Single(rule.clone())).expect("encode failed");
        assert!(encoded.starts_with(SHARE_PREFIX));
        println!("Encoded length: {}", encoded.len());

        let decoded = decode_and_decompress(&encoded).expect("decode failed");
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].id, rule.id);
        assert_eq!(decoded[0].name, rule.name);
        assert_eq!(decoded[0].actions.len(), 1);
        assert_eq!(decoded[0].custom_icon_path, Some("C:\\games\\game.exe".into()));
        assert!(decoded[0].auto_process_icon);
    }

    #[test]
    fn test_batch_rules_roundtrip() {
        let mut rules = Vec::new();
        for i in 0..5 {
            rules.push(Rule {
                id: format!("rule_{i}"),
                name: format!("Automation Rule {i}"),
                description: format!("Description for {i}"),
                is_enabled: true,
                cooldown_seconds: 2.0,
                trigger: None,
                actions: vec![Action::Notification {
                    title: format!("Title {i}"),
                    message: format!("Message {i}"),
                }],
                on_exit_actions: Vec::new(),
                execution_count: 0,
                custom_icon_path: None,
                auto_process_icon: false,
            });
        }

        let encoded = compress_and_encode(&SharedAutomation::Batch(rules.clone())).expect("batch encode");
        let decoded = decode_and_decompress(&encoded).expect("batch decode");
        assert_eq!(decoded.len(), 5);
        assert_eq!(decoded[0].name, "Automation Rule 0");
        assert_eq!(decoded[4].name, "Automation Rule 4");
    }
}
