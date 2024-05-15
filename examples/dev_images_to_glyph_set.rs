use histogram_text_matcher::glyphs::{Glyph, GlyphSet};
use histogram_text_matcher::image_support::image_to_histogram;
use histogram_text_matcher::{Rect, RGB};
use image::open;
use image::{GenericImageView, Rgb, RgbImage};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
struct AnnotatedImage {
    file_path: String,
    roi: Rect,
    text: String,
    color: (u8, u8, u8),
}

#[derive(Deserialize, Debug)]
struct Collection {
    base_dir: Option<String>,
    images: Vec<AnnotatedImage>,
}

fn load_collection(input_path: &Path) -> Result<Collection, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Read;
    let mut file = File::open(input_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let extension = input_path.extension().expect("should have an extension");

    let mut p: Collection;
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
    Ok(p)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = std::env::args()
        .nth(1)
        .expect("No collection set file specified.");
    let collection = load_collection(&PathBuf::from(file_path))?;

    println!("c: {collection:#?}");
    let mut glyph_set: GlyphSet = Default::default();
    let mut tallest = 0;

    let mut text_histogram = vec![];

    for img in collection.images.iter() {
        let final_path = if let Some(v) = collection.base_dir.as_ref() {
            let mut z = PathBuf::from(v);
            z.push(img.file_path.clone());
            z
        } else {
            PathBuf::from(img.file_path.clone())
        };

        let name = final_path
            .file_stem()
            .unwrap()
            .to_str()
            .map(|z| z.to_owned())
            .unwrap();

        let image = open(&final_path)
            .expect(&format!("could not load image at {:?}", final_path))
            .to_rgb8();

        // We got the image, now slice the roi.
        let roi_img = image
            .view(img.roi.x, img.roi.y, img.roi.w, img.roi.h)
            .to_image();
        roi_img.save(format!("/tmp/{name}_roi.png"))?;

        // Next, we filter the image to mask it with the color of interest.
        let mut masked_img = roi_img.clone();
        for p in masked_img.pixels_mut() {
            if *p == Rgb([img.color.0, img.color.1, img.color.2]) {
                *p = Rgb([255u8, 255u8, 255u8]);
            } else {
                *p = Rgb([0u8, 0u8, 0u8]);
            }
        }
        masked_img.save(format!("/tmp/{name}_masked.png"))?;

        let sub_img_gray = image::DynamicImage::ImageRgb8(masked_img).into_luma8();
        let histogram = image_to_histogram(&sub_img_gray);
        sub_img_gray.save(format!("/tmp/{name}_gray.png"))?;
        tallest = tallest.max(*histogram.iter().max().unwrap());
        println!("Histogram: {histogram:?}");

        // Now... we need text + histogram -> glyph :/
        // Let us start by pruning the left zeros.

        text_histogram.push((img.text.clone(), histogram, name));

        // glyph_set.entries.push(Glyph::new(&sub_img_histogram, &format!("{c}")));
    }

    fn splitter(hist: &[u8]) -> Vec<(usize, usize)> {
        let mut v = vec![];
        let mut s = None;
        // skip peeking to ensure max step for now.
        for (i, a) in hist.iter().enumerate() {
            if *a == 0 && s.is_some() {
                v.push((s.take().unwrap(), i));
            }
            if *a != 0 && s.is_none() {
                s = Some(i);
            }
        }
        v
    }
    // Now, we need to do smarts with the matches... slice them, then add them to the glyph set.
    #[derive(Default, Debug)]
    struct AnalysedGlyph {
        stripped_hist: Vec<u8>,
        left_dist: Option<(usize, char)>,
        right_dist: Option<(usize, char)>,
        name: String,
    }
    let mut s: std::collections::HashMap<char, Vec<AnalysedGlyph>> = Default::default();
    for (text, histogram, name) in text_histogram {
        let intervals = splitter(&histogram);
        println!("histogram: {histogram:?}");
        println!("intervals: {intervals:?}");
        let mut interval_pos = 0;
        let chars = text.chars().collect::<Vec<char>>();
        for (ci, c) in chars.iter().enumerate() {
            let mut v = s.entry(*c).or_default();

            // We can always populate this.
            let mut ag = AnalysedGlyph {
                name: name.clone(),
                stripped_hist: vec![],
                ..Default::default()
            };
            let interval = &intervals[interval_pos];
            if interval_pos != 0 {
                ag.left_dist = Some((interval.0 - intervals[interval_pos - 1].1, chars[ci - 1]));
            }
            if (interval_pos + 1) < intervals.len() {
                ag.right_dist = Some((intervals[interval_pos + 1].0 - interval.1, chars[ci + 1]));
            }
            if *c == ' ' {
                // Space will not be in the intervals, but we do know its size will be between
                // the interval left of current and right of current.
            } else {
                ag.stripped_hist = histogram[interval.0..interval.1].to_vec();
                interval_pos += 1;
            }
            v.push(ag);
        }
    }
    for (c, entries) in s.iter() {
        println!("c: {c:?}");
        for entry in entries.iter() {
            println!(
                "  {} {:?} {:?} {:?}",
                entry.name, entry.stripped_hist, entry.left_dist, entry.right_dist
            );
        }
    }

    glyph_set.line_height = tallest as u32;
    glyph_set.prepare();

    histogram_text_matcher::glyphs::write_glyph_set(
        &Path::new("/tmp/").join("glyph_set.json"),
        &glyph_set,
    )
    .expect("writing should succeed");

    histogram_text_matcher::glyphs::write_glyph_set(
        &Path::new("/tmp/").join("glyph_set.yaml"),
        &glyph_set,
    )
    .expect("writing should succeed");

    Ok(())
}
