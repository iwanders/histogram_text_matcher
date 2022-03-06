// This is really nice, should adhere to this;
// https://rust-lang.github.io/api-guidelines/naming.html

// Should consider https://rust-lang.github.io/rust-clippy/rust-1.59.0/index.html#shadow_same

pub mod glyphs;

mod interface;
pub use interface::*;

/// Type to hold a simple 1D histogram.
pub type SimpleHistogram = Vec<u8>;

// This here ensures that we have image support when the feature is enabled, but also for all tests.
#[cfg(feature = "image_support")]
pub mod image_support;
#[cfg(test)]
pub mod image_support;

/*
    Improvements:
        - Currently, a space character would be greedy and match until end of line.
          We need something to limit this from occuring more than 'n' times in a row and
          perform strip on the glyph sequence afterwards.
        - Token matcher can't split based on the labels, because the histogram may have labels
          in between glyphs that weren't colored appropriately. Search for CONSIDER_MATCH_LABEL.
*/

/// Function to match a single color in an image and convert this to a histogram.
fn image_to_simple_histogram(image: &dyn Image, color: RGB) -> SimpleHistogram {
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

fn histogram_glyph_matcher(
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

fn simple_histogram_to_bin_histogram(hist: &SimpleHistogram) -> Vec<Bin> {
    hist.iter()
        .map(|x| Bin {
            count: *x as u32,
            label: 0,
        })
        .collect::<_>()
}

fn bin_histogram_to_simple_histogram(hist: &[Bin]) -> SimpleHistogram {
    hist.iter().map(|x| x.count as u8).collect::<_>()
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
struct Bin {
    count: u32,
    label: usize,
}

type ColorLabel = (RGB, usize);

#[derive(Debug, Clone, PartialEq, Eq)]
struct LabelledGlyph<'a> {
    glyph: &'a glyphs::Glyph,
    label: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token<'a> {
    WhiteSpace(usize), // Value denotes amount of whitespace pixels.
    Glyph {
        glyph: &'a glyphs::Glyph,
        label: usize,
        error: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Match<'a> {
    token: Token<'a>,
    position: u32,
    width: u32,
}

fn print_match_slice<'a>(glyphs: &[Match<'a>]) {
    for t in glyphs.iter() {
        match &t.token {
            Token::WhiteSpace(w) => print!(" w{w}"),
            Token::Glyph {
                glyph,
                label,
                error,
            } => {
                let s = glyph.glyph();
                print!(" {}#{label}", s)
            }
        }
    }
}

fn bin_glyph_matcher<'a>(histogram: &[Bin], set: &'a glyphs::GlyphSet) -> Vec<Match<'a>> {
    let mut i: usize = 0; // index into the histogram.
    let mut res: Vec<Match<'a>> = Vec::new();

    fn calc_score(pattern: &[u8], to_match: &[Bin]) -> u32 {
        let mut res: u32 = 0;
        for (x_a, b) in (0..pattern.len()).zip(to_match.iter()) {
            let a = if x_a < pattern.len() {
                pattern[x_a]
            } else {
                0u8
            };
            let a = a as u32;
            res += if a > b.count {
                a - b.count
            } else {
                b.count - a
            };
        }
        res
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

        // We got a non zero entry in the first bin now.
        let remainder = &histogram[i..];

        // CONSIDER: Splitting the histogram by labels at the start, then match on the labels.
        // Next, make sure we only match the label found in the first bin.
        // This is problematic... Since space between letters may not have the label set.
        // Disabled this for now. See CONSIDER_MATCH_LABEL.
        // let max_index = remainder.iter().position(|x| x.label != remainder[0].label);
        // let remainder = &histogram[i..i + max_index.unwrap_or(remainder.len())];

        // Let us check all the glyphs and determine which one has the lowest score.
        type ScoreType = u32;
        let mut scores: Vec<ScoreType> = vec![];
        scores.resize(set.entries.len(), 0 as ScoreType);
        for (glyph, score) in set.entries.iter().zip(scores.iter_mut()) {
            *score = calc_score(
                if use_stripped {
                    glyph.lstrip_hist()
                } else {
                    glyph.hist()
                },
                remainder,
            );
        }

        // https://stackoverflow.com/a/53908709
        let index_of_min: Option<usize> = scores
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(index, _)| index);

        if let Some(best) = index_of_min {
            let found_glyph = &set.entries[best];
            let score = scores[best];
            // println!("#{i} score: {score} -> {found_glyph:?}");

            if score == 0 {
                // Calculate the true position, depends on whether we used stripped values.
                let first_non_zero = found_glyph.hist().len() - found_glyph.lstrip_hist().len();
                let position = i as u32 - if use_stripped { first_non_zero } else { 0 } as u32;
                // Position where histogram where this letter has the first non-zero;
                let label_position = position as usize + first_non_zero;

                // Add the newly detected glyph
                res.push(Match {
                    position: position,
                    token: Token::Glyph {
                        glyph: found_glyph,
                        error: score as u32,
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
            } else {
                i += 1;
            }

            // Only use stripped if we didn't get a perfect match.
            use_stripped = score != 0;
        } else {
            panic!("Somehow didn't get a lowest score... did we have glyphs?");
        }
    }
    res
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn overlaps(&self, b: &Rect) -> bool {
        self.right() >= b.left()
            && b.right() >= self.left()
            && self.top() >= b.bottom()
            && b.top() >= self.bottom()
    }

    pub fn top(&self) -> u32 {
        self.y + self.h
    }
    pub fn bottom(&self) -> u32 {
        self.y
    }

    pub fn left(&self) -> u32 {
        self.x
    }
    pub fn right(&self) -> u32 {
        self.x + self.w
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match2D<'a> {
    tokens: Vec<LabelledGlyph<'a>>,
    location: Rect,
}

use std::collections::VecDeque;

// Helper to add a row
fn add_pixel(x: usize, p: &RGB, labels: &[ColorLabel], histogram: &mut Vec<Bin>) {
    for (color, index) in labels.iter() {
        if color == p {
            histogram[x].count += 1;
            histogram[x].label = *index;
            return;
        }
    }
}

// Helper to subtract a row.
fn sub_pixel(x: usize, p: &RGB, labels: &[ColorLabel], histogram: &mut Vec<Bin>) {
    for (color, index) in labels.iter() {
        if color == p {
            histogram[x].count = histogram[x].count.saturating_sub(1);
            return;
        }
    }
}

// Helper to accept or discard matches to consider
fn finalize_considerations<'a>(
    y: u32,
    res_consider: &mut VecDeque<Match2D<'a>>,
    res_final: &mut Vec<Match2D<'a>>,
) {
    while !res_consider.is_empty() && res_consider.front().unwrap().location.top() < y {
        res_final.push(res_consider.pop_front().unwrap());
    }
}

// Helper to decide on matches.
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
        if let Token::WhiteSpace(_) = matches[match_index].token {
            match_index += 1;
            continue;
        }

        if let Token::Glyph {
            glyph,
            label,
            error,
        } = matches[match_index].token
        {
            // Find the index where this consecutive glyph block ends.
            let block_end = matches[match_index..]
                .iter()
                .position(|z| {
                    std::mem::discriminant(&z.token)
                        == std::mem::discriminant(&Token::WhiteSpace(0))
                })
                .unwrap_or(matches.len());
            // println!("block_end  {block_end}");
            let glyphs = &matches[match_index..match_index + block_end];
            let first_glyph = glyphs.first().expect("never empty");
            let last_glyph = glyphs.last().expect("never empty");

            // print!("y: {y} -> ");
            // print_match_slice(glyphs);
            // println!();

            let block_width = last_glyph.position + last_glyph.width - first_glyph.position;
            let this_block_region = Rect {
                x: first_glyph.position,
                y,
                w: block_width,
                h: window_size,
            };

            // Check if this overlaps with others in the consideration buffer;
            let mut do_insert = true;
            *res_consider = res_consider
                .drain(..)
                .filter(|m| {
                    if m.location.overlaps(&this_block_region) {
                        if glyphs.len() < m.tokens.len() {
                            do_insert = false; // already existing overlap is better.
                        } else {
                            return false; // yes, this is an upgrade, drop from res_consider
                        }
                    }
                    return true;
                })
                .collect::<_>();

            if do_insert {
                // we pruned boxes, so the new one must be better.
                res_consider.push_back(Match2D {
                    tokens: glyphs
                        .iter()
                        .map(|z| match z.token {
                            Token::Glyph {
                                glyph,
                                label,
                                error,
                            } => LabelledGlyph { glyph, label },
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

pub fn moving_windowed_histogram<'a>(
    image: &dyn Image,
    set: &'a glyphs::GlyphSet,
    labels: &[ColorLabel],
) -> Vec<Match2D<'a>> {
    let mut res_final: Vec<Match2D<'a>> = Vec::new();

    // Container for results under consideration, we check matches against overlap in this window
    // and keep the parts that are the best matches.
    let mut res_consider: VecDeque<Match2D<'a>> = VecDeque::new();
    // Once the matches here move out of the window, we move them to res itself.

    let mut histogram: Vec<Bin> = Vec::<Bin>::new();
    histogram.resize(image.width() as usize, Default::default());
    let window_size = set.line_height as u32;

    // Start at the top, with zero width, then we sum rows for the window size
    // Then, we iterate down, at the bottom of the window add to the histogram
    // and the top row that moves out we subtract.

    // Let us first, setup the first histogram, this is from 0 to window size.
    for y in 0..window_size {
        for x in 0..image.width() {
            let p = image.pixel(x, y);
            add_pixel(x as usize, &p, labels, &mut histogram);
        }
    }

    for y in 0..((image.height() - window_size) as u32) {
        // Here, we match the current histogram, and store matches.

        // println!("{histogram:?}");
        // let simple_hist = bin_histogram_to_simple_histogram(&histogram);
        // println!("y: {y} -> {simple_hist:?}");

        // Find glyphs in the histogram.
        let matches = bin_glyph_matcher(&histogram, &set);

        // Decide which matches are to be kept.
        decide_on_matches(y, window_size, &matches, &mut res_consider);

        // Move matches from res_consider to res_final.
        finalize_considerations(y, &mut res_consider, &mut res_final);

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
    }

    res_final.extend(res_consider.drain(..));

    res_final
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_histogram_glyph_matcher() {
        assert!(true);
        let rgb_image = image_support::dev_create_example_glyphs().expect("Succeeds");
        let image = image_support::rgb_image_to_view(&rgb_image);
        let hist = image_to_simple_histogram(&image, RGB::white());
        let mut glyph_set = image_support::dev_image_to_glyph_set(&rgb_image, Some(0), &None);
        glyph_set.prepare();

        println!("Histogram: {hist:?}");

        let res = histogram_glyph_matcher(&hist, &glyph_set, 10);

        assert!(res.len() == 4);
        for (g, score) in res.iter() {
            println!("{g:?}: {score}");
            assert!(*score == 0);
        }
    }

    #[test]
    fn test_bin_glyph_matcher() {
        assert!(true);
        let rgb_image = image_support::dev_create_example_glyphs().expect("Succeeds");
        let image = image_support::rgb_image_to_view(&rgb_image);
        let hist = image_to_simple_histogram(&image, RGB::white());
        let mut glyph_set = image_support::dev_image_to_glyph_set(&rgb_image, Some(0), &None);
        glyph_set.prepare();

        let binned = simple_histogram_to_bin_histogram(&hist);

        let matches = bin_glyph_matcher(&binned, &glyph_set);
        // println!("Histogram: {matches:?}");

        for (i, v) in matches.iter().enumerate() {
            println!("{i}: {v:?}");
        }
    }

    #[test]
    fn test_bin_glyph_matcher_no_white_space() {
        println!();

        use std::path::Path;

        let rgb_image = image_support::dev_create_example_glyphs().expect("Succeeds");
        let mut glyph_set = image_support::dev_image_to_glyph_set(&rgb_image, Some(0), &None);
        glyph_set.prepare();

        let image = image_support::dev_create_example_glyphs_packed().expect("Must have image");
        let _ = image
            .save(Path::new("dev_glyph_matcher_no_white_space.png"))
            .unwrap();

        let _ = image_support::filter_white(&image)
            .save(Path::new("dev_glyph_matcher_no_white_space_white.png"))
            .unwrap();

        let image = image_support::rgb_image_to_view(&image);
        let hist = image_to_simple_histogram(&image, RGB::white());
        println!("hist: {hist:?}");
        let binned = simple_histogram_to_bin_histogram(&hist);

        let matches = bin_glyph_matcher(&binned, &glyph_set);
        // println!("Histogram: {matches:?}");

        let mut glyph_counter = 0;
        for (i, v) in matches.iter().enumerate() {
            println!("{i}: {v:?}");
            if let Token::Glyph {
                glyph,
                label,
                error,
            } = v.token
            {
                assert!(*glyph == glyph_set.entries[glyph_counter]);
                assert!(error == 0);
                glyph_counter += 1;
            }
        }
    }
    #[test]
    fn test_bin_glyph_matcher_no_white_space_moving() {
        println!();
        let rgb_image = image_support::dev_create_example_glyphs().expect("Succeeds");
        let mut glyph_set = image_support::dev_image_to_glyph_set(&rgb_image, Some(0), &None);
        glyph_set.prepare();
        println!("glyph_set: {glyph_set:?}");

        let image = image_support::dev_create_example_glyphs_packed().expect("Must have image");
        let image = image_support::rgb_image_to_view(&image);
        let labels = vec![(RGB::white(), 0)];

        let matches = moving_windowed_histogram(&image, &glyph_set, &labels);
        for m in matches.iter() {
            let location = &m.location;
            print!("{location:?} -> ");
            for t in m.tokens.iter() {
                let g = t.glyph.glyph();
                print!(" {g:?}");
            }
            println!();
        }
    }
    #[test]
    fn test_bin_glyph_matcher_no_white_space_moving_multiple() {
        println!();

        let rgb_image = image_support::dev_create_example_glyphs().expect("Succeeds");
        let mut glyph_set = image_support::dev_image_to_glyph_set(&rgb_image, Some(0), &None);
        glyph_set.prepare();

        use image::Rgb;
        use rusttype::{Font, Scale};
        use std::path::Path;

        let font_size = 40.0;
        let font = std::fs::read("/usr/share/fonts/truetype/ttf-bitstream-vera/Vera.ttf").unwrap();
        let font = Font::try_from_vec(font).unwrap();

        let size = (500, 500);

        let mut drawables =
            image_support::dev_example_glyphs_packed(0, 0, &Rgb([255u8, 255u8, 255u8]));
        drawables.extend(image_support::dev_example_glyphs_packed(
            100,
            15,
            &Rgb([255u8, 0u8, 0u8]),
        ));

        let image = image_support::render_font_image(size, &font, font_size, &drawables);
        let _ = image
            .save(Path::new(
                "dev_glyph_matcher_no_white_space_moving_multiple.png",
            ))
            .unwrap();
        let _ = image_support::filter_primary(&image)
            .save(Path::new(
                "dev_glyph_matcher_no_white_space_moving_multiple_primary.png",
            ))
            .unwrap();

        let image = image_support::rgb_image_to_view(&image);
        let labels = vec![(RGB::white(), 0), (RGB::red(), 1)];

        let matches = moving_windowed_histogram(&image, &glyph_set, &labels);
        for m in matches.iter() {
            let location = &m.location;
            print!("{location:?} -> ");
            for t in m.tokens.iter() {
                let l = t.label;
                let g = t.glyph.glyph();
                print!(" {g:?}#{l}");
            }
            println!();
        }
    }
}
