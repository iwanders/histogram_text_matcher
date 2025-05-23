// This is really nice, should adhere to this;
// https://rust-lang.github.io/api-guidelines/naming.html

// Should consider https://rust-lang.github.io/rust-clippy/rust-1.59.0/index.html#shadow_same

// https://doc.rust-lang.org/rustc/profile-guided-optimization.html
//
// https://releases.llvm.org/11.0.1/docs/Benchmarking.html
// https://bheisler.github.io/criterion.rs/book/user_guide/command_line_options.html#baselines

pub mod glyphs;

// mod interface;
// pub use interface::*;

pub use image;
use image::{GenericImageView, Pixel, Rgb};

pub mod matcher;

pub mod util;

/// Type to hold a simple 1D histogram.
pub type SimpleHistogram = Vec<u8>;

use serde::{Deserialize, Serialize};

// This here ensures that we have image support when the feature is enabled, but also for all tests.
pub mod image_support;

#[cfg(test)]
pub mod test_util;

/*
    Improvements:
        - Keep track of histogram bin per match-color to avoid accidentally matching pixels from
          ruining the day.
          Lets do this!

    Current issues:
        - Need a way to deal with non-perfect GlyphSet objects... disregard whitespace < N -> Or...
          just make the glyph set perfect.
*/

/// Function to match a single color in an image and convert this to a histogram.
pub fn image_to_simple_histogram<I: GenericImageView>(image: &I, color: I::Pixel) -> SimpleHistogram
where
    I::Pixel: Eq,
{
    let mut res: SimpleHistogram = SimpleHistogram::new();
    res.resize(image.width() as usize, 0);
    for y in 0..image.height() {
        for x in 0..image.width() {
            res[x as usize] += if image.get_pixel(x, y) == color { 1 } else { 0 };
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
            // In this function, all glyphs must have an lstrip histogram as all whitespace is
            // stripped.
            let hist = glyph
                .lstrip_hist()
                .expect("all glyphs must have lstrip histogram");
            *score = calc_score_min(hist, &remainder, min_width);
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

            i += token
                .lstrip_hist()
                .expect("all glyphs must have lstrip histogram")
                .len();
        } else {
            // println!("Huh? didn't have a lowest score??");
            i += 1;
        }
    }
    res
}

/// The data type used in histograms, this is also the maximum height of the window.
pub type HistogramType = u32; // Do not make this u16, that's much slower.

/// Relate a particular color to a label.
pub type ColorLabel = (Rgb<u8>, u32);

/// A glyph with an associated label.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct LabelledGlyph<'a> {
    pub glyph: &'a glyphs::Glyph,
    pub label: u32,
}

/// A 2D match is a sequence of consecutive glyphs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Hash)]
pub struct Match2D<'a> {
    /// The glyphs that make up this match.
    pub tokens: Vec<LabelledGlyph<'a>>,
    /// The location that is covered by this match.
    pub location: Rect,
}

impl<'a> Match2D<'a> {
    /// Concatenate all tokens into a string.
    pub fn to_string(&self) -> String {
        self.tokens
            .iter()
            .map(|t| t.glyph.glyph())
            .collect::<Vec<&str>>()
            .join("")
    }
}

/// A 1D token found the histogram matching, denoting whitespace and glyphs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Token<'a> {
    WhiteSpace(usize), // Value denotes amount of whitespace pixels.
    Glyph {
        glyph: &'a glyphs::Glyph,
        label: u32,
    },
}
/// A 1D match in the histogram at a certain position.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Match<'a> {
    /// The matched token, can be whitespace or a glyph.
    pub token: Token<'a>,
    /// Position in the histogram
    pub position: u32,
    /// Width of this token.
    pub width: u32,
}

/// Trait that provides glyph matching functionality.
pub trait Matcher {
    fn find_match(&self, histogram: &[HistogramType]) -> Option<&glyphs::Glyph>;
    fn lstrip_find_match(&self, histogram: &[HistogramType]) -> Option<&glyphs::Glyph>;
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

/// This function takes a histogram made up of bins and a matcher, with this it creates a set of
/// matches in this particular histogram. These 1D matches hold tokens, either whitespace or
/// glyphs at a particular position.
pub fn bin_glyph_matcher<'a>(
    labelled_histogram: &LabelledHistogram,
    matcher: &'a (impl Matcher + ?Sized),
) -> Vec<Match<'a>> {
    let mut i: usize = 0; // index into the histogram.
    let mut res: Vec<Match<'a>> = Vec::with_capacity(32);

    fn _pattern_matches(pattern: &[u8], to_match: &[HistogramType]) -> bool {
        let min = std::cmp::min(pattern.len(), to_match.len());
        let a = pattern[0..min].iter().map(|x| *x);
        let b = to_match[0..min].iter().map(|x| *x as u8);
        a.eq(b)
    }

    let histogram = &labelled_histogram.histogram;

    // Boolean to keep track of whether we are using stripped values or non stripped values
    // to compare.
    let mut use_stripped = true;

    while i < histogram.len() - 1 {
        // If we are using stripped symbols, remove the padding from the left, this will be very fast.
        if use_stripped {
            if histogram[i] == 0 {
                // This checks if the last entry in the current matches is a whitespace token,
                // if it is, we will add one to it, otherwise, we push a new whitespace token.
                // CONSIDER: this is less than ideal, may want to do something smart here
                // run through it once to identify whitespace jumps at the top to prepare
                // then here just do a single jump if within one of the whitespace intervals.

                // Find the up to the next index that's non zero.
                let next_nonzero = histogram[i..].iter().position(|v| *v != 0);
                let width = next_nonzero.unwrap_or(histogram.len() - i);
                res.push(Match {
                    token: Token::WhiteSpace(width - 1),
                    position: i as u32,
                    width: width as u32 - 1,
                });
                i += width;
                continue;
            }
        }

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
            let position =
                (i as u32).saturating_sub(if use_stripped { first_non_zero } else { 0 } as u32);
            // Position where histogram where this letter has the first non-zero;

            // Add the newly detected glyph
            res.push(Match {
                position: position,
                token: Token::Glyph {
                    glyph: found_glyph,
                    label: labelled_histogram.label.1,
                },
                width: found_glyph.hist().len() as u32,
            });

            // Advance the cursor by the width of the glyph we just matched.
            i += if use_stripped {
                found_glyph
                    .lstrip_hist()
                    .expect("must have had a lstrip histogram to find it")
                    .len()
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

/// Return a string as best matched from the provided histogram.
///
/// This is the function to use when matching a single line to a single string.
pub fn match_histogram_to_string(histogram: &[u8], matcher: &dyn Matcher) -> String {
    let labelled_histogram = LabelledHistogram::from_u8(histogram, (Rgb([255, 255, 255]), 0));
    let matches = bin_glyph_matcher(&labelled_histogram, matcher);
    let mut s = String::new();
    for m in matches {
        match m.token {
            Token::WhiteSpace(_) => {}
            Token::Glyph { glyph, .. } => {
                s.push_str(glyph.glyph());
            }
        }
    }
    s.trim().to_owned()
}

/// Struct to represent a rectangle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Hash, Deserialize, PartialOrd)]
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

/// Helper to accept  matches if they have moved out of the window.
fn finalize_considerations<'a>(
    y: u32,
    res_consider: &mut VecDeque<Match2D<'a>>,
    res_final: &mut Vec<Match2D<'a>>,
) {
    while !res_consider.is_empty() && res_consider.front().unwrap().location.top() < y {
        res_final.push(res_consider.pop_front().unwrap());
    }
}

/// Helper to decide on matches that overlap with other matches.
fn decide_on_matches<'a>(matches: Vec<Match2D<'a>>, res_consider: &mut VecDeque<Match2D<'a>>) {
    for current_match in matches {
        // Determine the number of pixels this glyph sequence matched.
        let current_matching = current_match
            .tokens
            .iter()
            .map(|z| z.glyph.total())
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
                if m.location.overlaps(&current_match.location) {
                    // We overlap, and the current glyph sequence is still under consideration;
                    // Decide if better, more matching pixels is better, likely a longer
                    // word, or more complex glyph got matched.
                    let mlen = m
                        .tokens
                        .iter()
                        .map(|z| z.glyph.total())
                        .fold(0, |x, a| x + a);

                    // Make the decision.
                    let new_is_better = current_matching > mlen;

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
            res_consider.push_back(current_match.clone());
        }
    }
}

/// Function that resolves the individual 1D matches into consecutive glyphs and their Match2D
/// Representation. Also deals with special things like glyphs that may not be consecutive like
/// space characters.
pub fn match_resolver<'a>(y: u32, window_size: u32, matches: &[Match<'a>]) -> Vec<Match2D<'a>> {
    let mut res: Vec<Match2D<'a>> = vec![];

    // So, whitespace in matches, which delimit the consecutive glyph blocks.
    let mut match_index: usize = 0;
    while match_index < matches.len() {
        // Skip over whitespace matches
        if let Token::WhiteSpace(_) = matches[match_index].token {
            match_index += 1;
            continue;
        }

        if let Token::Glyph { glyph, .. } = matches[match_index].token {
            // Trimming left is easy, just check if this glyph should be tirmmed and advance if so.
            if glyph.trim_left() {
                match_index += 1;
                continue;
            }

            // Find the index where this consecutive glyph block ends, this is either on whitespace
            // or when max_consecutive is reached for a particular glyph.
            let mut block_end = matches.len() - match_index;
            let mut consecutive_counter: Option<(&glyphs::Glyph, usize)> = None;
            for (potential_i, potential) in matches[match_index..].iter().enumerate() {
                match potential.token {
                    Token::WhiteSpace(_v) => {
                        block_end = potential_i;
                        break;
                    }
                    Token::Glyph { ref glyph, .. } => {
                        // Check if this glyph has a maximum consecutive count.
                        if let Some(max_consecutive) = glyph.max_consecutive() {
                            // Check if there's already a counter.
                            if let Some((counting_glyph, count)) = consecutive_counter.as_mut() {
                                // Check if the glyph is identical, else clear it.
                                if glyph != counting_glyph {
                                    consecutive_counter = None;
                                } else {
                                    *count += 1;
                                    // Same glyph, increment the counter.
                                    if *count > max_consecutive {
                                        block_end = potential_i;
                                        break;
                                    }
                                }
                            } else {
                                // No counter yet, but we should have one.
                                consecutive_counter = Some((glyph, 1));
                            }
                        } else {
                            consecutive_counter = None;
                        }
                    }
                }
            }

            // Determine the slice of glyphs, first and last.
            let glyphs = &matches[match_index..match_index + block_end];
            let untrimmed_length = glyphs.len();

            // Now, all that remains is trimming the right side.
            // Perform trimming if glyphs on the sides require so.
            let mut right_limit = glyphs.len();
            for rev_i in (0..glyphs.len()).rev() {
                if let Token::Glyph { glyph, .. } = glyphs[rev_i].token {
                    if glyph.trim_right() {
                        right_limit = rev_i;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            let glyphs = &glyphs[..right_limit];

            if glyphs.is_empty() {
                match_index += untrimmed_length;
                continue;
            }

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

            // We should insert our current entry.
            res.push(Match2D {
                tokens: glyphs
                    .iter()
                    .map(|z| match z.token {
                        Token::Glyph { glyph, label } => LabelledGlyph { glyph, label },
                        _ => panic!("should never have whitespace here"),
                    })
                    .collect::<_>(),
                location: this_block_region,
            });

            match_index += untrimmed_length;
        }
    }
    res
}

#[derive(Debug, Clone)]
pub struct LabelledHistogram {
    /// Past histogram holds a ringbuffer of a previous row of pixel value checks.
    past_histograms: VecDeque<Vec<HistogramType>>,
    histogram: Vec<HistogramType>,
    label: ColorLabel,
}
impl LabelledHistogram {
    pub fn from_u8(data: &[u8], label: ColorLabel) -> Self {
        Self {
            histogram: data.iter().map(|z| *z as HistogramType).collect(),
            label,
            past_histograms: Default::default(),
        }
    }

    /// Add a new entry to the past histograms.
    pub fn add_past(&mut self) {
        self.past_histograms
            .push_back(vec![HistogramType::default(); self.histogram.len()]);
    }

    /// Remove the effect of a past histogram by subtracting it from the current histogram.
    pub fn remove_past(&mut self) {
        if let Some(mut d) = self.past_histograms.pop_front() {
            for (i, v) in d.drain(..).enumerate() {
                self.histogram[i] -= v;
            }
        }
    }

    /// Add pixel adds this pixel to the histogram and adds this value to the current past histogram.
    fn add_pixel<P: Pixel>(&mut self, x: usize, p: P) -> bool
    where
        u8: PartialEq<<P as Pixel>::Subpixel>,
    {
        let color = self.label.0;
        let matches = color.0[0] == p.channels()[0]
            && color.0[1] == p.channels()[1]
            && color.0[2] == p.channels()[2];
        if matches {
            self.histogram[x] += 1;
            self.past_histograms.back_mut().unwrap()[x] += 1;
            return true;
        }
        false
    }
}

/// Create an iterator that generates histogram lines.
pub struct WindowHistogramIterator<'b, I: GenericImageView> {
    image: &'b I,
    y: u32,
    window_size: u32,
    histograms: Vec<LabelledHistogram>,
}

impl<'b, I: GenericImageView> WindowHistogramIterator<'b, I>
where
    u8: PartialEq<<<I as GenericImageView>::Pixel as Pixel>::Subpixel>,
{
    /// Construct a new sliding window histogram iterator, this creates the initial histogram state.
    pub fn new(
        image: &'b I,
        labels: &[ColorLabel],
        window_size: u32,
    ) -> WindowHistogramIterator<'b, I> {
        let mut histograms: Vec<LabelledHistogram> = Vec::new();
        for l in labels {
            let labelled_histogram = LabelledHistogram {
                histogram: vec![0; image.width() as usize],
                label: *l,
                past_histograms: Default::default(),
            };
            histograms.push(labelled_histogram);
        }
        // Check if the colors are unique here.
        {
            let mut color_set = std::collections::HashSet::new();
            for l in labels {
                color_set.insert(l.0);
            }
            if color_set.len() != labels.len() {
                panic!("the colors in labels may not hold duplicates");
            }
        }
        for y in 0..window_size {
            for h in histograms.iter_mut() {
                h.add_past();
            }
            for x in 0..image.width() {
                let p = image.get_pixel(x, y);
                for h in histograms.iter_mut() {
                    if h.add_pixel(x as usize, p) {
                        break;
                    }
                }
            }
        }

        WindowHistogramIterator {
            image,
            histograms,
            y: 0,
            window_size,
        }
    }

    pub fn advance(&mut self) -> bool {
        if self.y < ((self.image.height() - self.window_size) as u32) {
            for h in self.histograms.iter_mut() {
                h.remove_past();
                h.add_past();
            }

            // Then update the window
            for x in 0..self.image.width() {
                // Add the side moving into the histogram.
                let p = self.image.get_pixel(x, self.y + self.window_size);
                for h in self.histograms.iter_mut() {
                    if h.add_pixel(x as usize, p) {
                        break;
                    }
                }
            }

            self.y += 1;

            true
        } else {
            false
        }
    }
    pub fn histograms(&self) -> &[LabelledHistogram] {
        &self.histograms
    }
    /// The y coordinate in the image for this histogram.
    pub fn y(&self) -> u32 {
        self.y
    }
}

/// Function to slide a window over an image and match glyphs for each histogram thats created.
pub fn moving_windowed_histogram<'a, I: GenericImageView>(
    image: &I,
    window_size: u32,
    matcher: &'a dyn Matcher,
    labels: &[ColorLabel],
) -> Vec<Match2D<'a>>
where
    u8: PartialEq<<<I as GenericImageView>::Pixel as Pixel>::Subpixel>,
{
    let mut res_final: Vec<Match2D<'a>> = Vec::new();

    // Container for results under consideration, we check matches against overlap in this window
    // and keep the parts that are the best matches.
    let mut res_consider: VecDeque<Match2D<'a>> = VecDeque::new();
    // Once the matches here move out of the window, we move them to res itself.

    // Create our histogram iterator.
    let mut iterable = WindowHistogramIterator::new(image, labels, window_size);
    let mut more_to_come = true;
    while more_to_come {
        let y = iterable.y();
        let mut matches_2d: Vec<Match2D<'a>> = vec![];
        for labelled_histogram in iterable.histograms().iter() {
            // Find glyphs in the histogram.
            let matches = bin_glyph_matcher(&labelled_histogram, matcher);

            // Resolve the found matches and group the consecutive tokens into 2d matches.
            matches_2d.extend(match_resolver(y, window_size, &matches));
        }
        // Decide which matches are to be kept.
        decide_on_matches(matches_2d, &mut res_consider);

        // Move matches from res_consider to res_final.
        finalize_considerations(y, &mut res_consider, &mut res_final);

        // Only advance afterwards
        more_to_come = iterable.advance();
    }

    res_final.extend(res_consider.drain(..));

    res_final
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_util::test_alphabet::{
        render_standard_alphabet, render_standard_color, standard_alphabet,
    };

    fn simple_histogram_to_bin_histogram(hist: &SimpleHistogram) -> LabelledHistogram {
        LabelledHistogram::from_u8(hist, (Rgb([255, 255, 255]), 0))
    }

    #[test]
    fn test_histogram_glyph_matcher() {
        let rgb_image = render_standard_alphabet();
        let image = &rgb_image;
        // let image = image_support::rgb_image_to_view(&rgb_image);
        let hist = image_to_simple_histogram(image, Rgb::<u8>([255, 255, 255]));
        let mut glyph_set = image_support::dev_image_to_glyph_set(
            &rgb_image,
            Some(0),
            &vec![Rgb::<u8>([255, 255, 255])],
            &None,
        );
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
        let image = &rgb_image;
        let hist = image_to_simple_histogram(image, Rgb::<u8>([255, 255, 255]));
        let mut glyph_set = image_support::dev_image_to_glyph_set(
            &rgb_image,
            Some(0),
            &vec![Rgb::<u8>([255, 255, 255])],
            &None,
        );
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
        if !have_dir {
            println!(
                "Directory {} does not exist, create it to write files for inspection.",
                &location
            );
        }

        use image::RgbImage;
        use std::path::PathBuf;

        // Create the glyph set.
        let (glyph_image, glyph_text) = standard_alphabet();
        let mut glyph_set = image_support::dev_image_to_glyph_set(
            &glyph_image,
            Some(0),
            &vec![Rgb::<u8>([255, 255, 255])],
            &None,
        );
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

        let white = Rgb::<u8>([255, 255, 255]);
        let red = Rgb::<u8>([255, 0, 0]);
        let blue = Rgb::<u8>([0, 0, 255]);

        let locations = [
            (10u32, 10u32, "caab", red),
            (50, 13, "deeb", white),
            (100u32, 10u32, "waab", blue),
            (150, 13, "wacb", white),
            (50, 20, "dwaaaaaa", red), // Touches the other DEEB.
            (10u32, 50u32, "cba", blue),
        ];

        for (x, y, text, color) in locations.iter() {
            render_standard_color(&mut image, *x, *y, text, *color);
        }

        if have_dir {
            let _ = image.save(location.to_owned() + "input_image.png").unwrap();
        }

        let image = &image;
        let labels = vec![(white, 0), (red, 1), (blue, 2)];

        let matches = moving_windowed_histogram(image, glyph_set.line_height, &matcher, &labels);

        if have_dir {
            util::write_match_html(
                image.width(),
                image.height(),
                &matches,
                &labels,
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

        let matches_2d = match_resolver(0, glyph_set.line_height, &matches);

        decide_on_matches(matches_2d, &mut res_consider);
        assert_eq!(res_consider.len(), 6);
        println!("res_consider: {res_consider:?}");
    }

    #[test]
    #[allow(unreachable_code)]
    fn histogram_matching_real_longest_char_not_best() {
        return;
        let input: Vec<u8> = vec![
            0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 0, 5, 2, 2, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0,
        ];
        // The real matches.
        let s1: Vec<u8> = vec![0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0];
        let s2: Vec<u8> = vec![0, 0, 0, 5, 2, 2, 2, 2, 2, 2, 2, 0, 0, 0];
        // And then the character that ruins it all.
        let s3: Vec<u8> = vec![0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0];

        let mut glyph_set: glyphs::GlyphSet = Default::default();
        glyph_set.entries.push(glyphs::Glyph::new(&s1, &"s1"));
        glyph_set.entries.push(glyphs::Glyph::new(&s2, &"s2"));
        glyph_set.entries.push(glyphs::Glyph::new(&s3, &"s3"));
        glyph_set.line_height = 1;
        glyph_set.prepare();
        let matcher = matcher::LongestGlyphMatcher::new(&glyph_set.entries);

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
    }

    #[test]
    fn matching_with_space_character() {
        // Some dummy glyphs
        let s1: Vec<u8> = vec![0, 0, 13, 13, 1, 1, 3, 4, 5, 3, 0];
        let s2: Vec<u8> = vec![0, 5, 3, 2, 3, 2, 2, 2, 3, 2, 4, 0, 0];
        let space: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0];

        let mut glyph_set: glyphs::GlyphSet = Default::default();
        glyph_set.entries.push(glyphs::Glyph::new(&s1, &"s1"));
        glyph_set.entries.push(glyphs::Glyph::new(&s2, &"s2"));
        let mut space_glyph = glyphs::Glyph::new(&space, &" ");

        space_glyph.set_ignore_on_lstrip(false);
        space_glyph.set_max_consecutive(Some(1));
        space_glyph.set_trim_left(true);
        space_glyph.set_trim_right(true);
        glyph_set.entries.push(space_glyph);

        glyph_set.line_height = 1;
        glyph_set.prepare();
        let matcher = matcher::LongestGlyphMatcher::new(&glyph_set.entries);
        println!("Glyph set: {glyph_set:?}");

        let mut input: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        input.extend(s1.clone());
        input.extend(s2.clone());
        input.extend(space.clone());
        input.extend(s2.clone());
        input.extend(s1.clone());
        input.extend(space.clone());
        input.extend(space.clone());
        input.extend(s2.clone());
        input.extend(s1.clone());
        input.extend(s1.clone());
        input.extend(s2.clone());
        input.extend(space.clone()); // space here
        input.extend(vec![0, 0, 0]); // non-space whitespace
        input.extend(s2.clone());
        input.extend(s1.clone());
        input.extend(vec![0, 0, 0]);

        // Current outcome
        // 's1s2 s2s1  s2s1s1s2   |s2s1'

        // Desired outcome, single space is allowed, more than one causes split and strip;
        // 's1s2 s2s1'
        // 's2s1s1s2 '
        // 's2s1'

        let binned = simple_histogram_to_bin_histogram(&input);
        let matches = bin_glyph_matcher(&binned, &matcher);
        let matches_2d = match_resolver(0, glyph_set.line_height, &matches);
        for m in matches_2d.iter() {
            println!("{m:?} ->  '{}'", m.to_string());
        }
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
            let mut glyph_set = image_support::dev_image_to_glyph_set(
                &original_image,
                Some(0),
                &vec![Rgb::<u8>([255, 255, 255])],
                &None,
            );
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
        let glyph_set = image_support::dev_image_to_glyph_set(
            &image,
            Some(0),
            &vec![Rgb::<u8>([255, 255, 255])],
            &output_dir,
        );

        let glyph_set_output = location.clone() + "dev_histogram_boxes.png";
        let image = open(&glyph_set_output)
            .expect(&format!("could not load image at {:?}", &glyph_set_output))
            .to_rgb8();
        let mut image_mut = image.clone();

        let matcher = matcher::LongestGlyphMatcher::new(&glyph_set.entries);
        let image = &scaled_readme_glyphs;
        let labels = vec![(Rgb::<u8>([255, 255, 255]), 0)];

        let matches = moving_windowed_histogram(image, glyph_set.line_height, &matcher, &labels);

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
