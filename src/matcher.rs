use crate::glyphs::Glyph;

/// A node in the lookup table tree.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LookupNode<'a> {
    /// Vector holding children, indexed by value in this histogram bin.
    children: Vec<Option<LookupNode<'a>>>,
    /// Histogram index where this node is located.
    index: usize,
    /// Glyphs that terminate at this node because of length.
    leafs: Vec<&'a Glyph>,
    /// All glyphs that are in this node and its descendents.
    glyphs: Vec<&'a Glyph>,
}

/// A lookup table based glyph matcher that jumps to offsets based on histogram values.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LookupMatcher<'a> {
    pub tree: LookupNode<'a>,
}

fn recurse_glyph_matching(n: &mut LookupNode, glyphs: &[Glyph], index: usize, stripped: bool) {
    // Iterate through all the glyph indices in this node.
    for glyph in n.glyphs.iter() {
        let hist = if stripped {
            if let Some(h) = glyph.lstrip_hist()
            {
                h
            }
            else
            {
                continue; // no lstrip histogram, skip this in lstrip situations.
            }
        } else {
            glyph.hist()
        };

        // Skip this glyph if its histogram is smaller than the current index being assigned.
        if hist.len() <= index {
            n.leafs.push(glyph);
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
            child.glyphs.push(glyph);
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

impl<'a> LookupMatcher<'a> {
    /// Prepare the glyph matcher from a set of glyphs.
    /// If stripped is true, lstrip_hist is used.
    /// If minimal is true, the decision graph is cut short if only one glyph remains.
    pub fn prepare(&mut self, glyphs: &'a [Glyph], stripped: bool) {
        // Assign the first node with all possible glyph indices.
        self.tree.glyphs = glyphs.iter().collect::<_>();
        // recurse from the first index and build out the tree.
        recurse_glyph_matching(&mut self.tree, &glyphs, 0, stripped);
    }

    /// Find a glyph matching the provided histogram. Returns None if no glyph exactly matches this
    /// histogram, if multiple glyphs would match perfectly it returns the one that occured earliest
    /// in the original slice used to setup the glyph matcher.
    pub fn find_match(&self, histogram: &[crate::Bin]) -> Option<&'a Glyph> {
        let mut c: &LookupNode = &self.tree;
        let mut best: &LookupNode = c;

        macro_rules! return_best {
            ( ) => {
                return match best.leafs.get(0) {
                    Some(g) => Some(*g),
                    _ => None,
                }
            };
        }

        // Iterate through the values in d.
        for b in histogram.iter() {
            // Use the value in the histrogram as index.
            let v = b.count as usize;

            if c.children.len() == 0 {
                // Reached a leaf in the tree, return the glyph... we must have one, otherwise 'c'
                // would be None.
                // We may have multiple though, in that case the glyph set is ambiguous.
                return match c.leafs.get(0) {
                    Some(g) => Some(g),
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

        fn recurser(glyphs: &[Glyph], r: &mut String, n: &LookupNode, index: usize) {
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
                    .map(|g| g.glyph().to_owned())
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

/// Matcher that returns the longest matching glyph.
#[derive(Debug, Default, Clone)]
pub struct LongestGlyphMatcher<'a> {
    matcher: LookupMatcher<'a>,
    lstrip_matcher: LookupMatcher<'a>,
}

impl<'a> LongestGlyphMatcher<'a> {
    /// Create a longest glyph matcher from the provided glyphs.
    pub fn new(glyphs: &'a [Glyph]) -> Self {
        let mut v: LongestGlyphMatcher = Default::default();
        v.matcher.prepare(glyphs, false);
        v.lstrip_matcher.prepare(glyphs, true);
        v
    }

    /// Return the internal glyph matcher used.
    pub fn matcher(&self) -> &LookupMatcher {
        &self.matcher
    }

    /// Return the internal glyph matcher used for lstripped matching.
    pub fn lstrip_matcher(&self) -> &LookupMatcher {
        &self.lstrip_matcher
    }
}

/// Implementation for the Matcher trait for the LongestGlyphMatcher.
impl<'a> crate::Matcher<'a> for LongestGlyphMatcher<'a> {
    fn find_match(&self, histogram: &[crate::Bin]) -> Option<&'a Glyph> {
        self.matcher.find_match(histogram)
    }
    fn lstrip_find_match(&self, histogram: &[crate::Bin]) -> Option<&'a Glyph> {
        self.lstrip_matcher.find_match(histogram)
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
        use crate::Bin;
        let a = Glyph::new(&[0, 0, 13, 0, 0], &"a");
        let b = Glyph::new(&[0, 0, 13, 0, 0, 0], &"b");
        let z = [a, b];
        let mut matcher: LookupMatcher = Default::default();
        matcher.prepare(&z, false);
        // In this case, both a and b would match, but b is the longer match so should be taken.
        let res = matcher.find_match(&Bin::from(&[0, 0, 13, 0, 0, 0, 1]));
        assert!(res.is_some());
        assert_eq!(res.unwrap(), &z[1]);

        // This will match a, but not b, we should get a, because it still matches perfectly.
        let res = matcher.find_match(&Bin::from(&[0, 0, 13, 0, 0, 1]));
        assert!(res.is_some());
        assert_eq!(res.unwrap(), &z[0]);
    }
}
