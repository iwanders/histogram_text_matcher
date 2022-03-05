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

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
struct Bin {
    count: u32,
    label: usize,
}

type ColorLabel = (RGB, usize);

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
                let mut last = res.last_mut();
                if last.is_some()
                    && std::mem::discriminant(&last.as_ref().unwrap().token)
                        == std::mem::discriminant(&Token::WhiteSpace(0))
                {
                    if let Token::WhiteSpace(ref mut z) = last.unwrap().token {
                        *z += 1;
                    }
                } else {
                    res.push(Match {
                        token: Token::WhiteSpace(1),
                        position: i as u32,
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
        let max_index = remainder.iter().position(|x| x.label != remainder[0].label);
        let remainder = &histogram[i..i + max_index.unwrap_or(remainder.len())];

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
                res.push(Match {
                    position: i as u32,
                    token: Token::Glyph {
                        glyph: found_glyph,
                        error: score as u32,
                        label: remainder[0].label,
                    },
                });

                // Advance the cursor by the width of the glyph we just matched.
                i += if use_stripped {
                    found_glyph.lstrip_hist().len()
                } else {
                    found_glyph.hist().len()
                };
            }
            else
            {
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

fn moving_windowed_histogram(image: &dyn Image, set: &glyphs::GlyphSet, labels: &[ColorLabel]) {
    let mut histogram: Vec<Bin> = Vec::<Bin>::new();
    histogram.resize(image.width() as usize, Default::default());
    let window_size = set.line_height as u32;

    // Start at the top, with zero width, then we sum rows for the window size
    // Then, we iterate down, at the bottom of the window add to the histogram
    // and the top row that moves out we subtract.

    fn add_pixel(x: usize, p: &RGB, labels: &[ColorLabel], histogram: &mut Vec<Bin>) {
        for (color, index) in labels.iter() {
            if color == p {
                histogram[x].count += 1;
                histogram[x].label = *index;
                return;
            }
        }
    }

    fn sub_pixel(x: usize, p: &RGB, labels: &[ColorLabel], histogram: &mut Vec<Bin>) {
        for (ref color, index) in labels.iter() {
            if color == p {
                histogram[x].count = histogram[x].count.saturating_sub(1);
                return;
            }
        }
    }

    // Let us first, setup the first histogram, this is from 0 to window size.
    for y in 0..window_size {
        for x in 0..image.width() {
            let p = image.pixel(x, y);
            add_pixel(x as usize, &p, labels, &mut histogram);
        }
    }

    for y in 1..((image.height() - window_size) as u32) {
        // Here, we match the current histogram, and store matches.
        // token_binned_histogram_matcher(y, &single_hist, &map, &histmap_reduced, &mut image_mutable);

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
        let mut glyph_set = image_support::dev_image_to_glyph_set(&rgb_image, Some(0));
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
        let mut glyph_set = image_support::dev_image_to_glyph_set(&rgb_image, Some(0));
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
        use image::Rgb;
        use rusttype::{Font, Scale};
        use std::path::Path;

        let rgb_image = image_support::dev_create_example_glyphs().expect("Succeeds");
        let mut glyph_set = image_support::dev_image_to_glyph_set(&rgb_image, Some(0));
        glyph_set.prepare();

        // Create an image without spaces.
        let font_size = 40.0;

        let font =
            std::fs::read("/usr/share/fonts/truetype/ttf-bitstream-vera/Vera.ttf").expect("Works");
        let font = Font::try_from_vec(font).unwrap();

        let size = (
            (font_size * (4 + 1) as f32) as u32,
            (font_size * 2.0) as u32,
        );

        let mut drawables: Vec<((u32, u32), String, Rgb<u8>)> = Vec::new();
        drawables.push(((20, 20), String::from("a"), Rgb([255u8, 255u8, 255u8])));
        drawables.push(((35, 20), String::from("b"), Rgb([255u8, 255u8, 255u8])));
        drawables.push(((54, 20), String::from("e"), Rgb([255u8, 255u8, 255u8])));
        drawables.push(((73, 20), String::from("z"), Rgb([255u8, 255u8, 255u8])));

        let image = image_support::render_font_image(size, &font, font_size, &drawables);
        let _ = image
            .save(Path::new("dev_glyph_matcher_no_white_space.png"))
            .unwrap();

        let _ = image_support::filter_white(&image)
            .save(Path::new("dev_glyph_matcher_no_white_space_white.png"))
            .unwrap();

        let image = image_support::rgb_image_to_view(&image);
        let hist = image_to_simple_histogram(&image, RGB::white());
        let binned = simple_histogram_to_bin_histogram(&hist);

        let matches = bin_glyph_matcher(&binned, &glyph_set);
        // println!("Histogram: {matches:?}");

        let mut glyph_counter = 0;
        for (i, v) in matches.iter().enumerate() {
            println!("{i}: {v:?}");
            if let Token::Glyph{glyph, label, error} = v.token
            {
                assert!(*glyph == glyph_set.entries[glyph_counter]);
                assert!(error == 0);
                glyph_counter += 1;
            }
        }
    }
}
