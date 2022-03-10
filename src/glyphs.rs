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

    /// Total number of pixels in this glyph (sum of histogram).
    #[serde(skip)]
    total: u32,

    /// Index of first non-zero bin in histogram.
    #[serde(skip)]
    first_non_zero: usize,
}

impl Glyph {
    /// Create a new glyph from a histogram and glyph to represent it.
    pub fn new(hist: &[HistogramValue], glyph: &str) -> Glyph {
        let mut z = Glyph {
            hist: hist.to_vec(),
            glyph: glyph.to_owned(),
            lstrip_hist: vec![],
            total: 0,
            first_non_zero: 0,
        };
        z.prepare();
        z
    }

    /// Prepare the glyph for use.
    fn prepare(&mut self) {
        let mut i = 0usize;
        while self.hist[i] == 0 && i < self.hist.len() {
            i += 1;
        }
        self.lstrip_hist = self.hist[i..].to_vec();

        self.total = self.hist.iter().fold(0u32, |x, a| x + *a as u32);
        self.first_non_zero = self.hist().len() - self.lstrip_hist().len();
    }

    /// Total number of pixels in the histogram.
    pub fn total(&self) -> u32 {
        self.total
    }

    /// The histogram that represents this histogram.
    pub fn hist(&self) -> &[HistogramValue] {
        &self.hist
    }

    /// The histogram without empty bins on the left.
    pub fn lstrip_hist(&self) -> &[HistogramValue] {
        &self.lstrip_hist
    }

    /// The string that reprsents this glyph.
    pub fn glyph(&self) -> &str {
        &self.glyph
    }

    /// The index of the first bin in the histogram that's non zero.
    pub fn first_non_zero(&self) -> usize {
        self.first_non_zero
    }
}

/// GlyphSet holds a collection of glyphs and associated data.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct GlyphSet {
    /// List of glyphs that make up this set.
    pub entries: Vec<Glyph>,

    /// Line height used for all glyphs in this set.
    /// This is just the distance in which all characters would fit, so the bottom of the p to the
    /// top of the d.
    pub line_height: u32,

    /// Name associated to this glyph set, not required, but useful in debugging.
    pub name: String,
}

impl GlyphSet {
    /// Prepare the glyph set for use.
    pub fn prepare(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.prepare();
        }
    }
}

/// Load a glyph set from a json or yaml file.
pub fn load_glyph_set(input_path: &PathBuf) -> Result<GlyphSet, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Read;
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

fn to_yaml_string(set: &GlyphSet) -> String {
    let mut s = String::new();
    s.push_str(&format!("name: \"{}\"\n", set.name));
    s.push_str(&format!("line_height: {}\n", set.line_height));
    if !set.entries.is_empty() {
        s.push_str(&format!("entries:\n"));
        for entry in set.entries.iter() {
            s.push_str(&format!("  -\n"));
            s.push_str(&format!("    glyph: \"{}\"\n", entry.glyph));
            s.push_str(&format!(
                "    hist: {}\n",
                serde_json::to_string(&entry.hist).unwrap()
            ));
        }
    } else {
        s.push_str(&format!("entries: []\n"));
    }
    s
}

/// Write a glyph set to a json or yaml file.
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
        // Instead of relying on serde_yaml, we manually conver the glyph set here to ensure
        // newlines are convenient.
        s = to_yaml_string(&set);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_to_yaml_string_empty() {
        let mut set: GlyphSet = Default::default();
        set.name = String::from("");
        set.line_height = 137;
        let as_yaml = to_yaml_string(&set);
        let res: GlyphSet = serde_yaml::from_str(&as_yaml).unwrap();
        assert_eq!(res, set);
    }
    #[test]
    fn test_to_yaml_string_with_glyphs() {
        let mut set: GlyphSet = Default::default();
        set.name = String::from("lkdsjflds");
        set.line_height = 137;
        set.entries.push(Glyph {
            hist: vec![1, 2, 3, 4],
            glyph: String::from(" a"),
            ..Default::default()
        });
        set.entries.push(Glyph {
            hist: vec![1, 3],
            glyph: String::from("ba"),
            ..Default::default()
        });
        let as_yaml = to_yaml_string(&set);
        let res: GlyphSet = serde_yaml::from_str(&as_yaml).unwrap();
        assert_eq!(res, set);
    }
}
