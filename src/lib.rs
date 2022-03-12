// This is really nice, should adhere to this;
// https://rust-lang.github.io/api-guidelines/naming.html

// Should consider https://rust-lang.github.io/rust-clippy/rust-1.59.0/index.html#shadow_same

pub mod glyphs;

mod interface;
pub use interface::*;

pub mod matcher;

pub mod util;

/// Type to hold a simple 1D histogram.
pub type SimpleHistogram = Vec<u8>;

use serde::{Deserialize, Serialize};

// This here ensures that we have image support when the feature is enabled, but also for all tests.
#[cfg(feature = "image_support")]
pub mod image_support;
#[cfg(test)]
pub mod image_support;

#[cfg(test)]
pub mod test_util;

/*
    Improvements:
        - Currently, a space character would be greedy and match until end of line.
          We need something to limit this from occuring more than 'n' times in a row and
          perform strip on the glyph sequence afterwards.
        - Token matcher can't split based on the labels, because the histogram may have labels
          in between glyphs that weren't colored appropriately. If we need this, we could set the
          histogram labels to a sentinel, and ignore sentinels in this split.
          Search for CONSIDER_MATCH_LABEL.

    Current issues:
        - Need a way to deal with non-perfect GlyphSet objects... disregard whitespace < N
*/

/// Function to match a single color in an image and convert this to a histogram.
pub fn image_to_simple_histogram(image: &dyn Image, color: RGB) -> SimpleHistogram {
    let mut res: SimpleHistogram = SimpleHistogram::new();
    res.resize(image.width() as usize, 0);
    for y in 0..image.height() {
        for x in 0..image.width() {
            res[x as usize] += if image.pixel(x, y) == color { 1 } else { 0 };
        }
    }
    res
}

fn calc_score_min(pattern: &[u8], to_match: &[u8], min_width: usize) -> u8 {
    let mut res: u8 = 0;
    for (x_a, b) in (0..std::cmp::max(pattern.len(), min_width)).zip(to_match.iter()) {
        let a = &(if x_a < pattern.len() {
            pattern[x_a]
        } else {
            0u8
        });
        res += if a > b { a - b } else { b - a };
    }
    res
}

/// Simple histogram matcher that removes any zero bins and just matches lowest scoring glyphs.
pub fn histogram_glyph_matcher(
    histogram: &[u8],
    set: &glyphs::GlyphSet,
    min_width: usize,
) -> Vec<(glyphs::Glyph, u8)> {
    let v = histogram;
    let mut i: usize = 0;
    let mut res: Vec<(glyphs::Glyph, u8)> = Vec::new();

    while i < v.len() - 1 {
        if v[i] == 0 {
            i += 1;
            continue;
        }
        // v[i] is now the first non-zero entry.
        let remainder = &v[i..];

        type ScoreType = u8;
        let mut scores: Vec<ScoreType> = vec![];
        scores.resize(set.entries.len(), 0 as ScoreType);
        for (glyph, score) in set.entries.iter().zip(scores.iter_mut()) {
            *score = calc_score_min(glyph.lstrip_hist(), &remainder, min_width);
        }
        // println!("{scores:?}");

        // The lowest score is the best match, (the least difference)...
        // https://stackoverflow.com/a/53908709
        let index_of_min: Option<usize> = scores
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(index, _)| index);

        if let Some(best) = index_of_min {
            let token = &set.entries[best];
            let score = scores[best];
            res.push((token.clone(), score));

            i += token.lstrip_hist().len();
        } else {
            // println!("Huh? didn't have a lowest score??");
            i += 1;
        }
    }
    res
}

/// Representation of a histogram bin and associated label color.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Bin {
    pub count: u32,
    pub label: u32,
}
impl Bin {
    /// Helper function to create a vector of bins from a slice of counts.
    pub fn vec(v: &[u32]) -> Vec<Bin> {
        v.iter()
            .map(|x| Bin {
                count: *x,
                label: 0,
            })
            .collect()
    }
}

/// Relate a particular color to a label.
type ColorLabel = (RGB, u32);

/// A glyph with an associated label.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct LabelledGlyph<'a> {
    pub glyph: &'a glyphs::Glyph,
    pub label: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Hash)]
pub struct Match2D<'a> {
    pub tokens: Vec<LabelledGlyph<'a>>,
    pub location: Rect,
}

/// A token in the histogram matching, denoting whitespace and glyphs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Token<'a> {
    WhiteSpace(usize), // Value denotes amount of whitespace pixels.
    Glyph {
        glyph: &'a glyphs::Glyph,
        label: u32,
    },
}
/// A match in the histogram at a certain position.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Match<'a> {
    /// The matched token.
    pub token: Token<'a>,
    /// Position in the histogram
    pub position: u32,
    /// Width of this token.
    pub width: u32,
}

/// Trait that provides glyph matching functionality.
pub trait Matcher<'a> {
    fn find_match(&self, histogram: &[Bin]) -> Option<&'a glyphs::Glyph>;
    fn lstrip_find_match(&self, histogram: &[Bin]) -> Option<&'a glyphs::Glyph>;
}

// There are situation where linear - longest glyph matching is not correct;
// glyph a: [0, 2, 3, 3]
// glyph b: [0, 2, 3]
// glyph c: [3, 4, 5, 6]
// And hist:[0, 2, 3, 3, 4, 5, 6]
//          [0, 2, 3, 3] <- Glyph a is the best matching glyph and longest match.
//                      [4, 5, 6] This is the remainder of the histogram, which can't be matched.
// Better:
//          [0, 2, 3] <- Match glyph A
//                   [3, 4, 5, 6] <- Match glyph C.
//
// This requires the glyph matcher to return a list of all possible glyphs for for the given
// histogram. Then we can perform a search over the possible interpretations in the histogram.
//
// This would also allow for accounting for situations where multiple glyphs map to identical
// histograms and we need context to decide which one would be best.
//
// Also allows for accounting for a whitespace character that is equal to empty histograms while
// ensuring that can't occur twice at the end of a word.

fn bin_glyph_matcher<'a>(histogram: &[Bin], matcher: &'a dyn Matcher) -> Vec<Match<'a>> {
    let mut i: usize = 0; // index into the histogram.
    let mut res: Vec<Match<'a>> = Vec::new();

    fn _pattern_matches(pattern: &[u8], to_match: &[Bin]) -> bool {
        let min = std::cmp::min(pattern.len(), to_match.len());
        let a = pattern[0..min].iter().map(|x| *x);
        let b = to_match[0..min].iter().map(|x| x.count as u8);
        a.eq(b)
    }

    // Boolean to keep track of whether we are using stripped values or non stripped values
    // to compare.
    let mut use_stripped = true;

    while i < histogram.len() - 1 {
        // If we are using stripped symbols, remove the padding from the left, this will be very
        // fast.
        if use_stripped {
            if histogram[i].count == 0 {
                // This checks if the last entry in the current matches is a whitespace token,
                // if it is, we will add one to it, otherwise, we push a new whitespace token.
                // CONSIDER: this is less than ideal, may want to do something smart here
                // run through it once to identify whitespace jumps at the top to prepare
                // then here just do a single jump if within one of the whitespace intervals.

                // https://stackoverflow.com/a/32554326
                let mut last = res.last_mut();
                if last.is_some()
                    && std::mem::discriminant(&last.as_ref().unwrap().token)
                        == std::mem::discriminant(&Token::WhiteSpace(0))
                {
                    last.as_mut().unwrap().width += 1;
                    if let Token::WhiteSpace(ref mut z) = last.unwrap().token {
                        *z += 1;
                    }
                } else {
                    res.push(Match {
                        token: Token::WhiteSpace(1),
                        position: i as u32,
                        width: 1,
                    });
                }
                i += 1;
                continue;
            }
        }

        // CONSIDER: Splitting the histogram by labels at the start, then match on the labels.
        // Next, make sure we only match the label found in the first bin.
        // This is problematic... Since space between letters may not have the label set.
        // Disabled this for now. See CONSIDER_MATCH_LABEL.
        // let max_index = remainder.iter().position(|x| x.label != remainder[0].label);
        // let remainder = &histogram[i..i + max_index.unwrap_or(remainder.len())];

        /*
        // This was the old linear search over all glyphs.
        // It should be re-evaluated when we can order the glyphs by occurence rate
        // it may be more performant than the more random access of the matcher.

        // Get the first glyph that matches with zero cost:
        let mut index_of_min: Option<usize> = None;

        // We got a non zero entry in the first bin now.
        let remainder = &histogram[i..];

        for (zz, glyph) in set.entries.iter().enumerate() {
            let hist_to_use = if use_stripped {
                glyph.lstrip_hist()
            } else {
                glyph.hist()
            };

          let exactly_equal = _pattern_matches(hist_to_use, remainder);
          if exactly_equal {
            index_of_min = Some(zz);
            break;
          }
        }
        */

        let remainder = &histogram[i..];
        let glyph_search: Option<&glyphs::Glyph>;
        if !use_stripped {
            glyph_search = matcher.find_match(&remainder);
        } else {
            glyph_search = matcher.lstrip_find_match(&remainder);
        }

        if let Some(found_glyph) = glyph_search {
            // Calculate the true position, depends on whether we used stripped values.
            let first_non_zero = found_glyph.first_non_zero();
            let position = i as u32 - if use_stripped { first_non_zero } else { 0 } as u32;
            // Position where histogram where this letter has the first non-zero;
            let label_position = position as usize + first_non_zero;

            // Add the newly detected glyph
            res.push(Match {
                position: position,
                token: Token::Glyph {
                    glyph: found_glyph,
                    label: histogram[label_position].label,
                },
                width: found_glyph.hist().len() as u32,
            });

            // Advance the cursor by the width of the glyph we just matched.
            i += if use_stripped {
                found_glyph.lstrip_hist().len()
            } else {
                found_glyph.hist().len()
            };

            use_stripped = false;
        } else {
            i += 1;
            use_stripped = true; // Switch to using stripped, we didn't get a perfect match.
        }
    }
    res
}

/// Struct to represent a rectangle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Hash)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    /// Return whether this rectangle overlaps with the provided rectangle. Including boundary.
    pub fn overlaps(&self, b: &Rect) -> bool {
        self.right() >= b.left()
            && b.right() >= self.left()
            && self.top() >= b.bottom()
            && b.top() >= self.bottom()
    }
    /// Return whether this rectangle overlaps with the provided rectangle. Excluding boundary.
    pub fn overlaps_excluding(&self, b: &Rect) -> bool {
        self.right() > b.left()
            && b.right() > self.left()
            && self.top() > b.bottom()
            && b.top() > self.bottom()
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.left() && x <= self.right() && y >= self.bottom() && y <= self.top()
    }

    /// The highest y value of the rectangle (bottom in image coordinates!)
    pub fn top(&self) -> u32 {
        self.y + self.h
    }

    /// The lowest y value of the rectangle (top in image coordinates!)
    pub fn bottom(&self) -> u32 {
        self.y
    }

    /// The lowest x value of the rectangle.
    pub fn left(&self) -> u32 {
        self.x
    }

    /// The highest x value of the rectangle.
    pub fn right(&self) -> u32 {
        self.x + self.w
    }

    /// The width of the rectangle.
    pub fn width(&self) -> u32 {
        self.w
    }

    /// The height of the rectangle.
    pub fn height(&self) -> u32 {
        self.h
    }
}

use std::collections::VecDeque;

// Helper to add a row
fn add_pixel(x: usize, p: &RGB, labels: &[ColorLabel], histogram: &mut Vec<Bin>) {
    for (color, label) in labels.iter() {
        if color == p {
            histogram[x].count += 1;
            histogram[x].label = *label;
            return;
        }
    }
}

// Helper to subtract a row.
fn sub_pixel(x: usize, p: &RGB, labels: &[ColorLabel], histogram: &mut Vec<Bin>) {
    for (color, _label) in labels.iter() {
        if color == p {
            histogram[x].count = histogram[x].count.saturating_sub(1);
            return;
        }
    }
}

// Helper to accept or discard matches based on whether they have moved out of the window.
fn finalize_considerations<'a>(
    y: u32,
    res_consider: &mut VecDeque<Match2D<'a>>,
    res_final: &mut Vec<Match2D<'a>>,
) {
    while !res_consider.is_empty() && res_consider.front().unwrap().location.top() < y {
        res_final.push(res_consider.pop_front().unwrap());
    }
}

// Helper to decide on matches that overlap with other matches.
fn decide_on_matches<'a>(
    y: u32,
    window_size: u32,
    matches: &[Match<'a>],
    res_consider: &mut VecDeque<Match2D<'a>>,
) {
    // Matches are 1D matches, we want consecutive glyph blocks.
    // https://github.com/rust-lang/rust/issues/80552 would be nice... but lets stick
    // with stable for now.

    // So, whitespace in matches, which delimit the consecutive glyph blocks.
    let mut match_index: usize = 0;
    while match_index < matches.len() {
        // Skip over whitespace matches
        if let Token::WhiteSpace(_) = matches[match_index].token {
            match_index += 1;
            continue;
        }

        if let Token::Glyph { .. } = matches[match_index].token {
            // Find the index where this consecutive glyph block ends.
            let block_end = matches[match_index..]
                .iter()
                .position(|z| {
                    std::mem::discriminant(&z.token)
                        == std::mem::discriminant(&Token::WhiteSpace(0))
                })
                .unwrap_or(matches.len() - match_index);

            // Determine the slice of glyphs, first and last.
            let glyphs = &matches[match_index..match_index + block_end];
            let first_glyph = glyphs.first().expect("never empty");
            let last_glyph = glyphs.last().expect("never empty");

            // Use the width of the match and to create a bounding box for this glyph block.
            let block_width = last_glyph.position + last_glyph.width - first_glyph.position;
            let this_block_region = Rect {
                x: first_glyph.position,
                y,
                w: block_width - 1, // -1 to stay inside our window instead of one pixel beyond.
                h: window_size - 1,
            };

            // Determine the number of pixels this glyph sequence matched.
            let current_matching = glyphs
                .iter()
                .map(|z| {
                    if let Token::Glyph { glyph, .. } = z.token {
                        glyph.total()
                    } else {
                        0
                    }
                })
                .fold(0, |x, a| x + a);
            // Now, we need to decide whether this block of glyphs is better than the ones currently
            // in res_consider.

            // Options:
            //   - No overlap, always add this glyph.
            //   - Overlap, decide which glyph is the best, remove the other.

            // Check if this overlaps with others in the consideration buffer;
            let mut do_insert = true;
            *res_consider = res_consider
                .drain(..)
                .filter(|m| {
                    if do_insert == false {
                        return true; // already discarded current glyph, keep everything.
                    }

                    // Check if this block overlaps the match were checking against.
                    if m.location.overlaps(&this_block_region) {
                        // We overlap, and the current glyph sequence is still under consideration;
                        // Decide if better, more matching pixels is better, likely a longer
                        // word, or more complex glyph got matched.
                        let mlen = m
                            .tokens
                            .iter()
                            .map(|z| z.glyph.total())
                            .fold(0, |x, a| x + a);

                        // Make the decision.
                        let new_is_better = current_matching >= mlen;

                        if !new_is_better {
                            // new is not better than what we have
                            do_insert = false; // ensure we don't insert.
                            return true; // keep old
                        } else {
                            return false; // drop old.
                        }
                    }
                    return true; // no overlap, always keep m under consideration.
                })
                .collect::<_>();

            if do_insert {
                // We should insert our current entry.
                res_consider.push_back(Match2D {
                    tokens: glyphs
                        .iter()
                        .map(|z| match z.token {
                            Token::Glyph { glyph, label } => LabelledGlyph { glyph, label },
                            _ => panic!("should never have whitespace here"),
                        })
                        .collect::<_>(),
                    location: this_block_region,
                });
            }

            match_index += glyphs.len();
        }
    }
}

/// Function to slide a window over an image and match glyphs for each histogram thats created.
pub fn moving_windowed_histogram<'a>(
    image: &dyn Image,
    window_size: u32,
    matcher: &'a dyn Matcher,
    labels: &[ColorLabel],
) -> Vec<Match2D<'a>> {
    let mut res_final: Vec<Match2D<'a>> = Vec::new();

    // Container for results under consideration, we check matches against overlap in this window
    // and keep the parts that are the best matches.
    let mut res_consider: VecDeque<Match2D<'a>> = VecDeque::new();
    // Once the matches here move out of the window, we move them to res itself.

    let mut histogram: Vec<Bin> = Vec::<Bin>::new();
    histogram.resize(image.width() as usize, Default::default());

    // Start at the top, with zero width, then we sum rows for the window size
    // Then, we iterate down, at the bottom of the window add to the histogram
    // and the top row that moves out we subtract.

    use std::time::Instant;
    let mut duration_hist = 0.0;
    let mut duration_matcher = 0.0;
    let mut duration_decider = 0.0;
    let mut duration_finalizer = 0.0;

    // Let us first, setup the first histogram, this is from 0 to window size.
    let now = Instant::now();
    for y in 0..window_size {
        for x in 0..image.width() {
            let p = image.pixel(x, y);
            add_pixel(x as usize, &p, labels, &mut histogram);
        }
    }
    duration_hist += now.elapsed().as_secs_f64();

    for y in 0..((image.height() - window_size) as u32) {
        // Here, we match the current histogram, and store matches.

        // Find glyphs in the histogram.
        let now = Instant::now();
        let matches = bin_glyph_matcher(&histogram, matcher);
        duration_matcher += now.elapsed().as_secs_f64();

        // Decide which matches are to be kept.
        let now = Instant::now();
        decide_on_matches(y, window_size, &matches, &mut res_consider);
        duration_decider += now.elapsed().as_secs_f64();

        // Move matches from res_consider to res_final.
        let now = Instant::now();
        finalize_considerations(y, &mut res_consider, &mut res_final);
        duration_finalizer += now.elapsed().as_secs_f64();

        let now = Instant::now();
        // Subtract from the side moving out of the histogram.
        for x in 0..image.width() {
            let p = image.pixel(x, y);
            sub_pixel(x as usize, &p, labels, &mut histogram);
        }

        // Add the side moving into the histogram.
        for x in 0..image.width() {
            let p = image.pixel(x, y + window_size);
            add_pixel(x as usize, &p, labels, &mut histogram);
        }
        duration_hist += now.elapsed().as_secs_f64();
    }

    res_final.extend(res_consider.drain(..));

    println!("duration_hist: {duration_hist: >10.6}");
    println!("duration_matcher: {duration_matcher: >10.6}");
    println!("duration_decider: {duration_decider: >10.6}");
    println!("duration_finalizer: {duration_finalizer: >10.6}");

    res_final
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_util::test_alphabet::{
        render_standard_alphabet, render_standard_color, standard_alphabet,
    };

    fn simple_histogram_to_bin_histogram(hist: &SimpleHistogram) -> Vec<Bin> {
        hist.iter()
            .map(|x| Bin {
                count: *x as u32,
                label: 0,
            })
            .collect::<_>()
    }

    #[test]
    fn test_histogram_glyph_matcher() {
        let rgb_image = render_standard_alphabet();
        let image = image_support::rgb_image_to_view(&rgb_image);
        let hist = image_to_simple_histogram(&image, RGB::white());
        let mut glyph_set = image_support::dev_image_to_glyph_set(&rgb_image, Some(0), &None);
        glyph_set.prepare();

        println!("Histogram: {hist:?}");

        let res = histogram_glyph_matcher(&hist, &glyph_set, 5);

        assert!(res.len() == 6);
        for (g, score) in res.iter() {
            println!("{g:?}: {score}");
            assert!(*score == 0);
        }
    }

    #[test]
    fn test_bin_glyph_matcher() {
        let rgb_image = render_standard_alphabet();
        let image = image_support::rgb_image_to_view(&rgb_image);
        let hist = image_to_simple_histogram(&image, RGB::white());
        let mut glyph_set = image_support::dev_image_to_glyph_set(&rgb_image, Some(0), &None);
        glyph_set.prepare();
        let matcher = matcher::LongestGlyphMatcher::new(&glyph_set.entries);

        let binned = simple_histogram_to_bin_histogram(&hist);

        let matches = bin_glyph_matcher(&binned, &matcher);

        let mut glyph_counter = 0;
        for (i, v) in matches.iter().enumerate() {
            println!("{i}: {v:?}");
            if let Token::Glyph { glyph, .. } = v.token {
                assert!(*glyph == glyph_set.entries[glyph_counter]);
                glyph_counter += 1;
            }
        }
    }

    #[test]
    fn test_moving_window() {
        println!();

        use std::path::Path;
        let location = String::from("/tmp/test_moving_window/");
        let have_dir = Path::new(&location).is_dir();
        if !have_dir
        {
            println!("Directory {} does not exist, create it to write files for inspection.", &location);
        }


        use image::RgbImage;
        use std::path::PathBuf;

        // Create the glyph set.
        let (glyph_image, glyph_text) = standard_alphabet();
        let mut glyph_set = image_support::dev_image_to_glyph_set(&glyph_image, Some(0), &None);
        // Patch up the glyph set's glyphs.
        for (i, c) in glyph_text.chars().enumerate() {
            let old_glyph = &glyph_set.entries[i];
            glyph_set.entries[i] = glyphs::Glyph::new(old_glyph.hist(), &String::from(c));
        }
        glyph_set.prepare();

        let matcher = matcher::LongestGlyphMatcher::new(&glyph_set.entries);

        // Create an image with some text in it at various places, different colors and various
        // offset line alignments.
        let mut image = RgbImage::new(200, 100);

        let white = RGB::white();
        let red = RGB::red();
        let blue = RGB::blue();

        let locations = [
            (10u32, 10u32, "caab", red.to_rgb()),
            (50, 13, "deeb", white.to_rgb()),
            (100u32, 10u32, "waab", blue.to_rgb()),
            (150, 13, "wacb", white.to_rgb()),
            (50, 20, "dwaaaaaa", red.to_rgb()), // Touches the other DEEB.
            (10u32, 50u32, "cba", blue.to_rgb()),
        ];

        for (x, y, text, color) in locations.iter() {
            render_standard_color(&mut image, *x, *y, text, *color);
        }


        if have_dir
        {
            let _ = image.save(location.to_owned() + "input_image.png").unwrap();
        }

        let image = image_support::rgb_image_to_view(&image);
        let labels = vec![(white, 0), (red, 1), (blue, 2)];

        let matches = moving_windowed_histogram(&image, glyph_set.line_height, &matcher, &labels);

        if have_dir {
            util::write_match_html(
                image.width(),
                image.height(),
                &matches,
                &PathBuf::from(location.to_owned() + "input_image.png"),
                &PathBuf::from(location.to_owned() + "moving_window.html"),
            )
            .expect("");
        }

        for m in matches.iter() {
            let location = &m.location;
            print!("{location:?} -> ");
            for t in m.tokens.iter() {
                let l = t.label;
                let g = t.glyph.glyph();
                print!(" {g:?}#{l}");
            }
            let z = m
                .tokens
                .iter()
                .map(|t| t.glyph.glyph().to_owned())
                .collect::<Vec<String>>()
                .join("");
            print!(" -> {z}");
            println!();
        }

        // Finally test them.
        let mut matches = matches;
        for (x, y, text, color) in locations.iter() {
            // We know the position, there must be a single element in the match_set for this
            // position.
            let index = matches
                .iter()
                .position(|m| m.location.contains(x + 3, y + 3))
                .expect("could not find match for this text");
            // Now, we found the index, check if this match matches the original input.
            let m = &matches[index];
            let z = m
                .tokens
                .iter()
                .map(|t| t.glyph.glyph().to_owned())
                .collect::<Vec<String>>()
                .join("");
            assert_eq!(z, *text);

            for t in m.tokens.iter() {
                for (label_color, label) in labels.iter() {
                    if *label == t.label {
                        assert_eq!(color, label_color);
                    }
                }
            }

            matches.remove(index);
        }
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn histogram_matcher_real() {
        // Somehow... this fails :\
        let s1: Vec<u8> = vec![0, 0, 13, 13, 1, 1, 3, 4, 5, 3, 0];
        let s2: Vec<u8> = vec![0, 5, 3, 2, 3, 2, 2, 2, 3, 2, 4, 0, 0];
        let s3: Vec<u8> = vec![0, 0, 11, 2, 2, 2, 2, 2, 2, 0];
        let s4: Vec<u8> = vec![0, 1, 1, 1, 1, 10, 10, 1, 1, 1, 1, 1, 0];
        let s5: Vec<u8> = vec![0, 0, 4, 2, 0, 1, 3, 2, 0];
        let s6: Vec<u8> = vec![0, 0, 1, 0, 0, 0, 0, 0];

        let mut glyph_set: glyphs::GlyphSet = Default::default();
        glyph_set.entries.push(glyphs::Glyph::new(&s1, &"s1"));
        glyph_set.entries.push(glyphs::Glyph::new(&s2, &"s2"));
        glyph_set.entries.push(glyphs::Glyph::new(&s3, &"s3"));
        glyph_set.entries.push(glyphs::Glyph::new(&s4, &"s4"));
        glyph_set.entries.push(glyphs::Glyph::new(&s5, &"s5"));
        glyph_set.entries.push(glyphs::Glyph::new(&s6, &"s6"));
        glyph_set.line_height = 1;
        glyph_set.prepare();
        let matcher = matcher::LongestGlyphMatcher::new(&glyph_set.entries);
        println!("Glyph set: {glyph_set:?}");

        let mut input: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        input.extend(s1);
        input.extend(vec![0, 0, 0, 0, 0]);
        input.extend(s2);
        input.extend(vec![0, 0, 0]);
        input.extend(s3);
        input.extend(vec![0, 0]);
        input.extend(s4);
        input.extend(vec![0]);
        input.extend(s5);
        input.extend(vec![0, 0, 0, 0, 0]);
        input.extend(s6);
        input.extend(vec![0, 0, 0, 0, 0]);

        let binned = simple_histogram_to_bin_histogram(&input);

        let matches = bin_glyph_matcher(&binned, &matcher);
        let mut glyph_counter = 0;
        for (i, v) in matches.iter().enumerate() {
            println!("{i}: {v:?}");
            if let Token::Glyph { glyph, .. } = v.token {
                assert!(*glyph == glyph_set.entries[glyph_counter]);
                glyph_counter += 1;
            }
        }
        let mut res_consider: VecDeque<Match2D> = Default::default();

        decide_on_matches(0, glyph_set.line_height as u32, &matches, &mut res_consider);
        assert_eq!(res_consider.len(), 6);
        println!("res_consider: {res_consider:?}");
    }

    #[test]
    fn render_readme_images() {
        // This entire function is a bit ugly... we write some images to disk that we then
        // read back etc...
        use image::open;
        use image::{Rgb, RgbImage};
        use std::path::Path;

        let location = String::from("/tmp/readme_image_dir/");
        let have_dir = Path::new(&location).is_dir();
        if !have_dir {
            return;
        }

        let glyph_text = "abc";
        let mut original_image = RgbImage::new(28, 28);
        crate::test_util::test_alphabet::render_standard(&mut original_image, 0, 10, &glyph_text);

        let scale_factor = 5u32;
        let final_zoom = 1u32;

        let output_dir = if have_dir {
            Some(location.as_str())
        } else {
            None
        };

        let readme_glyph_image = location.clone() + "readme_glyphs.png";
        let scaled_readme_glyphs = crate::image_support::scale_image(&original_image, scale_factor);
        let _ = scaled_readme_glyphs.save(&readme_glyph_image).unwrap();

        let image = open(&readme_glyph_image)
            .expect(&format!("could not load image at {:?}", readme_glyph_image))
            .to_rgb8();

        // Print the real histograms
        {
            let mut glyph_set =
                image_support::dev_image_to_glyph_set(&original_image, Some(0), &None);
            glyph_set.entries[0] = glyphs::Glyph::new(glyph_set.entries[0].hist(), &"a");
            glyph_set.entries[1] = glyphs::Glyph::new(glyph_set.entries[1].hist(), &"b");
            glyph_set.entries[2] = glyphs::Glyph::new(glyph_set.entries[2].hist(), &"c");
            glyph_set.prepare();
            for g in glyph_set.entries.iter() {
                println!("{g:?}");
            }

            let matcher = matcher::LongestGlyphMatcher::new(&glyph_set.entries);
            use std::fs::File;
            use std::io::Write;
            let mut file = File::create(location.to_owned() + "readme_glyphs.dot").unwrap();
            file.write(&matcher.matcher().to_dot(&glyph_set.entries).as_bytes())
                .unwrap();
        }

        // Calculate the glyph set for the scaled image to output the segmentation.
        let glyph_set = image_support::dev_image_to_glyph_set(&image, Some(0), &output_dir);

        let glyph_set_output = location.clone() + "dev_histogram_boxes.png";
        let image = open(&glyph_set_output)
            .expect(&format!("could not load image at {:?}", &glyph_set_output))
            .to_rgb8();
        let mut image_mut = image.clone();

        let matcher = matcher::LongestGlyphMatcher::new(&glyph_set.entries);
        let image = image_support::rgb_image_to_view(&scaled_readme_glyphs);
        let labels = vec![(RGB::white(), 0)];

        let matches = moving_windowed_histogram(&image, glyph_set.line_height, &matcher, &labels);

        // We should draw a grid here.
        let bottom = 26 * scale_factor;
        for v in matches.iter() {
            let mut left_offset = 0;
            for t in v.tokens.iter() {
                let glyph = t.glyph;
                let hist = glyph.hist();
                for x in 0..hist.len() {
                    let img_x = left_offset + v.location.left() + x as u32;
                    for y in 0..hist[x] {
                        let c = Rgb([255u8, 0, 0]);
                        *(image_mut.get_pixel_mut(img_x, bottom + 1 - (y as u32))) = c;
                    }
                }
                left_offset += hist.len() as u32 + 1; // bit of a hack this + 1
            }
        }

        // Finally, copy the white letters that were the original input to the top of the image
        // crate::test_util::test_alphabet::render_standard(&mut original_image, 0, 10, &glyph_text);
        let original_start = 10u32;
        let original_end = 18u32;
        let offset = (original_start - 1) * scale_factor;
        for y in (original_start * scale_factor)..(original_end * scale_factor) {
            for x in 0..scaled_readme_glyphs.width() {
                *image_mut.get_pixel_mut(x, y - offset) = *scaled_readme_glyphs.get_pixel(x, y);
            }
        }

        let _ = crate::image_support::scale_image(&image_mut, final_zoom)
            .save(location.to_owned() + "readme_glyphs_hist.png")
            .unwrap();
    }
}
