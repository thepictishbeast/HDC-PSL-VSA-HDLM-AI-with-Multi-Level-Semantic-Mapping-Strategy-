// NODE 027: Serial DataStreamer (Hardware Ingress)
// STATUS: ALPHA - Material Ingress Active
// PROTOCOL: Bit-Level-Ingestion / UART-VSA-Pipe

use std::fs::File;
use std::io::Read;
use tracing::{info, debug, error};
use crate::hdc::sensory::{SensoryEncoder, Modality, MultimodalFrame};

pub struct SerialStreamer {
    pub device_path: String,
    pub baud_rate: u32,
}

impl SerialStreamer {
    pub fn new(path: &str, baud: u32) -> Self {
        info!("// AUDIT: Initializing Serial Ingress on {}", path);
        Self {
            device_path: path.to_string(),
            baud_rate: baud,
        }
    }

    /// INGEST: Continuously reads from the material port and yields VSA frames.
    pub async fn stream_to_vsa(&self) -> Result<MultimodalFrame, Box<dyn std::error::Error>> {
        debug!("// DEBUG: Opening material base at {}", self.device_path);
        
        // In a Termux environment, we typically target /dev/ttyUSB0
        let mut file = match File::open(&self.device_path) {
            Ok(f) => f,
            Err(e) => {
                error!("// CRITICAL: Failed to open serial port: {}", e);
                return Err(e.into());
            }
        };

        let mut buffer = [0u8; 64]; // Standard packet size
        match file.read(&mut buffer) {
            Ok(n) if n > 0 => {
                debug!("// AUDIT: Ingested {} bytes from hardware", n);
                let signal_hv = SensoryEncoder::encode_serial(&buffer[..n]);
                Ok(MultimodalFrame {
                    modality: Modality::Serial,
                    timestamp: 0, // Placeholder
                    signal_hv,
                })
            }
            Ok(_) => Err("EOF reached".into()),
            Err(e) => Err(e.into()),
        }
    }
}
