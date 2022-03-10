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

/// A node in the lookup table tree.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct Node {
    /// Vector holding children, indexed by value in this histogram bin.
    children: Vec<Option<Node>>,
    /// Histogram index where this node is located.
    index: usize,
    /// Glyphs that terminate at this node because of length.
    leafs: Vec<usize>,
    /// All glyphs that are in this node and its descendents.
    glyphs: Vec<usize>,
}

/// A lookup table based glyph matcher that jumps to offsets based on histogram values.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct GlyphMatcher {
    pub tree: Node,
}

fn recurse_glyph_matching(n: &mut Node, glyphs: &[Glyph], index: usize, stripped: bool) {
    // Iterate through all the glyph indices in this node.
    for i in n.glyphs.iter() {
        let glyph = &glyphs[*i];
        let hist = if stripped {
            glyph.lstrip_hist()
        } else {
            glyph.hist()
        };

        // Skip this glyph if its histogram is smaller than the current index being assigned.
        if hist.len() <= index {
            n.leafs.push(*i);
            continue;
        }

        // Determine the which position this glyph will have in the children vector, based on its
        // histogram value at the index being considered.
        let pos_in_children = hist[index] as usize;

        // Ensure the children vector is appropriate size.
        n.children
            .resize(std::cmp::max(n.children.len(), pos_in_children + 1), None);

        // If it is currently a None value, create a child entry in this slot.
        if let None = &n.children[pos_in_children] {
            n.children[pos_in_children] = Some(Default::default());
        }

        // Finally, assign this glyph index into this child.
        if let Some(child) = n.children[pos_in_children].as_mut() {
            child.index = index;
            child.glyphs.push(*i);
        }
    }

    // Finally, iterate over all the populated entries in the children vector
    // and ensure they get populated, recursing. Recursion terminates when the
    // children vector stays empty (at the leafs).
    for child in n.children.iter_mut() {
        if let Some(mut child_node) = child.as_mut() {
            recurse_glyph_matching(&mut child_node, &glyphs, index + 1, stripped);
        }
    }
}

impl GlyphMatcher {
    /// Prepare the glyph matcher from a set of glyphs.
    /// If stripped is true, lstrip_hist is used.
    /// If minimal is true, the decision graph is cut short if only one glyph remains.
    pub fn prepare(&mut self, glyphs: &[Glyph], stripped: bool) {
        // Assign the first node with all possible glyph indices.
        self.tree.glyphs = (0..glyphs.len()).collect::<_>();
        // recurse from the first index and build out the tree.
        recurse_glyph_matching(&mut self.tree, &glyphs, 0, stripped);
    }

    /// Find a glyph matching the provided histogram. Returns None if no glyph exactly matches this
    /// histogram, if multiple glyphs would match perfectly it returns the one that occured earliest
    /// in the original slice used to setup the glyph matcher.
    pub fn find_match(&self, histogram: &[u32]) -> Option<usize> {
        let mut c: &Node = &self.tree;
        let mut best: &Node = c;

        macro_rules! return_best {
            ( ) => {
                return match best.leafs.get(0) {
                    Some(g) => Some(*g),
                    _ => None,
                }
            };
        }

        // Iterate through the values in d.
        for k in histogram.iter() {
            // Use the value in the histrogram as index.
            let v = *k as usize;

            if c.children.len() == 0 {
                // Reached a leaf in the tree, return the glyph... we must have one, otherwise 'c'
                // would be None.
                // We may have multiple though, in that case the glyph set is ambiguous.
                return match c.leafs.get(0) {
                    Some(g) => Some(*g),
                    _ => None,
                };
            }

            // If v exceeds the length of children, we terminate the search and return the
            // best matching token so far.
            if v >= c.children.len() {
                return_best!();
            }

            // Check if we have a new node in our search tree at this histogram value.
            if let Some(ref new_c) = c.children[v] {
                c = new_c; // assign new position.

                // If we would have a leaf here, assign it to the best match, because this is a
                // valid match, but we'll continue searching to find a longer match.
                if !new_c.leafs.is_empty() {
                    best = new_c;
                }
            } else {
                // We got a none, return best matching glyph, or none.
                return_best!();
            }
        }
        None
    }

    pub fn to_dot(&self, glyphs: &[Glyph]) -> String {
        let mut res: String = String::new();
        res.push_str(
            r#"digraph g {
                fontname="Helvetica,Arial,sans-serif"
                node [fontname="Helvetica,Arial,sans-serif"]
                edge [fontname="Helvetica,Arial,sans-serif"]
                graph [
                    rankdir = "LR"
                ];
                node [
                    fontsize = "16"
                    shape = "ellipse"
                ];
                edge [
                ];
            "#,
        );

        fn recurser(glyphs: &[Glyph], r: &mut String, n: &Node, index: usize) {
            r.push_str(&format!(
                r#"
                    "n{:p}" [
                        shape = "record"
                        label = ""#,
                n
            ));
            let mut edges: String = String::new();

            r.push_str(&format!("<base> [{}] {} Glyphs ", index, n.glyphs.len()));

            let mut childs: String = String::new();
            for (i, v) in n.children.iter().enumerate() {
                r.push_str(&format!(r#" | <f{}> {}"#, i, i));
                if let Some(z) = v {
                    recurser(glyphs, &mut childs, z, index + 1);
                    edges.push_str(&format!(
                        r#"
                    "n{:p}":f{} -> "n{:p}":base [];
                    "#,
                        n, i, z
                    ));
                }
            }

            r.push_str("\"\n                    ];\n");
            // If glyphs were put in the leafs vector, show those here.
            if !n.leafs.is_empty() {
                let glyph_string = n
                    .leafs
                    .iter()
                    .map(|x| &glyphs[*x])
                    .map(|g| g.glyph.clone())
                    .collect::<Vec<String>>()
                    .join(", ")
                    .replace("\\", "\\\\")
                    .replace('"', "\\\"");
                r.push_str(&format!(
                    r#"
                        "n{:p}_leafs" [
                            shape = "record"
                            label = ""#,
                    n
                ));
                r.push_str(&format!("<base> {} Leaf: {}", n.leafs.len(), glyph_string));
                r.push_str("\"\n                        ");
                if n.leafs.len() > 1 {
                    r.push_str("fillcolor = red\n                        ");
                    r.push_str("style = filled\n                        ");
                }
                r.push_str("\n                        ];\n");
                edges.push_str(&format!(
                    r#"
                "n{:p}":base -> "n{:p}_leafs":base [];
                "#,
                    n, n,
                ));
            }

            r.push_str(&edges);
            r.push_str(&childs);
        }

        recurser(glyphs, &mut res, &self.tree, 0);
        res.push_str("}\n");

        res
    }
}

#[cfg(test)]
mod matcher_tests {
    // Following unit test is based on these comments:
    // Currently the glyph matcher (or dot visualisation can't distinguish between:
    // [0, 0, 13, 0, 0]
    // [0, 0, 13, 0, 0, 0]
    // It seems the first one would get discarded in all cases in favour of the second one.
    // Even if the second one would not match against the histogram. The first would become unreachable.
    // A slighly different change would be:
    //
    // a: [0, 0, 13, 0, 1]
    // b: [0, 0, 13, 0, 1, 3]
    // Matching against [0, 0, 13, 0, 1, 3], in which case we would want a, but in case  we match
    // against [0, 0, 13, 0, 1, 5], then b cannot match in full, so a is prefered.
    // We ideally want to match the longest token...

    use super::*;
    #[test]
    fn test_take_longest() {
        let a = Glyph::new(&[0, 0, 13, 0, 0], &"a");
        let b = Glyph::new(&[0, 0, 13, 0, 0, 0], &"b");
        let z = [a, b];
        let mut matcher: GlyphMatcher = Default::default();
        matcher.prepare(&z, false);
        // In this case, both a and b would match, but b is the longer match so should be taken.
        let res = matcher.find_match(&[0, 0, 13, 0, 0, 0, 1]);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 1);

        // This will match a, but not b, we should get a, because it still matches perfectly.
        let res = matcher.find_match(&[0, 0, 13, 0, 0, 1]);
        assert!(res.is_some());
        assert_eq!(res.unwrap(), 0);
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
    pub line_height: u8,

    /// Name associated to this glyph set, not required, but useful in debugging.
    pub name: String,

    #[serde(skip)]
    pub matcher: GlyphMatcher,
    #[serde(skip)]
    pub lstrip_matcher: GlyphMatcher,
}

impl GlyphSet {
    /// Prepare the glyph set for use.
    pub fn prepare(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.prepare();
        }
        self.matcher.prepare(&self.entries, false);
        self.lstrip_matcher.prepare(&self.entries, true);
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
