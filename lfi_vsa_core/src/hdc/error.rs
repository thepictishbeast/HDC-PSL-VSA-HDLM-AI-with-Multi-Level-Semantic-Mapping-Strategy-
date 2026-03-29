// ============================================================
// HDC Error Types — Forensic Fault Handling
// ============================================================

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum HdcError {
    DimensionMismatch { expected: usize, actual: usize },
    InitializationFailed { reason: String },
    MemoryFull,
    InvalidBipolarValue,
    PersistenceFailure { detail: String },
    LogicFault { reason: String },
    EmptyBundle,
}

impl fmt::Display for HdcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HdcError::DimensionMismatch { expected, actual } => 
                write!(f, "Dimension Mismatch: expected {}, got {}", expected, actual),
            HdcError::InitializationFailed { reason } => 
                write!(f, "Initialization Failed: {}", reason),
            HdcError::MemoryFull => write!(f, "Holographic Memory Full"),
            HdcError::InvalidBipolarValue => write!(f, "Values must be strictly -1 or 1"),
            HdcError::PersistenceFailure { detail } => 
                write!(f, "Failed to commit state: {}", detail),
            HdcError::LogicFault { reason } => 
                write!(f, "Material Logic Fault: {}", reason),
            HdcError::EmptyBundle => write!(f, "Attempted to bundle zero vectors"),
        }
    }
}

impl std::error::Error for HdcError {}
