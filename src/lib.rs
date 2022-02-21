use image::imageops::colorops::grayscale;
use image::{open, GenericImage, GenericImageView, Rgb, RgbImage};

use imageproc::map::map_colors;
use imageproc::rect::Rect;

use std::path::Path;

type TokenIndex = (usize, usize);
type Token = (TokenIndex, Rect, image::GrayImage, image::GrayImage);
type TokenMap = Vec<Token>;

type Histogram = Vec<u8>;
type HistogramMap = Vec<((usize, usize), Rect, Histogram)>;

pub fn grow_rect(r: &Rect) -> Rect {
    Rect::at(
        std::cmp::max(r.left() - 1, 0),
        std::cmp::max(r.top() - 1, 0),
    )
    .of_size(r.width() + 2, r.height() + 2)
}

pub fn filter_relevant(image: &RgbImage) -> RgbImage {
    let _tooltip_border = Rgb([58u8, 58u8, 58u8]);
    let font_magic = Rgb([100u8, 100u8, 255u8]);
    let font_unique = Rgb([194u8, 172u8, 109u8]);
    let font_meta = Rgb([255u8, 255u8, 255u8]);
    let font_common = Rgb([238u8, 238u8, 238u8]);
    let font_rune = Rgb([255u8, 160u8, 0u8]);
    let font_ui_value = Rgb([204u8, 189u8, 110u8]);
    let font_ui_map = Rgb([192u8, 170u8, 107u8]);

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
            _ => Rgb([0u8, 0u8, 0u8]),
        }
    })
}

pub fn filter_white(image: &RgbImage) -> RgbImage {
    let white = Rgb([255u8, 255u8, 255u8]);

    map_colors(image, |p| -> Rgb<u8> {
        match p {
            _ if p == white => white,
            _ => Rgb([0u8, 0u8, 0u8]),
        }
    })
}

pub fn line_splitter(image: &RgbImage) -> Vec<imageproc::rect::Rect> {
    // let gray = DynamicImage::ImageRgb8(*image).into_luma8();
    let gray = grayscale(image);
    let height = image.height();

    let mut start: Option<u32> = None;

    let mut res: Vec<imageproc::rect::Rect> = vec![];

    for r in 0..height {
        let sum = (0..image.width())
            .map(|v| gray.get_pixel(v, r))
            .fold(0u32, |a, b| a + ((*b).0[0] as u32));
        let something = sum != 0;

        if start.is_none() && something {
            // start of new row.
            start = Some(r);
        } else if start.is_some() && !something {
            let begin_pos = start.unwrap();
            // finalize
            res.push(Rect::at(0, begin_pos as i32).of_size(image.width(), r - begin_pos));
            start = None;
        }
    }
    res
}

pub fn token_splitter(image: &RgbImage) -> Vec<imageproc::rect::Rect> {
    // let gray = DynamicImage::ImageRgb8(*image).into_luma8();
    let gray = grayscale(image);
    let width = image.width();

    let mut start: Option<u32> = None;

    let mut res: Vec<imageproc::rect::Rect> = vec![];

    for c in 0..width {
        let sum = (0..image.height())
            .map(|v| gray.get_pixel(c, v))
            .fold(0u32, |a, b| a + ((*b).0[0] as u32));
        let something = sum != 0;

        if start.is_none() && something {
            // start of new row.
            start = Some(c);
        } else if start.is_some() && !something {
            let begin_pos = start.unwrap();
            // finalize
            res.push(Rect::at(begin_pos as i32, 0).of_size(c - begin_pos, image.height()));
            start = None;
        }
    }
    res
}

pub fn line_selector_screenshot() {
    let path = Path::new("Screenshot167.png");
    // let path = Path::new("z.png");

    let image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();
    let image = filter_relevant(&image);

    // let mut image = filter_relevant(&image);
    let filtered = filter_relevant(&image);
    let _ = filtered
        .save(Path::new("example_canvas_filtered.png"))
        .unwrap();

    let lines = line_splitter(&image);
    // println!("{lines:#?}");
    let mut image_with_rect = image.clone();
    for b in lines.iter() {
        image_with_rect =
            imageproc::drawing::draw_hollow_rect(&image_with_rect, *b, Rgb([255u8, 0u8, 255u8]));
    }
    let _ = image_with_rect
        .save(Path::new("example_14_lines.png"))
        .unwrap();

    for b in lines.iter() {
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

        for z in tokens.iter() {
            let mut drawable = image::GenericImage::sub_image(
                &mut image_with_rect,
                b.left() as u32,
                b.top() as u32,
                b.width(),
                b.height(),
            );
            imageproc::drawing::draw_hollow_rect_mut(&mut drawable, *z, Rgb([0u8, 255u8, 255u8]));
        }
    }
    let _ = image_with_rect
        .save(Path::new("example_14_boxes.png"))
        .unwrap();
    // let relevant = image_text_matcher::filter_relevant(&image);
    // let _ = relevant.save("result_14.png").unwrap();
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

fn image_to_histogram(image: &image::GrayImage) -> Histogram {
    let mut hist: Histogram = vec![];
    for x in 0..image.width() {
        let mut s: u8 = 0;
        for y in 0..image.height() {
            if image.get_pixel(x, y).0[0] != 0u8 {
                s += 1;
            }
        }
        hist.push(s)
    }
    hist
}

fn histogram_token_map(map: &TokenMap) -> HistogramMap {
    let mut res: HistogramMap = vec![];
    for (pos, input_rect, _image, image_filtered) in map {
        let hist = image_to_histogram(image_filtered);
        res.push((*pos, *input_rect, hist));
    }
    res
}

fn draw_histogram(image: &RgbImage, r: &Rect, hist: &Histogram, color: Rgb<u8>) -> RgbImage {
    let mut c = image.clone();
    for x in 0..hist.len() {
        let img_x = r.left() as u32 + x as u32;
        for y in 0..hist[x] {
            *(c.get_pixel_mut(img_x, r.bottom() as u32 - (y as u32))) = color;
        }
    }
    c
}

fn calc_score(pattern: &[u8], to_match: &[u8]) -> u8 {
    let mut res: u8 = 0;
    let min_width = 4;
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

fn calc_score_normalized(pattern: &[u8], to_match: &[u8]) -> f32 {
    let mut res: u8 = 0;
    // let min_width = 4;
    for (x_a, b) in (0..pattern.len()).zip(to_match.iter()) {
        let a = &(if x_a < pattern.len() {
            pattern[x_a]
        } else {
            0u8
        });
        res += if a > b { a - b } else { b - a };
    }
    ((res as f32) / (pattern.len() as f32)) + 0.75 * 0.1 * (10.0 - (pattern.len() as f32))
}

fn search_tokens(input: &[u8], map: &HistogramMap) -> (Vec<usize>, u32) {
    type Cost = i32;
    let mut res: Vec<usize> = vec![];
    let mut res_cost: i32 = 0;
    let mut fringe = std::collections::BinaryHeap::<(Cost, Vec<usize>, &[u8])>::new();
    // Heap holds (cost_so_far, path_taken, remainder)

    let mut hist_tokens = vec![];
    for (_i, _, hist) in map.iter() {
        hist_tokens.push(hist);
    }
    // let whitespace_cost = vec!(0u8);
    // hist_tokens.push(&whitespace_cost); // Add a space element.

    // Pop first whitespace.
    let mut start_base = 0;
    while start_base < input.len() && input[start_base] == 0 {
        start_base += 1;
    }
    let stripped_input = &input[start_base..];

    // Seed the fringe.
    for (i, z) in hist_tokens.iter().enumerate() {
        let cost_this_token = -(calc_score_min(&z, stripped_input, 0) as Cost);
        let mut start_next = z.len();
        while start_next < stripped_input.len() && stripped_input[start_next] == 0 {
            start_next += 1;
        }
        fringe.push((cost_this_token, vec![i], &stripped_input[start_next..]));
    }
    // println!("Fringe: {fringe:?}");

    let limit = input.len() * map.len();
    let mut counter = 0;

    // Now, we can do the actual search, searching into the lowest cost, expanding the fringe with
    // all options there.
    while let Some((cost_so_far, path_taken, remainder)) = fringe.pop() {
        res = path_taken.clone();
        res_cost = cost_so_far;
        if (remainder.len() == 0) || remainder.len() < 10 {
            println!("Bailing, because remainder is short: {}", remainder.len());
            // res = path_taken;
            break;
        }

        for (i, z) in hist_tokens.iter().enumerate() {
            let cost_this_token = cost_so_far - (calc_score_min(&z, remainder, 0) as Cost);
            let mut new_path = path_taken.clone();
            new_path.push(i);

            // Consume all whitespace until the next token...
            let mut start_next = z.len();
            while start_next < remainder.len() && remainder[start_next] == 0 {
                start_next += 1;
            }
            fringe.push((
                cost_this_token,
                new_path,
                &remainder[std::cmp::min(remainder.len(), start_next)..],
            ));
        }
        // fringe.push((cost_so_far, path_taken, remainder));

        counter += 1;
        if counter > limit || fringe.len() > limit {
            // println!("Something is bad... {counter}, {}", fringe.len());
            break;
        }
    }
    // println!("Fringe: {fringe:#?}");

    (res, (res_cost * -1) as u32)
}

fn things_with_token_map(map: &TokenMap) {
    let reduced_map = crop_token_map(map, true);

    let path = Path::new("./priv/example_canvas.png");
    let image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();
    let mut image = filter_white(&image);

    let hist_map = histogram_token_map(&reduced_map);

    for (i, (index, b, _, _)) in reduced_map.iter().enumerate() {
        image =
            imageproc::drawing::draw_hollow_rect(&image, grow_rect(&b), Rgb([255u8, 0u8, 255u8]));
        let hist = &hist_map[i].2;
        image = draw_histogram(&image, &b, hist, Rgb([255u8, 255u8, 0u8]));
        println!("{index:?} -> {hist:?}");
    }
    let _ = image.save(Path::new("token_map_reduced.png")).unwrap();

    // let path = Path::new("Screenshot167.png");
    // let path = Path::new("Screenshot169.png");
    let path = Path::new("Screenshot176.png");
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

    let mut lines = line_splitter(&image);
    // lines = vec![lines[1]];

    let mut image_line_histogram = image.clone();
    let mut image_mutable = image.clone();
    let mut image_mutable_search = image.clone();
    for line in lines {
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

        // Now, the problem is reduced to some 1d vectors, and we can just advance to the first non-zero
        // then match the best letter, pop that letter, advance again.

        let use_rows = std::collections::hash_set::HashSet::from([0usize, 2, 4, 5, 7, 9]);
        // 0 and 2 are the big font sizes.
        // 4 is the big numbers
        // 5 and 7 are the smaller font sizes.
        // 9 is small numbers

        // Lets reduce the palette we have to use a bit here.
        let mut histmap_reduced: HistogramMap = vec![];
        for z in hist_map.iter() {
            if use_rows.contains(&z.0 .0) {
                histmap_reduced.push(z.clone());
            }
        }
        // println!("reduced: {reduced:?}");
        // println!("Image hist: {sub_image_hist:?}");

        let v = sub_image_hist.clone();
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
            for ((_index, _rect, hist), score) in histmap_reduced.iter().zip(scores.iter_mut()) {
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
                let mut drawable = image_mutable.sub_image(
                    i as u32,
                    line.top() as u32,
                    rect.width(),
                    rect.height(),
                );

                drawable.copy_from(&img, 0, 0);
                // This causes an ICE?
                // for (x, y, p) in img.enumerate_pixels() {
                // let scaling = p.0[0] as f32 / 255.0;
                // let colored_orig = Rgb([(255.0 * scaling) as u8, (255.0 * scaling) as u8, (255.0 * scaling) as u8]);
                // let v = imageproc::pixelops::interpolate(*drawable.get_pixel_mut(x, y), colored_orig, 0.5);
                // drawable.get_pixel_mut(x, y) = v;
                // drawable.get_pixel_mut(x, y) = colored_orig;
                // };

                i += token.2.len();
            } else {
                println!("Huh? didn't have a lowest score??");
                i += 1;
            }
        }

        println!("sub_image_hist: {sub_image_hist:?}");
        let search_res = search_tokens(&sub_image_hist, &histmap_reduced);
        let (best_path, score) = search_res;
        // Draw the search result...
        let mut draw_index = 0usize;
        for index in best_path.iter() {
            let token = &histmap_reduced[*index];
            let original_map_token = find_token(token.0, map);
            // println!("search token: {:?}", histmap_reduced[*index]);
            let (_index, rect, gray, reduced) = original_map_token;
            let img = image::DynamicImage::ImageLuma8(gray.clone()).to_rgb8();
            // Try to blit the original token onto the image we're drawing on.
            let mut drawable = image_mutable_search.sub_image(
                std::cmp::min(draw_index as u32, image_mutable.width() - rect.width()),
                line.top() as u32,
                rect.width(),
                rect.height(),
            );
            drawable.copy_from(&img, 0, 0);
            let shifted_rect = Rect::at(draw_index as i32, new_rect.top())
                .of_size(new_rect.width(), new_rect.height());
            image_mutable_search = draw_histogram(
                &image_mutable_search,
                &shifted_rect,
                &token.2,
                Rgb([255u8, 255u8, 0u8]),
            );
            draw_index += reduced.width() as usize;
        }
        println!("search token: {best_path:?} -> {score:?}");
    }
    let _ = image_mutable_search
        .save(Path::new("token_map_line_search.png"))
        .unwrap();

    let _ = image_mutable
        .save(Path::new("token_map_line_guessed.png"))
        .unwrap();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_v() {
        let full_rejuv_pot_histogram = [
            // F                                             u
            2u8, 15, 14, 4, 4, 4, 4, 4, 2, 0, 0, 0, 0, 0, 6, 2, 1, 1, 1, 2, 1, 1, 1, 7, 0, 0, 0, 0,
            //       l                                       l
            0, 0, 0, 11, 2, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 11, 2, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0,
            //                               R
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 14, 14, 2, 2, 2, 3, 3, 4, 1, 1, 0, 0, 0, 0, 0, 0, 1,
            //e                                        j                           u
            11, 3, 3, 3, 3, 3, 3, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 6, 2, 1, 1, 1, 2,
            //                            v
            1, 1, 1, 7, 0, 0, 0, 0, 0, 0, 2, 2, 2, 1, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0, 1, 11, 3, 3, 3,
            3, 3, 3, 0, 0, 0, 0, 0, 0, 0, 9, 2, 1, 1, 1, 1, 1, 2, 9, 0, 0, 0, 0, 0, 0, 0, 2, 3, 3,
            2, 3, 2, 2, 3, 2, 1, 0, 0, 0, 0, 1, 1, 1, 1, 10, 10, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 9,
            0, 0, 0, 0, 0, 0, 0, 5, 3, 2, 3, 2, 2, 2, 3, 2, 4, 0, 0, 0, 0, 0, 0, 0, 9, 2, 1, 1, 1,
            1, 1, 2, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 14, 14, 2, 0, 0, 2,
            2, 3, 0, 0, 0, 0, 0, 5, 3, 2, 3, 2, 2, 2, 3, 2, 4, 0, 0, 0, 0, 0, 1, 1, 1, 1, 10, 10,
            1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 5, 3, 2, 3, 2, 2, 2, 3, 2, 4, 0,
            0, 0, 0, 0, 0, 0, 9, 2, 1, 1, 1, 1, 1, 2, 9, 0, 0, 0,
        ];

        let histogram_v = [2u8, 2, 2, 1, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0, ];
    }
}
