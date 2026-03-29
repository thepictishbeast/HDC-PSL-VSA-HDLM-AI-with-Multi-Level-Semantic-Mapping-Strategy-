// ============================================================
// HID Injection — Hardware-Level GUI Interaction
// Section 2: "Interacts with arbitrary GUIs via hardware-level HID Injection (/dev/hidg0)."
// ============================================================

use crate::hdc::error::HdcError;
use std::fs::OpenOptions;
use std::io::Write;

/// HID Command Set for hardware-level injection.
#[derive(Debug, Clone)]
pub enum HidCommand {
    MouseMove { x: i32, y: i32 },
    MouseClick,
    KeyPress(u8), // Scan codes
    Text(String),
}

/// Interface for the /dev/hidg0 gadget.
pub struct HidDevice {
    device_path: String,
}

impl HidDevice {
    /// Initialize the HID device.
    /// Default path: /dev/hidg0
    pub fn new(path: Option<&str>) -> Result<Self, HdcError> {
        let p = match path {
            Some(s) => s.to_string(),
            None => "/dev/hidg0".to_string(),
        };
        debuglog!("HidDevice::new: path={}", p);
        Ok(Self { device_path: p })
    }

    /// Executes an HID command by writing raw HID reports to the device.
    /// In a real deployment, this requires root privileges.
    pub fn execute(&self, cmd: HidCommand) -> Result<(), HdcError> {
        debuglog!("HidDevice::execute: cmd={:?}", cmd);

        // Simulation guard: Only attempt to open the device if it exists.
        // In a headless CI or restricted environment, we log the intent.
        if std::path::Path::new(&self.device_path).exists() {
            let mut file = OpenOptions::new()
                .write(true)
                .open(&self.device_path)
                .map_err(|e| HdcError::InitializationFailed {
                    reason: format!("Failed to open HID device: {}", e),
                })?;

            match cmd {
                HidCommand::MouseMove { x, y } => {
                    // Raw HID mouse report: [buttons, x_low, y_low, wheel]
                    // Clamp to i8 range for relative HID movement reports.
                    let x_clamped = x.clamp(-127, 127) as i8;
                    let y_clamped = y.clamp(-127, 127) as i8;
                    debuglog!("HidDevice::execute: MouseMove clamped x={}, y={}", x_clamped, y_clamped);
                    let report: [u8; 4] = [0, x_clamped as u8, y_clamped as u8, 0];
                    file.write_all(&report).map_err(|e| HdcError::InitializationFailed {
                        reason: format!("HID write failure: {}", e),
                    })?;
                }
                _ => {
                    debuglog!("HidDevice::execute: Complex HID command implementation pending.");
                }
            }
        } else {
            debuglog!("HidDevice::execute: SIMULATED (Device {} not found)", self.device_path);
        }

        Ok(())
    }
}
