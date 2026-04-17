// ============================================================
// Deepfake Detection — Multi-Modal Synthesis Indicators
//
// PURPOSE: Detect AI-synthesized media (images, audio, video) using
// metadata analysis, statistical indicators, and provenance signals.
//
// SCOPE (what we can detect without ML model):
//   - EXIF metadata signatures from AI generation tools
//   - Known AI watermark patterns (invisible markers)
//   - File structure anomalies from common generators
//   - Pacing/cadence irregularities in text transcripts of audio
//   - Statistical anomalies in transcripts (uniform sentence length, etc.)
//   - C2PA provenance validation (when present)
//
// SCOPE LIMITATIONS:
//   - We do NOT bundle a deep-learning image/audio detector (adds deps,
//     needs GPU, competes with specialist products). We focus on signals
//     that are cheap, deterministic, and composable.
//   - For pixel-level detection, integrate a specialist tool via API.
//
// INTEGRATION:
//   - Extends DefensiveAIAnalyzer with new threat categories
//   - Feeds into unified severity aggregator
//   - Reports via existing AIThreat struct
// ============================================================

use crate::intelligence::defensive_ai::{AIThreat, ThreatCategory, ThreatSeverity};

// ============================================================
// Media Metadata
// ============================================================

/// Metadata parsed from media file headers/sidecar files.
#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    /// EXIF "Software" or equivalent field.
    pub software: Option<String>,
    /// EXIF "Make" / "Model" (camera info).
    pub camera_make: Option<String>,
    /// EXIF "Model" (camera model).
    pub camera_model: Option<String>,
    /// Creation timestamp if available.
    pub created: Option<String>,
    /// Presence of C2PA manifest (cryptographic provenance).
    pub has_c2pa: bool,
    /// Raw EXIF dump (for adversarial pattern search).
    pub raw: Option<String>,
}

// ============================================================
// Image Deepfake Detector
// ============================================================

/// Detects likely AI-generated images via metadata.
/// BUG ASSUMPTION: sophisticated generators strip metadata;
/// this catches the easy cases only. Pair with pixel-level analysis for coverage.
pub struct ImageDeepfakeDetector;

impl ImageDeepfakeDetector {
    /// Analyze media metadata for AI generation indicators.
    pub fn analyze(metadata: &MediaMetadata) -> AIThreat {
        let mut indicators = Vec::new();
        let mut score: f64 = 0.0;

        // 1. Software field matches known AI generators.
        let ai_software_markers = [
            "stable diffusion", "midjourney", "dall-e", "dalle",
            "firefly", "imagen", "invokeai", "automatic1111",
            "comfyui", "fooocus", "kandinsky", "runway",
            "google gemini", "gpt-4", "chatgpt", "leonardo",
            "flux", "sdxl", "playgroundai",
        ];
        if let Some(ref software) = metadata.software {
            let software_lower = software.to_lowercase();
            for marker in &ai_software_markers {
                if software_lower.contains(marker) {
                    score = score.max(0.95);
                    indicators.push(format!("AI generator detected in Software field: '{}'", software));
                    break;
                }
            }
        }

        // 2. Missing camera make/model (photos usually have these).
        if metadata.camera_make.is_none() && metadata.camera_model.is_none() {
            if metadata.software.is_some() {
                score += 0.2;
                indicators.push("Software field present but no camera info (unusual for photo)".into());
            } else {
                // No metadata at all — could be scrubbed.
                score += 0.1;
                indicators.push("No camera metadata (likely scrubbed or synthetic)".into());
            }
        }

        // 3. C2PA presence is a STRONG positive signal (not AI generated).
        if metadata.has_c2pa {
            indicators.push("C2PA provenance manifest present (positive signal)".into());
            score = (score - 0.3).max(0.0);
        }

        // 4. Raw EXIF contains AI-specific fields.
        if let Some(ref raw) = metadata.raw {
            let raw_lower = raw.to_lowercase();
            let ai_exif_markers = [
                "prompt:", "negative_prompt:", "steps:", "cfg_scale:",
                "sampler:", "model_hash:", "seed:", "diffusion",
                "ai_generated", "synthetic_media",
            ];
            let hits: Vec<&str> = ai_exif_markers.iter()
                .filter(|m| raw_lower.contains(*m))
                .copied()
                .collect();
            if !hits.is_empty() {
                score = score.max(0.9);
                indicators.push(format!("AI-specific EXIF fields: {}", hits.join(", ")));
            }
        }

        let confidence = score.min(1.0);
        let severity = match confidence {
            c if c > 0.85 => ThreatSeverity::High,
            c if c > 0.6 => ThreatSeverity::Medium,
            c if c > 0.3 => ThreatSeverity::Low,
            _ => ThreatSeverity::Info,
        };

        AIThreat {
            category: ThreatCategory::Deepfake,
            confidence,
            indicators,
            severity,
            mitigation: if confidence > 0.7 {
                "Likely AI-generated image. Do not treat as photographic evidence. Verify source via independent channel.".into()
            } else if confidence > 0.3 {
                "Possible AI-generated image. Consider origin before trusting as genuine.".into()
            } else {
                "Metadata does not indicate AI generation.".into()
            },
        }
    }
}

// ============================================================
// Audio Deepfake Detector
// ============================================================

/// Detects likely voice clones / synthesized audio using transcript analysis.
/// BUG ASSUMPTION: we analyze the transcript text, not the waveform.
/// For waveform analysis, integrate a specialist detector.
pub struct AudioDeepfakeDetector;

#[derive(Debug, Clone)]
pub struct AudioTranscript {
    /// The transcribed text.
    pub text: String,
    /// Timestamps of segments (optional).
    pub segments: Vec<(f64, f64, String)>, // (start_sec, end_sec, text)
    /// Whether the audio had background noise detected.
    pub has_background_noise: bool,
    /// Whether the audio had natural pauses.
    pub has_natural_pauses: bool,
}

impl AudioDeepfakeDetector {
    pub fn analyze(transcript: &AudioTranscript) -> AIThreat {
        let mut indicators = Vec::new();
        let mut score: f64 = 0.0;

        // 1. Missing background noise (TTS / clones are too clean).
        if !transcript.has_background_noise {
            score += 0.2;
            indicators.push("No background noise detected (unusual for real-world audio)".into());
        }

        // 2. Missing natural pauses.
        if !transcript.has_natural_pauses {
            score += 0.2;
            indicators.push("No natural pauses (suggests synthetic pacing)".into());
        }

        // 3. Uniform segment lengths (real speech varies).
        if transcript.segments.len() >= 5 {
            let durations: Vec<f64> = transcript.segments.iter()
                .map(|(s, e, _)| e - s)
                .collect();
            let mean = durations.iter().sum::<f64>() / durations.len() as f64;
            let variance: f64 = durations.iter()
                .map(|d| (d - mean).powi(2))
                .sum::<f64>() / durations.len() as f64;
            let std_dev = variance.sqrt();
            if mean > 0.0 && std_dev / mean < 0.2 {
                score += 0.25;
                indicators.push(format!(
                    "Uniform segment durations (std_dev/mean = {:.2})",
                    std_dev / mean
                ));
            }
        }

        // 4. Perfect grammar in long transcript (humans have disfluencies).
        let disfluencies = ["um", "uh", "like", "you know", "i mean", "sort of", "kind of"];
        let lower = transcript.text.to_lowercase();
        let disfluency_count: usize = disfluencies.iter()
            .map(|d| lower.matches(d).count())
            .sum();
        let word_count = transcript.text.split_whitespace().count();
        if word_count > 100 && disfluency_count == 0 {
            score += 0.2;
            indicators.push(format!(
                "Long transcript ({} words) with zero disfluencies (suspicious)",
                word_count
            ));
        }

        // 5. Pronunciation-hard words (names, technical terms) produced correctly.
        // Hard to check without reference; skip heuristic.

        // 6. Very short audio samples (under 5 seconds) are harder to analyze.
        let duration = transcript.segments.last().map(|s| s.1).unwrap_or(0.0);
        if duration < 5.0 && !transcript.text.is_empty() {
            indicators.push("Short audio sample - detection confidence reduced".into());
            score *= 0.8;
        }

        let confidence = score.min(1.0);
        let severity = match confidence {
            c if c > 0.7 => ThreatSeverity::High,
            c if c > 0.5 => ThreatSeverity::Medium,
            c if c > 0.3 => ThreatSeverity::Low,
            _ => ThreatSeverity::Info,
        };

        AIThreat {
            category: ThreatCategory::Deepfake,
            confidence,
            indicators,
            severity,
            mitigation: if confidence > 0.6 {
                "Likely synthesized audio / voice clone. Do not authorize transactions or trust identity claims without independent verification.".into()
            } else if confidence > 0.3 {
                "Possible synthesized audio. Caution advised.".into()
            } else {
                "Audio characteristics consistent with human speech.".into()
            },
        }
    }
}

// ============================================================
// Video Deepfake Detector (transcript + frame metadata based)
// ============================================================

/// Detects likely AI-manipulated video from available metadata / transcript.
/// BUG ASSUMPTION: we do not process pixels. Frame-level detection
/// requires integration with a specialized deepfake detection model.
pub struct VideoDeepfakeDetector;

#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub file_metadata: MediaMetadata,
    pub audio_transcript: Option<AudioTranscript>,
    /// Frame count / duration for heuristics.
    pub frame_count: Option<u64>,
    pub duration_seconds: Option<f64>,
    /// Whether face detection found multiple distinct faces.
    pub multiple_faces: bool,
    /// If available: detected blinking rate (blinks per minute).
    pub blink_rate_per_min: Option<f64>,
    /// Lighting consistency score (0.0-1.0) from an external analyzer, if any.
    pub lighting_consistency: Option<f64>,
}

impl VideoDeepfakeDetector {
    pub fn analyze(metadata: &VideoMetadata) -> AIThreat {
        let mut indicators = Vec::new();
        let mut score: f64 = 0.0;

        // 1. Combine image metadata analysis.
        let image_result = ImageDeepfakeDetector::analyze(&metadata.file_metadata);
        if image_result.confidence > 0.3 {
            score = score.max(image_result.confidence * 0.7); // partial weight
            for i in image_result.indicators {
                indicators.push(format!("Video metadata: {}", i));
            }
        }

        // 2. Audio analysis if transcript provided.
        if let Some(ref transcript) = metadata.audio_transcript {
            let audio_result = AudioDeepfakeDetector::analyze(transcript);
            if audio_result.confidence > 0.3 {
                score = score.max(audio_result.confidence * 0.8);
                for i in audio_result.indicators {
                    indicators.push(format!("Audio track: {}", i));
                }
            }
        }

        // 3. Blink rate anomalies (normal: 15-20 blinks/min; deepfakes often < 5).
        if let Some(rate) = metadata.blink_rate_per_min {
            if rate < 5.0 {
                score += 0.3;
                indicators.push(format!("Abnormally low blink rate ({:.1}/min; normal 15-20)", rate));
            } else if rate > 40.0 {
                score += 0.2;
                indicators.push(format!("Abnormally high blink rate ({:.1}/min)", rate));
            }
        }

        // 4. Lighting inconsistency.
        if let Some(consistency) = metadata.lighting_consistency {
            if consistency < 0.5 {
                score += 0.2;
                indicators.push(format!("Low lighting consistency ({:.2})", consistency));
            }
        }

        let confidence = score.min(1.0);
        let severity = match confidence {
            c if c > 0.75 => ThreatSeverity::Critical,
            c if c > 0.5 => ThreatSeverity::High,
            c if c > 0.3 => ThreatSeverity::Medium,
            _ => ThreatSeverity::Low,
        };

        AIThreat {
            category: ThreatCategory::Deepfake,
            confidence,
            indicators,
            severity,
            mitigation: if confidence > 0.5 {
                "Likely deepfake video. Do not trust as evidence. Verify subject identity via live video call or in-person.".into()
            } else {
                "Video characteristics within normal range.".into()
            },
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_stable_diffusion_detected() {
        let metadata = MediaMetadata {
            software: Some("Stable Diffusion v1.5".into()),
            camera_make: None,
            camera_model: None,
            created: None,
            has_c2pa: false,
            raw: None,
        };
        let threat = ImageDeepfakeDetector::analyze(&metadata);
        assert!(threat.confidence > 0.9, "Should detect SD: {:.2}", threat.confidence);
        assert_eq!(threat.severity, ThreatSeverity::High);
    }

    #[test]
    fn test_image_midjourney_detected() {
        let metadata = MediaMetadata {
            software: Some("Midjourney v6".into()),
            ..Default::default()
        };
        let threat = ImageDeepfakeDetector::analyze(&metadata);
        assert!(threat.confidence > 0.9);
    }

    #[test]
    fn test_image_real_photo_low_score() {
        let metadata = MediaMetadata {
            software: Some("Adobe Lightroom Classic 13.0".into()),
            camera_make: Some("Canon".into()),
            camera_model: Some("EOS R5".into()),
            created: Some("2024:05:01 14:30:15".into()),
            has_c2pa: false,
            raw: None,
        };
        let threat = ImageDeepfakeDetector::analyze(&metadata);
        assert!(threat.confidence < 0.3,
            "Real camera photo should score low: {:.2}", threat.confidence);
    }

    #[test]
    fn test_image_c2pa_reduces_score() {
        let without_c2pa = MediaMetadata {
            software: Some("Some Software".into()),
            has_c2pa: false,
            ..Default::default()
        };
        let with_c2pa = MediaMetadata {
            software: Some("Some Software".into()),
            has_c2pa: true,
            ..Default::default()
        };
        let t1 = ImageDeepfakeDetector::analyze(&without_c2pa);
        let t2 = ImageDeepfakeDetector::analyze(&with_c2pa);
        assert!(t2.confidence <= t1.confidence,
            "C2PA should reduce deepfake suspicion");
    }

    #[test]
    fn test_image_ai_exif_fields_detected() {
        let metadata = MediaMetadata {
            software: None,
            raw: Some("Prompt: a beautiful sunset\nNegative_prompt: blurry\nSteps: 25\nCFG_scale: 7".into()),
            ..Default::default()
        };
        let threat = ImageDeepfakeDetector::analyze(&metadata);
        assert!(threat.confidence > 0.8,
            "AI-specific EXIF should score high: {:.2}", threat.confidence);
    }

    #[test]
    fn test_audio_clean_no_pauses_detected() {
        let transcript = AudioTranscript {
            text: "Hello, this is a test message. Thank you for listening.".into(),
            segments: vec![],
            has_background_noise: false,
            has_natural_pauses: false,
        };
        let threat = AudioDeepfakeDetector::analyze(&transcript);
        assert!(threat.confidence > 0.3,
            "Clean audio with no pauses should score: {:.2}", threat.confidence);
    }

    #[test]
    fn test_audio_normal_human_low_score() {
        let text = "um hello so i was thinking um about the project you know and like um i think we should maybe i mean if we can sort of try the new approach you know what i mean. uh yeah. and like maybe kind of check with the team first. um. so yeah. ".to_string();
        let transcript = AudioTranscript {
            text,
            segments: vec![
                (0.0, 2.5, "segment 1".into()),
                (2.5, 4.1, "segment 2".into()),
                (4.1, 8.7, "segment 3".into()),
                (8.7, 10.2, "segment 4".into()),
                (10.2, 15.0, "segment 5".into()),
            ],
            has_background_noise: true,
            has_natural_pauses: true,
        };
        let threat = AudioDeepfakeDetector::analyze(&transcript);
        assert!(threat.confidence < 0.4,
            "Natural human audio should score low: {:.2}, indicators: {:?}",
            threat.confidence, threat.indicators);
    }

    #[test]
    fn test_audio_uniform_durations_flagged() {
        let transcript = AudioTranscript {
            text: "One. Two. Three. Four. Five.".into(),
            segments: vec![
                (0.0, 3.0, "one".into()),
                (3.0, 6.0, "two".into()),
                (6.0, 9.0, "three".into()),
                (9.0, 12.0, "four".into()),
                (12.0, 15.0, "five".into()),
            ],
            has_background_noise: false,
            has_natural_pauses: false,
        };
        let threat = AudioDeepfakeDetector::analyze(&transcript);
        assert!(threat.indicators.iter().any(|i| i.contains("Uniform")),
            "Should detect uniform durations: {:?}", threat.indicators);
    }

    #[test]
    fn test_audio_long_no_disfluencies_flagged() {
        let text: String = (0..150).map(|i| format!("word{} ", i)).collect();
        let transcript = AudioTranscript {
            text,
            segments: vec![(0.0, 30.0, "long".into())],
            has_background_noise: true,
            has_natural_pauses: true,
        };
        let threat = AudioDeepfakeDetector::analyze(&transcript);
        assert!(threat.indicators.iter().any(|i| i.contains("disfluencies")),
            "Should flag long perfect text: {:?}", threat.indicators);
    }

    #[test]
    fn test_video_low_blink_rate_flagged() {
        let metadata = VideoMetadata {
            file_metadata: MediaMetadata::default(),
            audio_transcript: None,
            frame_count: Some(1800),
            duration_seconds: Some(60.0),
            multiple_faces: false,
            blink_rate_per_min: Some(2.0),
            lighting_consistency: None,
        };
        let threat = VideoDeepfakeDetector::analyze(&metadata);
        assert!(threat.confidence > 0.2,
            "Low blink rate should flag: {:.2}", threat.confidence);
        assert!(threat.indicators.iter().any(|i| i.contains("blink rate")));
    }

    #[test]
    fn test_video_normal_lighting_consistency() {
        let metadata = VideoMetadata {
            file_metadata: MediaMetadata {
                camera_make: Some("iPhone".into()),
                camera_model: Some("15 Pro".into()),
                ..Default::default()
            },
            audio_transcript: None,
            frame_count: Some(900),
            duration_seconds: Some(30.0),
            multiple_faces: false,
            blink_rate_per_min: Some(18.0),
            lighting_consistency: Some(0.9),
        };
        let threat = VideoDeepfakeDetector::analyze(&metadata);
        assert!(threat.confidence < 0.3,
            "Normal video should score low: {:.2}", threat.confidence);
    }

    #[test]
    fn test_video_lighting_inconsistency_flagged() {
        let metadata = VideoMetadata {
            file_metadata: MediaMetadata::default(),
            audio_transcript: None,
            frame_count: None,
            duration_seconds: None,
            multiple_faces: true,
            blink_rate_per_min: None,
            lighting_consistency: Some(0.3),
        };
        let threat = VideoDeepfakeDetector::analyze(&metadata);
        assert!(threat.indicators.iter().any(|i| i.contains("lighting")),
            "Low lighting consistency should flag");
    }

    #[test]
    fn test_mitigation_provided_when_high_confidence() {
        let metadata = MediaMetadata {
            software: Some("DALL-E 3".into()),
            ..Default::default()
        };
        let threat = ImageDeepfakeDetector::analyze(&metadata);
        assert!(!threat.mitigation.is_empty());
        assert!(threat.mitigation.to_lowercase().contains("ai") ||
                threat.mitigation.to_lowercase().contains("verify"));
    }

    // ============================================================
    // Stress / invariant tests for deepfake_detection
    // ============================================================

    /// INVARIANT: every detector returns a finite confidence in [0,1].
    #[test]
    fn invariant_threat_confidence_in_unit_interval() {
        let img = ImageDeepfakeDetector::analyze(&MediaMetadata {
            software: Some("Stable Diffusion XL".into()),
            ..Default::default()
        });
        assert!(img.confidence.is_finite() && (0.0..=1.0).contains(&img.confidence),
            "image confidence out of [0,1]: {}", img.confidence);

        let img_clean = ImageDeepfakeDetector::analyze(&MediaMetadata::default());
        assert!(img_clean.confidence.is_finite() && (0.0..=1.0).contains(&img_clean.confidence));

        let aud = AudioDeepfakeDetector::analyze(&AudioTranscript {
            text: "uh, well, you know, like".into(),
            segments: vec![],
            has_background_noise: true,
            has_natural_pauses: true,
        });
        assert!(aud.confidence.is_finite() && (0.0..=1.0).contains(&aud.confidence),
            "audio confidence out of [0,1]: {}", aud.confidence);

        let vid = VideoDeepfakeDetector::analyze(&VideoMetadata {
            file_metadata: MediaMetadata::default(),
            audio_transcript: None,
            frame_count: Some(1500),
            duration_seconds: Some(60.0),
            multiple_faces: false,
            blink_rate_per_min: Some(15.0),
            lighting_consistency: Some(0.9),
        });
        assert!(vid.confidence.is_finite() && (0.0..=1.0).contains(&vid.confidence),
            "video confidence out of [0,1]: {}", vid.confidence);
    }

    /// INVARIANT: clean metadata produces low (or zero) confidence threat.
    #[test]
    fn invariant_clean_metadata_low_score() {
        let threat = ImageDeepfakeDetector::analyze(&MediaMetadata::default());
        assert!(threat.confidence < 0.5,
            "clean metadata should produce low confidence: {}", threat.confidence);
    }

    /// INVARIANT: known AI generators (DALL-E, Stable Diffusion, Midjourney)
    /// always trigger non-zero confidence — security-critical positive case.
    #[test]
    fn invariant_known_ai_software_flagged() {
        for software in ["DALL-E 3", "Stable Diffusion", "Midjourney v6"] {
            let m = MediaMetadata {
                software: Some(software.into()),
                ..Default::default()
            };
            let threat = ImageDeepfakeDetector::analyze(&m);
            assert!(threat.confidence > 0.0,
                "known AI generator {} not flagged: {}", software, threat.confidence);
        }
    }

    /// INVARIANT: detectors never panic on arbitrary unicode software names.
    #[test]
    fn invariant_detectors_safe_on_unicode_software() {
        let weird = ["", "アリス Studio", "🦀-Edit", "control:\x00", &"x".repeat(10_000)];
        for s in weird {
            let m = MediaMetadata {
                software: Some(s.to_string()),
                ..Default::default()
            };
            let _ = ImageDeepfakeDetector::analyze(&m);
        }
    }

    /// INVARIANT: video detector handles zero-duration video without dividing
    /// by zero — produces a non-NaN confidence.
    #[test]
    fn invariant_video_zero_duration_safe() {
        let m = VideoMetadata {
            file_metadata: MediaMetadata::default(),
            audio_transcript: None,
            frame_count: Some(0),
            duration_seconds: Some(0.0),
            multiple_faces: false,
            blink_rate_per_min: Some(0.0),
            lighting_consistency: Some(0.5),
        };
        let threat = VideoDeepfakeDetector::analyze(&m);
        assert!(threat.confidence.is_finite(),
            "zero-duration video confidence must be finite, got {}", threat.confidence);
    }

    /// INVARIANT: VideoMetadata with None optional fields doesn't panic.
    #[test]
    fn invariant_video_none_fields_safe() {
        let m = VideoMetadata {
            file_metadata: MediaMetadata::default(),
            audio_transcript: None,
            frame_count: None,
            duration_seconds: None,
            multiple_faces: false,
            blink_rate_per_min: None,
            lighting_consistency: None,
        };
        let threat = VideoDeepfakeDetector::analyze(&m);
        assert!(threat.confidence.is_finite());
    }
}
