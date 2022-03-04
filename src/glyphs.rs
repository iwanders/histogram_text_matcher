//! Glyphs definition and helpers.

use serde::{Deserialize, Serialize};
use serde_json;

/// Representation for a single glyph.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Glyph {
    /// Histogram used to identify this glyph.
    pub hist: Vec<u8>,
    /// String representation to associate with the glyph, can contain multiple characters.
    pub glyph: String,
}

/// GlyphSet holds a collection of glyphs and associated data.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GlyphSet {
    /// List of glyphs that make up this set.
    pub entries: Vec<Glyph>,
    /// Line height used for all glyphs in this set.
    /// This is just the distance in which all characters would fit, so the bottom of the p to the
    /// top of the d.
    pub line_height: u8,

    /// Name associated to this glyph set, not required, but useful in debugging.
    pub name: String,
}

/// Load a glyph set from a json file.
pub fn load_glyph_set(path: &str) -> Result<GlyphSet, Box<dyn std::error::Error>> {
    use std::path::PathBuf;
    let path = PathBuf::from(path);
    let _res: GlyphSet;
    use std::fs::File;
    use std::io::Read;
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let p: GlyphSet = serde_json::from_str(&content)?;
    Ok(p)
}
