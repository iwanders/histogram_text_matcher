use histogram_text_matcher::glyphs::{Glyph, GlyphSet};
use histogram_text_matcher::image_support::image_to_histogram;
use histogram_text_matcher::match_histogram_to_string;
use histogram_text_matcher::Rect;

// use histogram_text_matcher::histogram_glyph_matcher;
// use histogram_text_matcher::Token;

// use histogram_text_matcher::Bin;
use image::open;
use image::{GenericImageView, Rgb};
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

    #[serde(default)]
    histogram_add_zero_start_end: bool,

    #[serde(default)]
    drop_space: bool,
    // If letters fall apart into two intervals... we can express for each char how many intervals it spans here.
    #[serde(default)]
    space_min_size: Option<u8>,

    #[serde(default)]
    char_intervals: std::collections::HashMap<char, usize>,
}

fn load_collection(input_path: &Path) -> Result<Collection, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Read;
    let mut file = File::open(input_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let extension = input_path.extension().expect("should have an extension");

    let p: Collection;
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
    let mut text_histogram = vec![];

    let mut tallest = 0;

    for (i, img) in collection.images.iter().enumerate() {
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
        roi_img.save(format!("/tmp/{name}_{i}_roi.png"))?;

        // Next, we filter the image to mask it with the color of interest.
        let mut masked_img = roi_img.clone();
        let mut lowest_y = 100000000;
        let mut heighest_y = 0;
        for (_x, y, p) in masked_img.enumerate_pixels_mut() {
            if *p == Rgb([img.color.0, img.color.1, img.color.2]) {
                *p = Rgb([255u8, 255u8, 255u8]);
                lowest_y = lowest_y.min(y);
                heighest_y = heighest_y.max(y);
            } else {
                *p = Rgb([0u8, 0u8, 0u8]);
            }
        }
        tallest = heighest_y - lowest_y + 1;
        masked_img.save(format!("/tmp/{name}_{i}_masked.png"))?;

        let sub_img_gray = image::DynamicImage::ImageRgb8(masked_img).into_luma8();

        let histogram = image_to_histogram(&sub_img_gray);
        sub_img_gray.save(format!("/tmp/{name}_{i}_gray.png"))?;
        // tallest = tallest.max(*histogram.iter().max().unwrap());
        // println!("Histogram: {histogram:?}");

        // Now... we need text + histogram -> glyph :/
        // Let us start by pruning the left zeros.
        println!("img text: {}   name: {name}", img.text);
        let text = if collection.drop_space {
            img.text.clone().replace(" ", "")
        } else {
            img.text.clone()
        };
        text_histogram.push((text, histogram, name));
    }

    fn splitter(
        hist: &[u8],
        chars: &[char],
        intervals: &std::collections::HashMap<char, usize>,
    ) -> Vec<(usize, usize)> {
        let mut v = vec![];
        let mut letter_start = None;
        let mut s = None;
        let mut ci = 0;
        // skip peeking to ensure max step for now.
        let mut intervals_this_letter = intervals.get(&chars[ci]).cloned().unwrap_or(1);
        // println!("intervals_this_letter: {intervals_this_letter:?} for {:?}", chars[ci]);
        for (i, a) in hist.iter().enumerate() {
            // println!("i: {i} hist val: {a}, s: {s:?}");
            if *a == 0 && s.is_some() {
                intervals_this_letter -= 1;
                if intervals_this_letter == 0 {
                    v.push((letter_start.take().unwrap(), i));
                    ci += 1;
                    s = None;

                    if ci < chars.len() {
                        intervals_this_letter = intervals.get(&chars[ci]).cloned().unwrap_or(1);
                    }
                    // println!("intervals_this_letter: {intervals_this_letter:?} for {:?}", chars[ci]);
                } else {
                    s = None;
                    // println!("Keepings: {letter_start:?} for {:?} at {i} with a {a}", chars[ci]);
                }
            }
            if *a != 0 && s.is_none() {
                s = Some(i);
                if letter_start.is_none() {
                    letter_start = Some(i);
                }
            }
        }
        v
    }
    // Now, we need to do smarts with the matches... slice them, then add them to the glyph set.
    #[derive(Default, Debug, Ord, Eq, PartialEq, PartialOrd)]
    struct AnalysedGlyph {
        stripped_hist: Vec<u8>,
        name: String,
    }

    // For the white space / bearing we need to do some grouping, but we cannot observe the space
    // itself.
    /* Consider the letter A and R, with a space in between:

          ^        |)
         /_\       |\
        /   \______| \
        < c >< l  ><d>

        let c be the width of A
        l contains both the right side bearing of A, as well as the left side bearing of R.

        If we obtain R right of something else, and A left of something else, we may be able to
        determine the bearing from that, then we can subtract those with l and obtain the width
        of the space character.
    */

    let mut s: std::collections::HashMap<char, Vec<AnalysedGlyph>> = Default::default();
    let mut bearing_info: std::collections::HashMap<String, Vec<(usize, String)>> =
        Default::default();

    for (text, histogram, name) in text_histogram.clone() {
        let mut interval_pos = 0;
        let chars = text.chars().collect::<Vec<char>>();
        let intervals = splitter(&histogram, &chars, &collection.char_intervals);
        println!("name: {name:?}");
        println!("  histogram: {histogram:?}");
        println!("  intervals: {intervals:?}");
        let max_found = histogram.iter().max().unwrap_or(&0);
        if *max_found == 0 {
            panic!("Max value found in histogram is 0, probably wrong color? {name:?}");
        }
        for (ci, c) in chars.iter().enumerate() {
            let v = s.entry(*c).or_default();
            let mut ag = AnalysedGlyph {
                name: name.clone(),
                stripped_hist: vec![],
                ..Default::default()
            };
            let interval = &intervals[interval_pos];
            // collect left bearing.
            if *c != ' ' && interval_pos != 0 && ci != 0 {
                let mut left_bearing_start = ci - 1;
                // Walk backwards through the spaces in the string at the current position.
                while left_bearing_start != 0 && chars[left_bearing_start] == ' ' {
                    left_bearing_start -= 1;
                }
                let left_bearing_str = (left_bearing_start..=ci)
                    .map(|i| chars[i])
                    .collect::<String>();
                let v = bearing_info.entry(left_bearing_str.clone()).or_default();
                let distance = interval.0 - intervals[interval_pos - 1].1;
                v.push((distance, name.clone()));
            }
            // collect right bearing.
            if *c != ' ' && (interval_pos + 1) < intervals.len() && (ci + 1) < chars.len() {
                let mut right_bearing_start = ci + 1;
                // Walk backwards through the spaces in the string at the current position.
                while right_bearing_start < chars.len() && chars[right_bearing_start] == ' ' {
                    right_bearing_start += 1;
                }
                let right_bearing_str = (ci..=right_bearing_start)
                    .map(|i| chars[i])
                    .collect::<String>();
                let v = bearing_info.entry(right_bearing_str.clone()).or_default();
                let distance = intervals[interval_pos + 1].0 - interval.1;
                v.push((distance, name.clone()));
            }
            if *c == ' ' {
                // Space will not be in the intervals, so don't populate the histogram and
                // don't advance the interval position.
            } else {
                ag.stripped_hist = histogram[interval.0..interval.1].to_vec();
                interval_pos += 1;
            }
            v.push(ag);
        }
    }

    // Do this pretty print.
    let mut glyph_histograms = s.iter().collect::<Vec<_>>();
    glyph_histograms.sort();
    for (c, entries) in glyph_histograms.iter() {
        println!("char: {c:?}");
        for entry in entries.iter() {
            println!("  {} {:?}", entry.name, entry.stripped_hist);
        }
    }
    println!("---");
    let mut bearings = bearing_info.iter().collect::<Vec<_>>();
    bearings.sort();
    for (c, entries) in bearings.iter() {
        println!("bearing: {c:?}");
        for (d, name) in entries.iter() {
            println!("  {} {:?}", d, name);
        }
    }

    let mut glyph_set: GlyphSet = Default::default();
    println!("Outputting glyph set");
    let mut found_ambiguity = false;
    for (c, entries) in glyph_histograms.iter() {
        if **c == ' ' {
            continue; // this is a footgun, the matcher recurses.
        }
        let mut annotated: std::collections::HashMap<&Vec<u8>, &String> = Default::default();
        for z in entries.iter() {
            // tallest = tallest.max(*z.stripped_hist.iter().max().unwrap_or(&0));
            annotated.insert(&z.stripped_hist, &z.name);
        }
        for (hist, name) in &annotated {
            let mut padded_hist = (*hist).clone();
            if collection.histogram_add_zero_start_end {
                padded_hist.insert(0, 0);
                padded_hist.push(0);
            }
            if annotated.len() == 1 {
                let hits = entries.len();
                println!("Data agrees for {c:?}  ({hits: >3} hits)  {padded_hist:?}");
                glyph_set
                    .entries
                    .push(Glyph::new(&padded_hist, &format!("{c}")));
            } else {
                found_ambiguity = true;
                println!("Ambiguity for char {c:?}, from {name}: {hist:?}");
                glyph_set
                    .entries
                    .push(Glyph::new(&padded_hist, &format!("{c}")));
            }
        }
    }
    if found_ambiguity {
        println!("Found some ambiguous characters");
    } else {
        println!("No ambiguity found");
    }

    if let Some(space_min_size) = collection.space_min_size {
        println!("Inserting space character of: {space_min_size}");
        let mut g = Glyph::new(&vec![0; space_min_size as usize], " ");
        g.set_max_consecutive(Some(1));
        g.set_trim_left(true);
        g.set_trim_right(true);
        g.set_ignore_on_lstrip(true);
        glyph_set.entries.push(g);
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

    let matcher = histogram_text_matcher::matcher::LongestGlyphMatcher::new(&glyph_set.entries);
    // Next, run through text_histogram with our created glyph set and see how we did.
    for (text, histogram, name) in text_histogram.iter() {
        // println!("Name; {name}");
        // println!("hist; {histogram:?}");
        // println!("text; {text}");

        // let matches = histogram_glyph_matcher(&histogram, &glyph_set, 0);
        // let s = matches.iter().map(|(g, _v)| g.glyph()).collect::<String>();

        let final_string = match_histogram_to_string(histogram, &matcher);

        // let s = matches.iter().map(|m|m.token).map(|(g, _v)| g.glyph()).collect::<String>();
        // let s = matches.iter().map(|m|m.token).map(|(g, _v)| g.glyph()).collect::<String>();
        let text_to_compare = if collection.drop_space {
            text.clone().replace(" ", "")
        } else {
            text.clone()
        };
        // println!("s: {s:?}");
        // println!("c: {text_to_compare:?}");
        println!(
            "{name}: found: {final_string:?}  provided: {text} compare: '{text_to_compare}' {}",
            if final_string == *text_to_compare {
                "✔️"
            } else {
                "❌"
            }
        );
    }

    Ok(())
}
