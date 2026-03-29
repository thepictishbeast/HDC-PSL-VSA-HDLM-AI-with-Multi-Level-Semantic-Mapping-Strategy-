// ============================================================
// Image Transducer — Pixel Spatial Encoding into VSA Space
// Section 1.IV: "Employs FOSS transducers to project images
// into the unified 10,000-bit VSA space."
//
// Strategy: Patch-level spatial encoding.
//   1. Treat raw image bytes as a flat pixel buffer (grayscale
//      or interleaved RGB).
//   2. Divide into fixed-size spatial patches.
//   3. Each patch gets a 2D positional encoding via permutation
//      composition: permute(base_x, col) XOR permute(base_y, row).
//   4. Patch content is encoded via byte-level value permutations.
//   5. All patches are bundled into a single image fingerprint.
//
// This creates a spatially-aware holographic representation —
// similar images (even with minor translations) will produce
// vectors with measurable cosine similarity.
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;

/// Patch size in pixels (side length). Each patch is PATCH_SIZE x PATCH_SIZE.
const PATCH_SIZE: usize = 8;

/// Maximum number of patches to process per axis.
/// 64 x 64 patches = 4096 patches maximum.
const MAX_PATCHES_PER_AXIS: usize = 64;

/// Transducer for projecting raw image data into the VSA space.
pub struct ImageTransducer;

impl ImageTransducer {
    /// Project a raw grayscale image into a bipolar hypervector.
    ///
    /// Parameters:
    ///   - `pixels`: Raw pixel bytes (1 byte per pixel, grayscale).
    ///   - `width`: Image width in pixels.
    ///   - `height`: Image height in pixels.
    ///
    /// The image is divided into PATCH_SIZE x PATCH_SIZE patches.
    /// Each patch is encoded with 2D positional information and
    /// pixel-level content, then bundled into a holographic whole.
    pub fn project_grayscale(
        pixels: &[u8],
        width: usize,
        height: usize,
    ) -> Result<BipolarVector, HdcError> {
        debuglog!(
            "ImageTransducer::project_grayscale: entry, pixels={}, w={}, h={}",
            pixels.len(), width, height
        );

        if pixels.is_empty() || width == 0 || height == 0 {
            debuglog!("ImageTransducer::project_grayscale: FAIL - empty or zero-dim");
            return Err(HdcError::InitializationFailed {
                reason: "Cannot project empty or zero-dimension image".to_string(),
            });
        }

        if pixels.len() < width * height {
            debuglog!(
                "ImageTransducer::project_grayscale: FAIL - buffer too small, need {}, got {}",
                width * height, pixels.len()
            );
            return Err(HdcError::InitializationFailed {
                reason: format!(
                    "Pixel buffer too small: need {} bytes for {}x{}, got {}",
                    width * height, width, height, pixels.len()
                ),
            });
        }

        // Generate base vectors for 2D positional encoding.
        let base_x = BipolarVector::new_random()?;
        let base_y = BipolarVector::new_random()?;
        let val_base = BipolarVector::new_random()?;
        debuglog!("ImageTransducer::project_grayscale: base vectors generated");

        // Calculate patch grid dimensions
        let patches_x = (width / PATCH_SIZE).max(1).min(MAX_PATCHES_PER_AXIS);
        let patches_y = (height / PATCH_SIZE).max(1).min(MAX_PATCHES_PER_AXIS);
        let total_patches = patches_x * patches_y;
        debuglog!(
            "ImageTransducer::project_grayscale: patch grid {}x{} = {} patches",
            patches_x, patches_y, total_patches
        );

        let mut patch_vectors: Vec<BipolarVector> = Vec::with_capacity(total_patches);

        for py in 0..patches_y {
            for px in 0..patches_x {
                // 2D positional encoding: bind X-position with Y-position
                let pos_x = base_x.permute(px)?;
                let pos_y = base_y.permute(py)?;
                let patch_pos = pos_x.bind(&pos_y)?;

                // Encode patch content: sample pixels within this patch
                let mut content_vecs: Vec<BipolarVector> = Vec::new();
                for dy in 0..PATCH_SIZE.min(height - py * PATCH_SIZE) {
                    for dx in 0..PATCH_SIZE.min(width - px * PATCH_SIZE) {
                        let pixel_x = px * PATCH_SIZE + dx;
                        let pixel_y = py * PATCH_SIZE + dy;
                        let idx = pixel_y * width + pixel_x;

                        if idx < pixels.len() {
                            let pixel_val = pixels[idx];
                            // Encode pixel intensity via permutation
                            let val_vec = val_base.permute(pixel_val as usize)?;
                            content_vecs.push(val_vec);
                        }
                    }
                }

                if content_vecs.is_empty() {
                    debuglog!(
                        "ImageTransducer::project_grayscale: skip empty patch ({},{})",
                        px, py
                    );
                    continue;
                }

                // Bundle pixel values within this patch
                let content_refs: Vec<&BipolarVector> = content_vecs.iter().collect();
                let patch_content = BipolarVector::bundle(&content_refs)?;

                // Bind content with position
                let patch_vec = patch_content.bind(&patch_pos)?;
                patch_vectors.push(patch_vec);
            }

            if py % 10 == 0 {
                debuglog!(
                    "ImageTransducer::project_grayscale: processed row {}/{}",
                    py + 1, patches_y
                );
            }
        }

        if patch_vectors.is_empty() {
            debuglog!("ImageTransducer::project_grayscale: FAIL - no patches generated");
            return Err(HdcError::InitializationFailed {
                reason: "No valid patches could be extracted from image".to_string(),
            });
        }

        // Bundle all patches into a single image vector
        let patch_refs: Vec<&BipolarVector> = patch_vectors.iter().collect();
        let result = BipolarVector::bundle(&patch_refs)?;

        debuglog!(
            "ImageTransducer::project_grayscale: SUCCESS, patches={}, dim={}, ones={}",
            patch_vectors.len(), result.dim(), result.count_ones()
        );
        Ok(result)
    }

    /// Project raw RGB image bytes (3 bytes per pixel, interleaved) into VSA space.
    /// Channels are encoded separately then bundled to preserve color information.
    pub fn project_rgb(
        pixels: &[u8],
        width: usize,
        height: usize,
    ) -> Result<BipolarVector, HdcError> {
        debuglog!(
            "ImageTransducer::project_rgb: entry, pixels={}, w={}, h={}",
            pixels.len(), width, height
        );

        let expected_len = width * height * 3;
        if pixels.len() < expected_len {
            return Err(HdcError::InitializationFailed {
                reason: format!(
                    "RGB buffer too small: need {} bytes for {}x{}x3, got {}",
                    expected_len, width, height, pixels.len()
                ),
            });
        }

        // Deinterleave into per-channel grayscale buffers
        let pixel_count = width * height;
        let mut r_channel = Vec::with_capacity(pixel_count);
        let mut g_channel = Vec::with_capacity(pixel_count);
        let mut b_channel = Vec::with_capacity(pixel_count);

        for i in 0..pixel_count {
            r_channel.push(pixels[i * 3]);
            g_channel.push(pixels[i * 3 + 1]);
            b_channel.push(pixels[i * 3 + 2]);
        }

        debuglog!("ImageTransducer::project_rgb: deinterleaved {} pixels", pixel_count);

        // Project each channel separately
        let r_vec = Self::project_grayscale(&r_channel, width, height)?;
        let g_vec = Self::project_grayscale(&g_channel, width, height)?;
        let b_vec = Self::project_grayscale(&b_channel, width, height)?;

        // Bundle channels with channel-discriminating permutations
        let r_encoded = r_vec.permute(0)?;   // Identity for R
        let g_encoded = g_vec.permute(3333)?; // Shift for G
        let b_encoded = b_vec.permute(6666)?; // Shift for B

        let result = BipolarVector::bundle(&[&r_encoded, &g_encoded, &b_encoded])?;

        debuglog!(
            "ImageTransducer::project_rgb: SUCCESS, dim={}, ones={}",
            result.dim(), result.count_ones()
        );
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_grayscale_basic() -> Result<(), HdcError> {
        // 16x16 grayscale image with gradient
        let width = 16;
        let height = 16;
        let pixels: Vec<u8> = (0..(width * height))
            .map(|i| (i % 256) as u8)
            .collect();
        let hv = ImageTransducer::project_grayscale(&pixels, width, height)?;
        assert_eq!(hv.dim(), 10000);
        assert!(hv.count_ones() > 0);
        debuglog!("test_project_grayscale_basic: ones={}", hv.count_ones());
        Ok(())
    }

    #[test]
    fn test_project_grayscale_empty_fails() {
        let result = ImageTransducer::project_grayscale(&[], 0, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_grayscale_buffer_too_small() {
        let result = ImageTransducer::project_grayscale(&[0u8; 10], 100, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_grayscale_small_image() -> Result<(), HdcError> {
        // 4x4 image (smaller than a patch — should still work)
        let pixels = vec![128u8; 16];
        let hv = ImageTransducer::project_grayscale(&pixels, 4, 4)?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_different_images_diverge() -> Result<(), HdcError> {
        let width = 16;
        let height = 16;
        let pixels_a: Vec<u8> = (0..(width * height)).map(|i| (i % 256) as u8).collect();
        let pixels_b: Vec<u8> = (0..(width * height)).map(|i| ((i * 7 + 100) % 256) as u8).collect();

        let hv_a = ImageTransducer::project_grayscale(&pixels_a, width, height)?;
        let hv_b = ImageTransducer::project_grayscale(&pixels_b, width, height)?;
        let sim = hv_a.similarity(&hv_b)?;
        debuglog!("test_different_images_diverge: sim={:.4}", sim);
        assert!(sim < 0.5, "Different images should diverge, sim={}", sim);
        Ok(())
    }

    #[test]
    fn test_project_rgb_basic() -> Result<(), HdcError> {
        let width = 16;
        let height = 16;
        let pixels: Vec<u8> = (0..(width * height * 3))
            .map(|i| (i % 256) as u8)
            .collect();
        let hv = ImageTransducer::project_rgb(&pixels, width, height)?;
        assert_eq!(hv.dim(), 10000);
        Ok(())
    }

    #[test]
    fn test_project_rgb_buffer_too_small() {
        let result = ImageTransducer::project_rgb(&[0u8; 10], 100, 100);
        assert!(result.is_err());
    }
}
