//! Glyphs definition and helpers.

use serde::{Deserialize, Serialize};
use serde_json;

type HistogramValue = u8;

/// Representation for a single glyph.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Glyph {
    /// Histogram used to identify this glyph.
    hist: Vec<HistogramValue>,
    /// Histogram with left zero's removed.
    #[serde(skip)]
    lstrip_hist: Vec<HistogramValue>,
    /// String representation to associate with the glyph, can contain multiple characters.
    glyph: String,
}

impl Glyph
{

    pub fn new(hist: &[HistogramValue], glyph: &str) -> Glyph
    {
        let mut z = Glyph {
                hist: hist.to_vec(),
                glyph: glyph.to_owned(),
                lstrip_hist: vec![],
            };
        z.prepare();
        z
    }

    pub fn prepare(&mut self)
    {
        let mut i = 0usize;
        while self.hist[i] == 0 && i < self.hist.len()
        {
            i += 1;
        }
        self.lstrip_hist = self.hist[i..].to_vec();
    }

    pub fn hist(&self) -> &[HistogramValue]
    {
        &self.hist
    }

    pub fn lstrip_hist(&self) -> &[HistogramValue]
    {
        &self.lstrip_hist
    }
    pub fn glyph(&self) -> &str
    {
        &self.glyph
    }
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


impl GlyphSet
{
    pub fn prepare(&mut self)
    {
        for entry in self.entries.iter_mut()
        {
            entry.prepare();
        }
    }
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
    let mut p: GlyphSet = serde_json::from_str(&content)?;
    p.prepare();
    Ok(p)
}
