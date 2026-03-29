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
