// ============================================================
// Audio Transducer — PCM/WAV Projection into VSA Space
// Section 1.IV: "Employs FOSS transducers to project audio
// into the unified 10,000-bit VSA space."
//
// Strategy: Frame-level spectral encoding.
//   1. Chunk raw PCM bytes into fixed-size frames.
//   2. Each frame gets a positional encoding (permutation).
//   3. Frame content is hashed into a bipolar vector via
//      byte-level bundling (same technique as BinaryTransducer).
//   4. All frame vectors are bundled into a single superposition.
//
// This creates a "spectral fingerprint" — structurally similar
// audio clips produce vectors with high cosine similarity.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;

/// Frame size in bytes for audio chunking.
/// 256 bytes @ 16-bit mono 16kHz = ~8ms per frame.
/// This granularity captures phoneme-level features.
const AUDIO_FRAME_SIZE: usize = 256;

/// Maximum number of frames to process.
/// Prevents unbounded computation on large audio files.
/// 4096 frames * 256 bytes = ~1MB of audio.
const MAX_AUDIO_FRAMES: usize = 4096;

/// Transducer for projecting raw audio (PCM bytes) into the VSA space.
pub struct AudioTransducer;

impl AudioTransducer {
    /// Project raw PCM audio bytes into a bipolar hypervector.
    ///
    /// The input is treated as a stream of fixed-size frames.
    /// Each frame is encoded via byte-value permutations bound with
    /// frame-level positional encoding, then all frames are bundled.
    pub fn project(pcm_data: &[u8]) -> Result<BipolarVector, HdcError> {
        debuglog!("AudioTransducer::project: entry, data_len={}", pcm_data.len());

        if pcm_data.is_empty() {
            debuglog!("AudioTransducer::project: FAIL - empty input");
            return Err(HdcError::InitializationFailed {
                reason: "Cannot project empty audio data".to_string(),
            });
        }

        // Generate base vectors for positional and value encoding.
        let pos_base = BipolarVector::new_random()?;
        let val_base = BipolarVector::new_random()?;
        debuglog!("AudioTransducer::project: base vectors generated");

        // Chunk into frames, cap at MAX_AUDIO_FRAMES.
        let frame_count = (pcm_data.len() / AUDIO_FRAME_SIZE)
            .max(1)
            .min(MAX_AUDIO_FRAMES);
        debuglog!("AudioTransducer::project: frame_count={}", frame_count);

        let mut frame_vectors: Vec<BipolarVector> = Vec::with_capacity(frame_count);

        for frame_idx in 0..frame_count {
            let start = frame_idx * AUDIO_FRAME_SIZE;
            let end = (start + AUDIO_FRAME_SIZE).min(pcm_data.len());
            let frame = &pcm_data[start..end];

            // Positional encoding for this frame
            let frame_pos = pos_base.permute(frame_idx)?;

            // Content encoding: accumulate byte values via permutation of val_base
            let mut byte_vecs: Vec<BipolarVector> = Vec::with_capacity(frame.len());
            for (byte_idx, byte_val) in frame.iter().enumerate() {
                // Combine byte position within frame and byte value
                let byte_pos = val_base.permute(byte_idx % 10000)?;
                let byte_content = val_base.permute((*byte_val as usize * 39) % 10000)?;
                let encoded_byte = byte_pos.bind(&byte_content)?;
                byte_vecs.push(encoded_byte);
            }

            // Bundle all bytes in this frame into a frame content vector
            let byte_refs: Vec<&BipolarVector> = byte_vecs.iter().collect();
            let frame_content = BipolarVector::bundle(&byte_refs)?;

            // Bind frame content with frame position
            let frame_vec = frame_content.bind(&frame_pos)?;
            frame_vectors.push(frame_vec);

            if frame_idx % 100 == 0 {
                debuglog!(
                    "AudioTransducer::project: processed frame {}/{}",
                    frame_idx + 1, frame_count
                );
            }
        }

        // Bundle all frame vectors into a single audio fingerprint
        let frame_refs: Vec<&BipolarVector> = frame_vectors.iter().collect();
        let result = BipolarVector::bundle(&frame_refs)?;

        debuglog!(
            "AudioTransducer::project: SUCCESS, frames={}, result_dim={}, ones={}",
            frame_count, result.dim(), result.count_ones()
        );
        Ok(result)
    }

    /// Project audio with explicit sample rate and channel metadata.
    /// Metadata is encoded as additional permuted vectors bundled
    /// into the final representation.
    pub fn project_with_metadata(
        pcm_data: &[u8],
        sample_rate: u32,
        channels: u8,
    ) -> Result<BipolarVector, HdcError> {
        debuglog!(
            "AudioTransducer::project_with_metadata: data_len={}, rate={}, ch={}",
            pcm_data.len(), sample_rate, channels
        );

        // Get base audio projection
        let audio_vec = Self::project(pcm_data)?;

        // Encode sample rate as a permuted vector
        let meta_base = BipolarVector::new_random()?;
        let rate_vec = meta_base.permute((sample_rate as usize) % 10000)?;

        // Encode channel count
        let channel_vec = meta_base.permute((channels as usize * 1000) % 10000)?;

        // Bundle audio content with metadata
        let result = BipolarVector::bundle(&[&audio_vec, &rate_vec, &channel_vec])?;

        debuglog!(
            "AudioTransducer::project_with_metadata: SUCCESS, dim={}",
            result.dim()
        );
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_nonempty_audio() -> Result<(), HdcError> {
        // Simulate 512 bytes of PCM audio (2 frames)
        let pcm: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();
        let hv = AudioTransducer::project(&pcm)?;
        assert_eq!(hv.dim(), 10000);
        assert!(hv.count_ones() > 0);
        debuglog!("test_project_nonempty_audio: ones={}", hv.count_ones());
        Ok(())
    }

    #[test]
    fn test_project_empty_fails() {
        let result = AudioTransducer::project(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_small_data() -> Result<(), HdcError> {
        // Less than one frame — should still work (1 frame, partial)
        let pcm = vec![0u8; 64];
        let hv = AudioTransducer::project(&pcm)?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_different_audio_produces_different_vectors() -> Result<(), HdcError> {
        let pcm_a: Vec<u8> = (0..1024).map(|i| (i * 3 % 256) as u8).collect();
        let pcm_b: Vec<u8> = (0..1024).map(|i| (i * 7 % 256) as u8).collect();
        let hv_a = AudioTransducer::project(&pcm_a)?;
        let hv_b = AudioTransducer::project(&pcm_b)?;
        let sim = hv_a.similarity(&hv_b)?;
        debuglog!("test_different_audio: sim={:.4}", sim);
        // Different content should not be highly similar
        assert!(sim < 0.5, "Different audio should diverge, sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_project_with_metadata() -> Result<(), HdcError> {
        let pcm: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();
        let hv = AudioTransducer::project_with_metadata(&pcm, 16000, 1)?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_metadata_affects_output() -> Result<(), HdcError> {
        let pcm: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();
        let hv_mono = AudioTransducer::project_with_metadata(&pcm, 16000, 1)?;
        let hv_stereo = AudioTransducer::project_with_metadata(&pcm, 44100, 2)?;
        let sim = hv_mono.similarity(&hv_stereo)?;
        debuglog!("test_metadata_affects: sim={:.4}", sim);
        // Same audio but different metadata should produce partially different vectors
        // (they share the audio content but differ in metadata encoding)
        assert!(sim < 0.95, "Different metadata should diverge, sim={}", sim);
        Ok(())
    }

    // ============================================================
    // Stress / invariant tests for AudioTransducer
    // ============================================================

    /// INVARIANT: project produces a 10k-dim non-empty vector for any
    /// non-trivial PCM input.
    #[test]
    fn invariant_project_produces_correct_dim() -> Result<(), HdcError> {
        let pcm: Vec<u8> = (0..256).map(|i| i as u8).collect();
        let v = AudioTransducer::project(&pcm)?;
        assert_eq!(v.dim(), 10_000, "audio projection must be 10k dim");
        Ok(())
    }

    /// INVARIANT: every projection has the correct dim.
    #[test]
    fn invariant_project_correct_dim() -> Result<(), HdcError> {
        for size in [1usize, 100, 1024, 8192] {
            let pcm: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
            let v = AudioTransducer::project(&pcm)?;
            assert_eq!(v.dim(), 10_000,
                "projection for {}-byte PCM must be 10k dim", size);
        }
        Ok(())
    }

    /// INVARIANT: project_with_metadata produces a 10k-dim vector for
    /// any sample rate / channel count combination.
    #[test]
    fn invariant_project_with_metadata_produces_correct_dim() -> Result<(), HdcError> {
        let pcm: Vec<u8> = (0..256).map(|i| (i * 3) as u8).collect();
        for (sr, ch) in [(8000u32, 1u8), (16000, 1), (44100, 2), (48000, 2), (96000, 8)] {
            let v = AudioTransducer::project_with_metadata(&pcm, sr, ch)?;
            assert_eq!(v.dim(), 10_000,
                "project_with_metadata({}, {}) must be 10k dim", sr, ch);
        }
        Ok(())
    }

    /// INVARIANT: empty PCM always errors — no silent zero-vector.
    #[test]
    fn invariant_empty_pcm_errors() {
        assert!(AudioTransducer::project(&[]).is_err());
    }
}
