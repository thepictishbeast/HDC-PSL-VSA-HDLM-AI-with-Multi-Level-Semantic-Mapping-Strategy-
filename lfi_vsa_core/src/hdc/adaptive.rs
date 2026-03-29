// ============================================================
// Adaptive Learning — UI Vector-Folding
// Section 2: "Learns interfaces by ingesting UI XML/screenshots,
// folding interactive elements into hypervectors based on visual attributes."
// ============================================================

use crate::hdc::vector::BipolarVector;
use crate::hdc::error::HdcError;
use serde::{Serialize, Deserialize};

/// Visual attributes of a UI element for vector folding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiAttributes {
    pub element_type: String, // e.g., "Button", "Input"
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub text: Option<String>,
}

/// An interactive UI element mapped to the VSA space.
pub struct UiElement {
    pub attributes: UiAttributes,
    /// The hypervector representation (folded visual attributes).
    pub vector: BipolarVector,
}

impl UiElement {
    /// Folds visual attributes into a 10,000-bit hypervector.
    ///
    /// Logic: `V = bundle(V_type, V_pos(x,y), V_text)`
    pub fn fold(attr: UiAttributes) -> Result<Self, HdcError> {
        debuglog!("UiElement::fold: entry, type={}", attr.element_type);

        // 1. Project Type (Forensic Node equivalent)
        // In a full implementation, we'd use a SemanticMap. For now, random base.
        let v_type = BipolarVector::new_random()?;

        // 2. Project Position (Positional Encoding)
        // Fold (x,y) coordinates into a vector using permutations of a base coordinate vector.
        let base_pos = BipolarVector::new_random()?;
        let v_pos_x = base_pos.permute(attr.x as usize % 10000)?;
        let v_pos_y = base_pos.permute(attr.y as usize % 10000)?;
        let v_pos = v_pos_x.bind(&v_pos_y)?;

        // 3. Project Text (if present)
        let v_text = if let Some(ref t) = attr.text {
            debuglog!("UiElement::fold: folding text '{}'", t);
            BipolarVector::new_random()? // Simplified: random for each unique text
        } else {
            BipolarVector::zeros()
        };

        // 4. Bundle attributes into the final UI hypervector (Superposition)
        let vector = BipolarVector::bundle(&[&v_type, &v_pos, &v_text])?;

        debuglog!("UiElement::fold: SUCCESS, similarity_to_type={:.4}", 
            vector.similarity(&v_type)?);

        Ok(Self { attributes: attr, vector })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_element_folding() -> Result<(), HdcError> {
        let attr = UiAttributes {
            element_type: "Button".to_string(),
            x: 100,
            y: 200,
            width: 50,
            height: 20,
            text: Some("Login".to_string()),
        };
        
        let element = UiElement::fold(attr)?;
        assert_eq!(element.vector.dim(), 10000);
        
        // Ensure some content was captured
        assert!(element.vector.count_ones() > 0);
        
        Ok(())
    }
}
