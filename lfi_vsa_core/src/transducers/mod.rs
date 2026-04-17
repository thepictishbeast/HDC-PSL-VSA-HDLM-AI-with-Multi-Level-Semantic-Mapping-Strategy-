// ============================================================
// LFI Transducers — Multimodal Projection
// Section 1.IV: "Employs FOSS transducers to project audio, video,
// images, and arbitrary file binaries into the unified 10,000-bit VSA space."
//
// Each transducer implements a domain-specific encoding strategy
// that maps raw sensory data into the unified 10,000-bit bipolar
// VSA space. The resulting vectors are compatible with all HDC
// algebra operations (Bind, Bundle, Permute, Similarity).
// ============================================================

pub mod binary;
pub mod audio;
pub mod image;
pub mod text;

pub use binary::BinaryTransducer;
pub use audio::AudioTransducer;
pub use image::ImageTransducer;
pub use text::TextTransducer;
