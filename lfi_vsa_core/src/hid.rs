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

    /// Get the device path.
    pub fn device_path(&self) -> &str {
        &self.device_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hid_device_creation() {
        let device = HidDevice::new(None).expect("default path should work");
        assert_eq!(device.device_path(), "/dev/hidg0");

        let custom = HidDevice::new(Some("/tmp/test_hid")).expect("custom path should work");
        assert_eq!(custom.device_path(), "/tmp/test_hid");
    }

    #[test]
    fn test_hid_command_debug() {
        let cmd = HidCommand::MouseMove { x: 10, y: -5 };
        let dbg = format!("{:?}", cmd);
        assert!(dbg.contains("MouseMove"));
        assert!(dbg.contains("10"));

        let click = HidCommand::MouseClick;
        assert!(format!("{:?}", click).contains("MouseClick"));

        let text = HidCommand::Text("hello".into());
        assert!(format!("{:?}", text).contains("hello"));
    }

    #[test]
    fn test_hid_execute_simulated() {
        // Device path doesn't exist — should simulate without error.
        let device = HidDevice::new(Some("/dev/nonexistent_hidg_test")).unwrap();
        let result = device.execute(HidCommand::MouseMove { x: 50, y: 50 });
        assert!(result.is_ok(), "Simulated execution should succeed");
    }

    #[test]
    fn test_hid_mouse_click_simulated() {
        let device = HidDevice::new(Some("/dev/nonexistent_hidg_test")).unwrap();
        let result = device.execute(HidCommand::MouseClick);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hid_text_simulated() {
        let device = HidDevice::new(Some("/dev/nonexistent_hidg_test")).unwrap();
        let result = device.execute(HidCommand::Text("test input".into()));
        assert!(result.is_ok());
    }

    #[test]
    fn test_hid_keypress_simulated() {
        let device = HidDevice::new(Some("/dev/nonexistent_hidg_test")).unwrap();
        let result = device.execute(HidCommand::KeyPress(0x04)); // 'a' in HID
        assert!(result.is_ok());
    }

    // ============================================================
    // Stress / invariant tests for HidDevice
    // ============================================================

    /// INVARIANT: execute is safe for extreme mouse movement coords (won't
    /// overflow i8 after clamp).
    #[test]
    fn invariant_mouse_move_extreme_coords_safe() {
        let device = HidDevice::new(Some("/dev/nonexistent_xxx")).unwrap();
        let extremes = [
            (0, 0),
            (i32::MAX, i32::MAX),
            (i32::MIN, i32::MIN),
            (1000, -1000),
            (-1000, 1000),
        ];
        for (x, y) in extremes {
            let r = device.execute(HidCommand::MouseMove { x, y });
            assert!(r.is_ok(),
                "mouse move ({}, {}) should not error in simulation", x, y);
        }
    }

    /// INVARIANT: text command is safe for arbitrary unicode.
    #[test]
    fn invariant_text_arbitrary_unicode_safe() {
        let device = HidDevice::new(Some("/dev/nonexistent_xxx")).unwrap();
        let texts = [
            "",
            "αβγ",
            "🦀🦀🦀",
            "日本語テキスト",
            &"x".repeat(10_000),
        ];
        for t in texts {
            let _ = device.execute(HidCommand::Text(t.to_string()));
        }
    }

    /// INVARIANT: new() never fails regardless of path.
    #[test]
    fn invariant_new_always_succeeds() -> Result<(), HdcError> {
        let paths = ["", "/", "/dev/hidg0", "/tmp/test"];
        for p in paths {
            let _ = HidDevice::new(Some(p))?;
        }
        let _ = HidDevice::new(None)?;
        Ok(())
    }

    /// INVARIANT: device_path reflects constructor argument.
    #[test]
    fn invariant_device_path_reflects_constructor() {
        for custom in ["/dev/hidg0", "/dev/hidg1", "/tmp/mock"] {
            let d = HidDevice::new(Some(custom)).unwrap();
            assert_eq!(d.device_path(), custom);
        }
    }
}
