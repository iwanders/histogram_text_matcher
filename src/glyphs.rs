//! Glyphs definition and helpers.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json;
use serde_yaml;

type HistogramValue = u8;

/// Representation for a single glyph.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Glyph {
    /// Histogram used to identify this glyph.
    hist: Vec<HistogramValue>,
    /// Histogram with left zero's removed.
    #[serde(skip)]
    lstrip_hist: Vec<HistogramValue>,
    /// String representation to associate with the glyph, can contain multiple characters.
    glyph: String,
}

impl Glyph {
    pub fn new(hist: &[HistogramValue], glyph: &str) -> Glyph {
        let mut z = Glyph {
            hist: hist.to_vec(),
            glyph: glyph.to_owned(),
            lstrip_hist: vec![],
        };
        z.prepare();
        z
    }

    pub fn prepare(&mut self) {
        if let Some(stripped) = self.hist.strip_prefix(&[0]) {
            self.lstrip_hist = stripped.to_vec();
        } else {
            self.lstrip_hist = self.hist.clone();
        }
    }

    pub fn hist(&self) -> &[HistogramValue] {
        &self.hist
    }

    pub fn lstrip_hist(&self) -> &[HistogramValue] {
        &self.lstrip_hist
    }
    pub fn glyph(&self) -> &str {
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

impl GlyphSet {
    pub fn prepare(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.prepare();
        }
    }
}

/// Load a glyph set from a json file.
pub fn load_glyph_set(input_path: &PathBuf) -> Result<GlyphSet, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;
    let mut file = File::open(input_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let extension = input_path.extension().expect("should have an extension");

    let mut p: GlyphSet;
    if extension == "json" {
        p = serde_json::from_str(&content)?;
    } else if extension == "yaml" {
        p = serde_yaml::from_str(&content)?;
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Unknown type",
        )));
    }
    p.prepare();
    Ok(p)
}

/// Write a glyph set to a file.
pub fn write_glyph_set(
    output_path: &PathBuf,
    set: &GlyphSet,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;

    // https://doc.rust-lang.org/std/path/struct.Path.html#method.ends_with
    // Yikes, that's a footgun.

    let extension = output_path.extension().expect("should have an extension");
    let s;
    if extension == "json" {
        s = serde_json::to_string(&set)?;
    } else if extension == "yaml" {
        s = serde_yaml::to_string(&set)?;
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Unknown type: {:?}", output_path),
        )));
    }
    let mut file = File::create(output_path)?;
    file.write(s.as_bytes())?;

    Ok(())
}
