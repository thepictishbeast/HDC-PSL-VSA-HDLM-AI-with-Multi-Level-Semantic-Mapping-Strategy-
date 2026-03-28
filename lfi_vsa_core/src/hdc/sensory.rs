// ============================================================
// VSA Sensory Mapper — The Material Sensory Cortex
// Section 1.II: "Total sensory integration... bypass HAL and
// implement Direct Memory Access (DMA) logic."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use crate::debuglog;

/// Core sensor groups mapped to the VSA space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorGroup {
    Biometric,
    IMU,
    RF,
    Environmental,
    Visual,
    Auditory,
}

/// A multimodal sensor frame.
pub struct SensoryFrame {
    pub group: SensorGroup,
    pub timestamp: u64,
    pub raw_signal: Vec<f64>,
}

/// The Sensory Logic Mapper.
pub struct SensoryCortex {
    /// Item memory for sensor group anchors.
    pub group_base: Vec<(SensorGroup, BipolarVector)>,
}

impl SensoryCortex {
    pub fn new() -> Result<Self, HdcError> {
        debuglog!("SensoryCortex::new: Initializing Sensory Cortex");
        let mut cortex = Self { group_base: Vec::new() };
        
        // Register orthogonal anchors for each group
        cortex.group_base.push((SensorGroup::Biometric, BipolarVector::new_random()?));
        cortex.group_base.push((SensorGroup::IMU, BipolarVector::new_random()?));
        cortex.group_base.push((SensorGroup::RF, BipolarVector::new_random()?));
        cortex.group_base.push((SensorGroup::Environmental, BipolarVector::new_random()?));
        cortex.group_base.push((SensorGroup::Visual, BipolarVector::new_random()?));
        cortex.group_base.push((SensorGroup::Auditory, BipolarVector::new_random()?));
        
        Ok(cortex)
    }

    /// Maps a raw sensory frame into a temporal high-dimensional context.
    /// Result = Group_Anchor XOR Temporal_Anchor XOR Signal_Vector
    pub fn encode_frame(&self, frame: &SensoryFrame) -> Result<BipolarVector, HdcError> {
        // 1. Get Group Anchor
        let group_anchor = self.group_base.iter()
            .find(|(g, _)| *g == frame.group)
            .map(|(_, v)| v)
            .ok_or(HdcError::InitializationFailed { reason: "Unknown sensor group".into() })?;

        // 2. Generate Temporal Hypervector (using timestamp as seed)
        let temporal_hv = BipolarVector::from_seed(frame.timestamp);

        // 3. Vectorize Raw Signal (Simulated positional encoding)
        let mut signal_bits = bitvec::vec::BitVec::<u8, bitvec::order::Lsb0>::repeat(false, 10000);
        for (i, &val) in frame.raw_signal.iter().enumerate() {
            let pos = (i * 1337) % 10000;
            if val > 0.0 {
                signal_bits.set(pos, true);
            }
        }
        let signal_vector = BipolarVector::from_bitvec(signal_bits)?;

        // 4. Structural Binding: Binding perception to context
        let encoded = group_anchor.bind(&temporal_hv)?.bind(&signal_vector)?;
        
        debuglog!("SensoryCortex: Frame from {:?} encoded into VSA space", frame.group);
        Ok(encoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensory_encoding() -> Result<(), HdcError> {
        let cortex = SensoryCortex::new()?;
        let frame = SensoryFrame {
            group: SensorGroup::IMU,
            timestamp: 123456789,
            raw_signal: vec![0.5, -0.2, 0.8],
        };
        
        let hv = cortex.encode_frame(&frame)?;
        assert_eq!(hv.dim(), 10000);
        
        // Verification: same frame should yield same vector
        let hv2 = cortex.encode_frame(&frame)?;
        assert_eq!(hv, hv2);
        
        Ok(())
    }
}
