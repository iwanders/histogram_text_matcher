use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Glyph {
    hist: Vec<u8>,
    letter: char,
    baseline: i8,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GlyphSet {
    entires: Vec<Glyph>,
    line_height: u8,
    name: String,
}

pub fn load_glyph_set(path: &str) -> Result<GlyphSet, Box<dyn std::error::Error>> {
    use std::path::{Path, PathBuf};
    let path = PathBuf::from(path);
    let res: GlyphSet;
    use std::fs::File;
    use std::io::Read;
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let p: GlyphSet = serde_json::from_str(&content)?;
    Ok(p)
}
