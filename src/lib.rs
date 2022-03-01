use image::imageops::colorops::grayscale;
use image::{open, GenericImage, GenericImageView, Rgb, RgbImage};
use imageproc::map::map_colors;
use imageproc::rect::Rect;
use imageproc::rect::Region;

use std::time::{Duration, Instant};

use std::path::Path;

mod dev_util;
use dev_util::*;

mod glyphs;
pub use glyphs::*;

type HistogramMap = Vec<((usize, usize), Rect, Histogram)>;

pub fn grow_rect(r: &Rect) -> Rect {
    Rect::at(
        std::cmp::max(r.left() - 1, 0),
        std::cmp::max(r.top() - 1, 0),
    )
    .of_size(r.width() + 2, r.height() + 2)
}

const _tooltip_border: Rgb<u8> = Rgb([58u8, 58u8, 58u8]);
const font_ui_value: Rgb<u8> = Rgb([204u8, 189u8, 110u8]);
const font_ui_map: Rgb<u8> = Rgb([192u8, 170u8, 107u8]);

const font_meta: Rgb<u8> = Rgb([255u8, 255u8, 255u8]);

const font_common: Rgb<u8> = Rgb([238u8, 238u8, 238u8]);
const font_magic: Rgb<u8> = Rgb([100u8, 100u8, 255u8]);
const font_rare: Rgb<u8> = Rgb([255u8, 255u8, 90u8]);
const font_unique: Rgb<u8> = Rgb([194u8, 172u8, 109u8]);
const font_rune: Rgb<u8> = Rgb([255u8, 160u8, 0u8]);
const font_items: [Rgb<u8>; 5] = [font_common, font_magic, font_rare, font_unique, font_rune];

pub fn filter_relevant(image: &RgbImage) -> RgbImage {
    map_colors(image, |p| -> Rgb<u8> {
        match p {
            // _ if p == tooltip_border => tooltip_border,
            _ if p == font_magic => font_magic,
            _ if p == font_unique => font_unique,
            _ if p == font_common => font_common,
            _ if p == font_meta => font_meta,
            _ if p == font_ui_value => font_ui_value,
            _ if p == font_ui_map => font_ui_map,
            _ if p == font_rune => font_rune,
            _ if p == font_rare => font_rare,
            _ => Rgb([0u8, 0u8, 0u8]),
        }
    })
}

pub fn get_pixel_optional<C>(image: &C, x: i32, y: i32) -> Option<C::Pixel>
where
    C: GenericImageView,
{
    if x >= 0 && x < image.width() as i32 && y >= 0 && y < image.height() as i32 {
        return Some(image.get_pixel(x as u32, y as u32));
    }
    None
}

fn find_token(token: TokenIndex, map: &TokenMap) -> Token {
    for a in map.iter() {
        if a.0 == token {
            return a.clone();
        }
    }
    panic!("Couldn't find token!?");
}

pub fn manipulate_canvas() {
    // line_selector_screenshot();

    let path = Path::new("./priv/example_canvas.png");
    let image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();

    // let mut image = filter_relevant(&image);
    let filtered = filter_white(&image);
    let _ = filtered
        .save(Path::new("example_canvas_filtered.png"))
        .unwrap();

    let lines = line_splitter(&image);
    // println!("{lines:#?}");
    let image_with_rect = image.clone();
    let mut image_with_rect = filter_relevant(&image_with_rect);
    for b in lines.iter() {
        image_with_rect =
            imageproc::drawing::draw_hollow_rect(&image_with_rect, *b, Rgb([255u8, 0u8, 255u8]));
    }
    let _ = image_with_rect
        .save(Path::new("example_canvas_boxes.png"))
        .unwrap();

    let mut token_map: TokenMap = vec![];
    for (r, b) in lines.iter().enumerate() {
        let sub_img = image::SubImage::new(
            &image,
            b.left() as u32,
            b.top() as u32,
            b.width(),
            b.height(),
        );
        let sub_img = sub_img.to_image();
        let tokens = token_splitter(&sub_img);
        // println!("{tokens:#?}");

        for (c, z) in tokens.iter().enumerate() {
            let mut drawable = image::GenericImage::sub_image(
                &mut image_with_rect,
                b.left() as u32,
                b.top() as u32,
                b.width(),
                b.height(),
            );
            imageproc::drawing::draw_hollow_rect_mut(&mut drawable, *z, Rgb([0u8, 255u8, 255u8]));

            let global_rect =
                Rect::at(b.left() + z.left(), b.top() + z.top()).of_size(z.width(), z.height());
            token_map.push((
                (r, c),
                global_rect,
                image::DynamicImage::ImageRgb8(
                    image
                        .view(
                            global_rect.left() as u32,
                            global_rect.top() as u32,
                            global_rect.width(),
                            global_rect.height(),
                        )
                        .to_image(),
                )
                .into_luma8(),
                image::DynamicImage::ImageRgb8(
                    filtered
                        .view(
                            global_rect.left() as u32,
                            global_rect.top() as u32,
                            global_rect.width(),
                            global_rect.height(),
                        )
                        .to_image(),
                )
                .into_luma8(),
            ));
        }
    }
    let _ = image_with_rect
        .save(Path::new("example_canvas_boxes.png"))
        .unwrap();

    // Actually, we only need lines, and the token map.
    things_with_token_map(&token_map);
}

fn crop_token_map(map: &TokenMap, only_width: bool) -> TokenMap {
    let mut output: TokenMap = vec![];
    for (pos, input_rect, image, image_filtered) in map {
        let tmp = image.clone();
        // let mut tmp = filter_white(&tmp);
        let reduced_image = image::DynamicImage::ImageRgb8(filter_white(
            &image::DynamicImage::ImageLuma8(tmp).to_rgb8(),
        ))
        .into_luma8();
        // Determine new bounds in this image.
        let mut min_y = u32::MAX;
        let mut max_y = 0;
        let mut min_x = u32::MAX;
        let mut max_x = 0;
        for (x, y, p) in reduced_image.enumerate_pixels() {
            if p.0[0] != 0u8 {
                min_y = std::cmp::min(min_y, y);
                max_y = std::cmp::max(max_y, y);
                min_x = std::cmp::min(min_x, x);
                max_x = std::cmp::max(max_x, x);
            }
        }

        if only_width {
            min_y = 0;
            max_y = image.height();
        }
        max_x += 1; // What's up with this here!? Necessary to fix the histogram, but something feels off.
        if (min_y == u32::MAX) || (min_x == u32::MAX) {
            output.push((*pos, *input_rect, image.clone(), image_filtered.clone()));
            continue;
        }

        // println!("min_y : {min_y:?}");
        // println!("max_y : {max_y:?}");
        // println!("min_x : {min_x:?}");
        // println!("max_x : {max_x:?}");
        // Crop the actual template.
        let cropped_image = image.view(min_x, min_y, max_x - min_x, max_y - min_y);
        let cropped_filtered_image =
            image_filtered.view(min_x, min_y, max_x - min_x, max_y - min_y);

        // Now, we cook up the new rectangle.
        let new_rect = Rect::at(
            input_rect.left() + min_x as i32,
            input_rect.top() + min_y as i32,
        )
        .of_size(max_x + 1 - min_x, max_y + 1 - min_y);
        // println!("Original rect: {input_rect:?}, new rect: {new_rect:?}");

        output.push((
            *pos,
            new_rect,
            cropped_image.to_image(),
            cropped_filtered_image.to_image(),
        ));
    }
    output
}

fn histogram_token_map(map: &TokenMap) -> HistogramMap {
    let mut res: HistogramMap = vec![];
    for (pos, input_rect, _image, image_filtered) in map {
        let hist = image_to_histogram(image_filtered);
        res.push((*pos, *input_rect, hist));
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

fn token_histogram_matcher(
    y_offset: u32,
    hist: &Vec<u8>,
    map: &TokenMap,
    histmap_reduced: &HistogramMap,
    image_mutable: &mut RgbImage,
) {
    let v = hist;
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
        scores.resize(histmap_reduced.len(), 0 as ScoreType);
        for ((.., hist), score) in histmap_reduced.iter().zip(scores.iter_mut()) {
            *score = calc_score_min(hist, &remainder, 10);
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
            let token = &histmap_reduced[best];
            let score = scores[best];
            println!("Found {best}, with {score} -> {token:?}");

            // sub_image_mut =
            //fn find_token(token: TokenIndex, map: &TokenMap) -> Token
            let original_map_token = find_token(token.0, map);

            let (_index, rect, gray, _reduced) = original_map_token;
            let img = image::DynamicImage::ImageLuma8(gray).to_rgb8();
            // Try to blit the original token onto the image we're drawing on.
            let mut drawable =
                image_mutable.sub_image(i as u32, y_offset, rect.width(), rect.height());

            drawable.copy_from(&img, 0, 0);
            i += token.2.len();
        } else {
            println!("Huh? didn't have a lowest score??");
            i += 1;
        }
    }
}

#[derive(Default, Debug, Copy, Clone)]
struct Bin {
    font_common: u8,
    font_magic: u8,
    font_rare: u8,
    font_unique: u8,
    font_rune: u8,
}
impl Bin {
    fn add_common(&mut self) {
        self.font_common += 1;
    }
    fn sub_common(&mut self) {
        self.font_common = self.font_common.saturating_sub(1);
    }
    fn add_magic(&mut self) {
        self.font_magic += 1;
    }
    fn sub_magic(&mut self) {
        self.font_magic = self.font_magic.saturating_sub(1);
    }
    fn add_rare(&mut self) {
        self.font_rare += 1;
    }
    fn sub_rare(&mut self) {
        self.font_rare = self.font_rare.saturating_sub(1);
    }
    fn add_unique(&mut self) {
        self.font_unique += 1;
    }
    fn sub_unique(&mut self) {
        self.font_unique = self.font_unique.saturating_sub(1);
    }
    fn add_rune(&mut self) {
        self.font_rune += 1;
    }
    fn sub_rune(&mut self) {
        self.font_rune = self.font_rune.saturating_sub(1);
    }
    fn total(&self) -> u8 {
        self.font_common + self.font_magic + self.font_rare + self.font_unique + self.font_rune
    }
}

fn token_binned_histogram_matcher(
    y_offset: u32,
    hist: &Vec<u8>,
    map: &TokenMap,
    histmap_reduced: &HistogramMap,
    image_mutable: &mut RgbImage,
) {
    let v = hist;
    let mut i: usize = 0;
    let mut correct_matches = vec![];
    let mut total_score: u32 = 0;
    while i < v.len() - 1 {
        if v[i] == 0 {
            i += 1;
            continue;
        }

        // v[i] is now the first non-zero entry.
        let remainder = &v[i..];

        type ScoreType = u8;
        let mut scores: Vec<ScoreType> = vec![];
        scores.resize(histmap_reduced.len(), 0 as ScoreType);
        for ((.., hist), score) in histmap_reduced.iter().zip(scores.iter_mut()) {
            *score = calc_score_min(hist, &remainder, 10);
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
            let token = &histmap_reduced[best];
            let score = scores[best];
            total_score += score as u32;
            if (score > 0) {
                // Eliminate this block.
                while v[i] != 0 {
                    i += 1;
                }
                continue;
            }
            /**/
            correct_matches.push((token, i));

            i += token.2.len();
        } else {
            println!("Huh? didn't have a lowest score??");
            i += 1;
        }
    }
    if (correct_matches.len() < 5) || (total_score as f32 / (correct_matches.len() as f32)) > 1.5 {
        return;
    }
    for (token, i) in correct_matches {
        // sub_image_mut =
        //fn find_token(token: TokenIndex, map: &TokenMap) -> Token
        let original_map_token = find_token(token.0, map);

        let (_index, rect, gray, _reduced) = original_map_token;
        let img = image::DynamicImage::ImageLuma8(gray).to_rgb8();
        // Try to blit the original token onto the image we're drawing on.
        let mut drawable = image_mutable.sub_image(i as u32, y_offset, rect.width(), rect.height());

        drawable.copy_from(&img, 0, 0);
    }
}

fn moving_windowed_histogram(image: &RgbImage, map: &TokenMap) {
    let mut image_mutable = image.clone();
    // const font_common: Rgb<u8> = Rgb([238u8, 238u8, 238u8]);
    // const font_magic: Rgb<u8> = Rgb([100u8, 100u8, 255u8]);
    // const font_rare: Rgb<u8> = Rgb([255u8, 255u8, 90u8]);
    // const font_unique: Rgb<u8> = Rgb([194u8, 172u8, 109u8]);
    // const font_rune: Rgb<u8> = Rgb([255u8, 160u8, 0u8]);

    let mut histogram: Vec<Bin> = Vec::<Bin>::new();
    histogram.resize(image.width() as usize, Default::default());
    let window_size = 18;

    // Start at the top, with zero width, then we sum rows for the window size
    // Then, we iterate down, at the bottom of the window add to the histogram
    // and the top row that moves out we subtract.

    fn add_pixel(x: usize, p: &Rgb<u8>, histogram: &mut Vec<Bin>) {
        match p {
            _ if p == &font_common => histogram[x as usize].add_common(),
            _ if p == &font_magic => histogram[x as usize].add_magic(),
            _ if p == &font_rare => histogram[x as usize].add_rare(),
            _ if p == &font_unique => histogram[x as usize].add_unique(),
            _ if p == &font_rune => histogram[x as usize].add_rune(),
            _ => {}
        }
    }

    fn sub_pixel(x: usize, p: &Rgb<u8>, histogram: &mut Vec<Bin>) {
        match p {
            _ if p == &font_common => histogram[x as usize].sub_common(),
            _ if p == &font_magic => histogram[x as usize].sub_magic(),
            _ if p == &font_rare => histogram[x as usize].sub_rare(),
            _ if p == &font_unique => histogram[x as usize].sub_unique(),
            _ if p == &font_rune => histogram[x as usize].sub_rune(),
            _ => {}
        }
    }

    // Let us first, setup the first histogram, this is from 0 to histogram size.
    for y in 0..window_size {
        for x in 0..image.width() {
            let p = image.get_pixel(x, y);
            add_pixel(x as usize, p, &mut histogram);
        }
    }

    let histmap_reduced = reduce_map(&map);
    for y in 1..(image.height() - window_size) {
        // let mut image_mutable_z = image.clone();
        // Do things with the current histogram.
        // Render the histogram to a single entity.

        //fn token_histogram_matcher(y_offset: u32, hist: &Vec<u8>, map: &TokenMap, histmap_reduced: &HistogramMap, image_mutable: &mut RgbImage)
        let mut single_hist: Vec<u8> = vec![];
        single_hist.resize(image.width() as usize, 0);
        for x in 0..image.width() {
            single_hist[x as usize] = histogram[x as usize].total();
        }
        //token_histogram_matcher(y_offset: u32, hist: &Vec<u8>, map: &TokenMap, histmap_reduced: &HistogramMap, image_mutable: &mut RgbImage)
        token_binned_histogram_matcher(y, &single_hist, &map, &histmap_reduced, &mut image_mutable);
        // token_binned_histogram_matcher(y, &single_hist, &map, &histmap_reduced, &mut image_mutable_z);
        // let _ = image_mutable_z.save(Path::new(format!("/tmp/{:0>4}_token_map_moving_histogram_matches.png", y).as_str())).unwrap();

        // Subtract from the side moving out of the histogram.
        for x in 0..image.width() {
            let p = image.get_pixel(x, y);
            sub_pixel(x as usize, p, &mut histogram);
        }

        // Add the side moving into the histogram.
        for x in 0..image.width() {
            let p = image.get_pixel(x, y + window_size);
            add_pixel(x as usize, p, &mut histogram);
        }
    }

    let _ = image_mutable
        .save(Path::new("token_map_moving_histogram_matches.png"))
        .unwrap();
}

fn reduce_map(map: &TokenMap) -> HistogramMap {
    let reduced_map = crop_token_map(map, true);
    let hist_map = histogram_token_map(&reduced_map);
    let use_rows = std::collections::hash_set::HashSet::from([0usize, 1, 2, 3, 4, 5]);
    // let use_rows = std::collections::hash_set::HashSet::from([6, 7, 8]);
    // 0 and 1 are the big font sizes.
    // 2 is the big numbers
    // 3 and 4 are the smaller font sizes.
    // 5 is small numbers
    // 6 is tiny caps
    // 7 is tiny small
    // 8 is tiny letters and symbols.

    // Lets reduce the palette we have to use a bit here.
    let mut histmap_reduced: HistogramMap = vec![];
    for z in hist_map.iter() {
        if use_rows.contains(&z.0 .0) {
            histmap_reduced.push(z.clone());
        }
    }
    histmap_reduced
}

fn things_with_token_map(map: &TokenMap) {
    let reduced_map = crop_token_map(map, true);

    let path = Path::new("./priv/example_canvas.png");
    let image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();
    let mut image = filter_white(&image);

    let hist_map = histogram_token_map(&reduced_map);

    for (i, (index, b, ..)) in reduced_map.iter().enumerate() {
        image =
            imageproc::drawing::draw_hollow_rect(&image, grow_rect(&b), Rgb([255u8, 0u8, 255u8]));
        let hist = &hist_map[i].2;
        image = draw_histogram(&image, &b, hist, Rgb([255u8, 255u8, 0u8]));
        println!("{index:?} -> {hist:?}");
    }
    let _ = image.save(Path::new("token_map_reduced.png")).unwrap();

    let histmap_reduced = reduce_map(&map);

    // let path = Path::new("Screenshot167.png"); // non-aligned lines
    // let path = Path::new("Screenshot169_no_inventory.png");
    // let path = Path::new("Screenshot014.png");
    // let path = Path::new("Screenshot176.png");
    let path = Path::new("Screenshot224.png"); // non-aligned lines
                                               // let path = Path::new("z.png");

    let image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();
    let image = filter_relevant(&image);

    // let mut image = filter_relevant(&image);
    let filtered = filter_relevant(&image);
    let _ = filtered
        .save(Path::new("token_map_test_filtered.png"))
        .unwrap();

    // let now = Instant::now();
    // alternative_to_line_splitter(&image, &histmap_reduced);
    // println!("Alternative {:.6}", now.elapsed().as_secs_f64());

    let now = Instant::now();
    moving_windowed_histogram(&image, &map);
    println!(
        "moving_windowed_histogram {:.6}",
        now.elapsed().as_secs_f64()
    );

    let now = Instant::now();
    let lines = line_splitter(&image);
    println!("line splitter {:.6}", now.elapsed().as_secs_f64());
    // lines = vec![lines[1]];

    let mut image_line_histogram = image.clone();
    let mut image_mutable = image.clone();

    for line in lines {
        // Now, the problem is reduced to some 1d vectors, and we can just advance to the first non-zero
        // then match the best letter, pop that letter, advance again.
        // let line = lines[0];
        println!("{line:?}");
        let sub_image = image::SubImage::new(
            &image,
            line.left() as u32,
            line.top() as u32,
            line.width(),
            line.height(),
        );
        let new_rect =
            Rect::at(line.left() as i32, line.top() as i32).of_size(line.width(), line.height());
        image_line_histogram = imageproc::drawing::draw_hollow_rect(
            &image_line_histogram,
            grow_rect(&line),
            Rgb([255u8, 0u8, 255u8]),
        );

        let _sub_image_mut = sub_image.to_image();

        let sub_image_gray = image::DynamicImage::ImageRgb8(sub_image.to_image()).into_luma8();
        let sub_image_hist = image_to_histogram(&sub_image_gray);

        image_line_histogram = draw_histogram(
            &image_line_histogram,
            &new_rect,
            &sub_image_hist,
            Rgb([255u8, 255u8, 0u8]),
        );
        let _ = image_line_histogram
            .save(Path::new("token_map_line_histogram.png"))
            .unwrap();

        // println!("reduced: {reduced:?}");
        // println!("Image hist: {sub_image_hist:?}");

        let v = sub_image_hist.clone();
        token_histogram_matcher(
            line.top() as u32,
            &v,
            &map,
            &histmap_reduced,
            &mut image_mutable,
        );
    }

    let _ = image_mutable
        .save(Path::new("token_map_line_guessed.png"))
        .unwrap();
}
