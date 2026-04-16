// NODE 026: Multimodal Sensory Cortex (Pixel Optimized)
// STATUS: ALPHA - Physical Grounding Active
// PROTOCOL: JEPA-World-State / Cross-Modal-Binding

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use crate::memory_bus::{HyperMemory, DIM_PROLETARIAT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorGroup {
    Biometric,
    IMU,
    RF,
    Environmental,
    Visual,
    Auditory,
    Serial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modality {
    Audio,  // MERT-style frequency embeddings
    Video,  // JEPA latent world-state
    Serial, // Raw bit-level hex streams
    Logic,  // Symbolic/Textual state
}

/// A legacy sensor frame for backward compatibility.
pub struct SensoryFrame {
    pub group: SensorGroup,
    pub timestamp: u64,
    pub raw_signal: Vec<f64>,
}

/// A high-stakes multimodal event frame.
pub struct MultimodalFrame {
    pub modality: Modality,
    pub timestamp: u64,
    pub signal_hv: HyperMemory,
}

pub struct SensoryCortex {
    pub group_base: Vec<(SensorGroup, BipolarVector)>,
}

impl SensoryCortex {
    pub fn new() -> Result<Self, HdcError> {
        let mut cortex = Self { group_base: Vec::new() };
        cortex.group_base.push((SensorGroup::Biometric, BipolarVector::new_random()?));
        cortex.group_base.push((SensorGroup::IMU, BipolarVector::new_random()?));
        cortex.group_base.push((SensorGroup::RF, BipolarVector::new_random()?));
        cortex.group_base.push((SensorGroup::Environmental, BipolarVector::new_random()?));
        cortex.group_base.push((SensorGroup::Visual, BipolarVector::new_random()?));
        cortex.group_base.push((SensorGroup::Auditory, BipolarVector::new_random()?));
        cortex.group_base.push((SensorGroup::Serial, BipolarVector::new_random()?));
        Ok(cortex)
    }

    pub fn encode_frame(&self, frame: &SensoryFrame) -> Result<BipolarVector, HdcError> {
        let base = self.group_base.iter().find(|(g, _)| g == &frame.group)
            .map(|(_, v)| v).ok_or(HdcError::LogicFault { reason: "Unknown group".into() })?;
        
        let signal_hash = crate::identity::IdentityProver::hash(&format!("{:?}", frame.raw_signal));
        let signal_vec = BipolarVector::from_seed(signal_hash);
        
        base.bind(&signal_vec)
    }
}

impl MultimodalFrame {
    pub fn bind_event(frames: &[Self]) -> Result<HyperMemory, Box<dyn std::error::Error>> {
        if frames.is_empty() { return Err("Empty event frame set".into()); }
        let mut event_hv = frames[0].signal_hv.clone();
        for frame in &frames[1..] {
            event_hv = event_hv.bind(&frame.signal_hv)?;
        }
        Ok(event_hv)
    }
}

pub struct SensoryEncoder;

impl SensoryEncoder {
    pub fn encode_serial(raw_hex: &[u8]) -> HyperMemory {
        let hex_string = raw_hex.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<String>();
        HyperMemory::from_string(&hex_string, DIM_PROLETARIAT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensory_cortex_creation() -> Result<(), HdcError> {
        let cortex = SensoryCortex::new()?;
        assert_eq!(cortex.group_base.len(), 7, "Should have 7 sensor groups");
        Ok(())
    }

    #[test]
    fn test_encode_frame() -> Result<(), HdcError> {
        let cortex = SensoryCortex::new()?;
        let frame = SensoryFrame {
            group: SensorGroup::IMU,
            timestamp: 12345,
            raw_signal: vec![1.0, 2.0, 3.0],
        };
        let encoded = cortex.encode_frame(&frame)?;
        assert_eq!(encoded.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_different_signals_different_encodings() -> Result<(), HdcError> {
        let cortex = SensoryCortex::new()?;
        let f1 = SensoryFrame { group: SensorGroup::Visual, timestamp: 0, raw_signal: vec![1.0] };
        let f2 = SensoryFrame { group: SensorGroup::Visual, timestamp: 0, raw_signal: vec![2.0] };
        let e1 = cortex.encode_frame(&f1)?;
        let e2 = cortex.encode_frame(&f2)?;
        let sim = e1.similarity(&e2)?;
        assert!(sim < 0.9, "Different signals should produce different encodings: {:.4}", sim);
        Ok(())
    }

    #[test]
    fn test_different_groups_different_bases() -> Result<(), HdcError> {
        let cortex = SensoryCortex::new()?;
        let same_signal = vec![1.0, 2.0];
        let f1 = SensoryFrame { group: SensorGroup::Auditory, timestamp: 0, raw_signal: same_signal.clone() };
        let f2 = SensoryFrame { group: SensorGroup::RF, timestamp: 0, raw_signal: same_signal };
        let e1 = cortex.encode_frame(&f1)?;
        let e2 = cortex.encode_frame(&f2)?;
        let sim = e1.similarity(&e2)?;
        // Different groups bind with different base vectors → should differ.
        assert!(sim < 0.9, "Different sensor groups should produce different encodings: {:.4}", sim);
        Ok(())
    }

    #[test]
    fn test_serial_encoder() {
        let data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let encoded = SensoryEncoder::encode_serial(&data);
        assert_eq!(encoded.dimensions, DIM_PROLETARIAT);

        // Same data should produce same encoding.
        let encoded2 = SensoryEncoder::encode_serial(&data);
        let sim = encoded.similarity(&encoded2);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_multimodal_bind_event() -> Result<(), Box<dyn std::error::Error>> {
        let frames = vec![
            MultimodalFrame {
                modality: Modality::Audio,
                timestamp: 100,
                signal_hv: HyperMemory::generate_seed(DIM_PROLETARIAT),
            },
            MultimodalFrame {
                modality: Modality::Video,
                timestamp: 100,
                signal_hv: HyperMemory::generate_seed(DIM_PROLETARIAT),
            },
        ];
        let bound = MultimodalFrame::bind_event(&frames)?;
        assert_eq!(bound.dimensions, DIM_PROLETARIAT);
        Ok(())
    }

    #[test]
    fn test_multimodal_empty_fails() {
        let result = MultimodalFrame::bind_event(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_sensor_group_equality() {
        assert_eq!(SensorGroup::Biometric, SensorGroup::Biometric);
        assert_ne!(SensorGroup::IMU, SensorGroup::RF);
    }

    #[test]
    fn test_modality_equality() {
        assert_eq!(Modality::Audio, Modality::Audio);
        assert_ne!(Modality::Audio, Modality::Video);
    }

    // ============================================================
    // Stress / invariant tests for SensoryCortex / SensoryEncoder
    // ============================================================

    /// INVARIANT: encoding the same frame is deterministic.
    #[test]
    fn invariant_encode_frame_deterministic() -> Result<(), HdcError> {
        let cortex = SensoryCortex::new()?;
        let frame = SensoryFrame {
            group: SensorGroup::IMU,
            timestamp: 1700000000,
            raw_signal: vec![0.1, 0.2, 0.3],
        };
        let v1 = cortex.encode_frame(&frame)?;
        let v2 = cortex.encode_frame(&frame)?;
        assert_eq!(v1, v2,
            "encoding the same frame must produce identical vectors");
        Ok(())
    }

    /// INVARIANT: serial encoder handles empty/extreme inputs without panic.
    #[test]
    fn invariant_serial_encoder_arbitrary_inputs_safe() {
        let inputs: Vec<Vec<u8>> = vec![
            vec![],
            vec![0u8; 1],
            vec![255u8; 256],
            (0..=255).collect::<Vec<u8>>(),
            vec![0xFF; 10_000],
        ];
        for input in &inputs {
            let result = SensoryEncoder::encode_serial(input);
            assert_eq!(result.dimensions, crate::memory_bus::DIM_PROLETARIAT,
                "result dim must be DIM_PROLETARIAT for input len {}", input.len());
        }
    }

    /// INVARIANT: different raw_signal vectors produce different encodings.
    #[test]
    fn invariant_different_signals_different_encodings() -> Result<(), HdcError> {
        let cortex = SensoryCortex::new()?;
        let f1 = SensoryFrame {
            group: SensorGroup::IMU,
            timestamp: 1700000000,
            raw_signal: vec![0.1, 0.1, 0.1],
        };
        let f2 = SensoryFrame {
            group: SensorGroup::IMU,
            timestamp: 1700000000,
            raw_signal: vec![0.9, 0.9, 0.9],
        };
        let v1 = cortex.encode_frame(&f1)?;
        let v2 = cortex.encode_frame(&f2)?;
        assert_ne!(v1, v2,
            "different raw_signal must produce different vectors");
        Ok(())
    }

    /// INVARIANT: SensoryCortex has all known SensorGroups registered.
    #[test]
    fn invariant_cortex_has_all_groups() -> Result<(), HdcError> {
        let cortex = SensoryCortex::new()?;
        let groups = [
            SensorGroup::Biometric, SensorGroup::IMU, SensorGroup::RF,
            SensorGroup::Environmental, SensorGroup::Visual,
            SensorGroup::Auditory, SensorGroup::Serial,
        ];
        for g in groups {
            let found = cortex.group_base.iter().any(|(gg, _)| gg == &g);
            assert!(found, "missing group {:?}", g);
        }
        Ok(())
    }

    /// INVARIANT: encode_frame never panics on arbitrary signal lengths.
    #[test]
    fn invariant_encode_frame_safe_on_any_length() -> Result<(), HdcError> {
        let cortex = SensoryCortex::new()?;
        for len in [0usize, 1, 10, 100, 1000] {
            let frame = SensoryFrame {
                group: SensorGroup::IMU,
                timestamp: 1000,
                raw_signal: vec![0.5; len],
            };
            let _ = cortex.encode_frame(&frame)?;
        }
        Ok(())
    }

    /// INVARIANT: encode_serial returns a valid HyperMemory.
    #[test]
    fn invariant_encode_serial_nonempty() {
        for data in [vec![], vec![0x00], vec![0xFF; 100], (0..255u8).collect::<Vec<_>>()] {
            let hv = SensoryEncoder::encode_serial(&data);
            assert_eq!(hv.dimensions, crate::memory_bus::DIM_PROLETARIAT);
        }
    }
}
