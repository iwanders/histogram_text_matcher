use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Glyph {
    pub hist: Vec<u8>,
    pub glyph: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GlyphSet {
    pub entries: Vec<Glyph>,
    pub line_height: u8,
    pub name: String,
}

pub fn load_glyph_set(path: &str) -> Result<GlyphSet, Box<dyn std::error::Error>> {
    use std::path::{PathBuf};
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
