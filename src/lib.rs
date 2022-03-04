// use image::imageops::colorops::grayscale;
// use image::{open, GenericImage, GenericImageView, Rgb, RgbImage};
// use imageproc::map::map_colors;
// use imageproc::rect::Rect;
// use imageproc::rect::Region;

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
    set: &glyphs::GlyphSet
) {
    let v = histogram;
    let mut i: usize = 0;
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
            *score = calc_score_min(&glyph.hist, &remainder, 10);
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
            println!("Found {best}, with {score} -> {token:?}");

            i += token.hist.len();
        } else {
            println!("Huh? didn't have a lowest score??");
            i += 1;
        }
    }
}


#[derive(Debug, Clone, Default)]
struct Bin
{
    count: u32,
    label: usize,
}

type ColorLabel = (RGB, usize);

fn moving_windowed_histogram(
    image: &dyn Image,
    set: &glyphs::GlyphSet,
    labels: &[ColorLabel]) {


    let mut histogram: Vec<Bin> = Vec::<Bin>::new();
    histogram.resize(image.width() as usize, Default::default());
    let window_size = set.line_height as u32;

    // Start at the top, with zero width, then we sum rows for the window size
    // Then, we iterate down, at the bottom of the window add to the histogram
    // and the top row that moves out we subtract.

    fn add_pixel(x: usize, p: &RGB, labels: &[ColorLabel], histogram: &mut Vec<Bin>) {
        for (color, index) in labels.iter()
        {
            if color == p
            {
                histogram[x].count += 1;
                histogram[x].label = *index;
                return;
            }
        }
    }

    fn sub_pixel(x: usize, p: &RGB, labels: &[ColorLabel], histogram: &mut Vec<Bin>) {
        for (ref color, index) in labels.iter()
        {
            if color == p
            {
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
    fn test_key_lookup() {
        assert!(true);
        let rgb_image = image_support::dev_create_example_glyphs().expect("Succeeds");
        let image = image_support::rgb_image_to_view(&rgb_image);
        let hist = image_to_simple_histogram(&image, RGB::white());
        let glyph_set = image_support::dev_image_to_glyph_set(&rgb_image, Some(0));
        let trimmed_set = glyphs::strip_glyph_set(&glyph_set);

        println!("Histogram: {hist:?}");

        let res = histogram_glyph_matcher(&hist, &trimmed_set);
    }
}

