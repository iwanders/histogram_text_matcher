use image::imageops::colorops::grayscale;
use image::{open, DynamicImage, Rgb, RgbImage, GenericImageView, GenericImage};

use imageproc::map::map_colors;
use imageproc::rect::Rect;

use std::path::Path;

pub fn grow_rect(r: &Rect) -> Rect {
    Rect::at(
        std::cmp::max(r.left() - 1, 0),
        std::cmp::max(r.top() - 1, 0),
    )
    .of_size(r.width() + 2, r.height() + 2)
}

pub fn filter_relevant(image: &RgbImage) -> RgbImage {
    let tooltip_border = Rgb([58u8, 58u8, 58u8]);
    let font_magic = Rgb([100u8, 100u8, 255u8]);
    let font_unique = Rgb([194u8, 172u8, 109u8]);
    let font_meta = Rgb([255u8, 255u8, 255u8]);
    let font_common = Rgb([238u8, 238u8, 238u8]);
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

    let mut image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();
    let mut image = filter_relevant(&image);

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

type TokenMap = Vec<((usize, usize), Rect, image::GrayImage, image::GrayImage)>;

pub fn manipulate_canvas() {
    // line_selector_screenshot();

    let path = Path::new("./priv/example_canvas.png");
    let mut image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();

    // let mut image = filter_relevant(&image);
    let filtered = filter_white(&image);
    let _ = filtered
        .save(Path::new("example_canvas_filtered.png"))
        .unwrap();

    let lines = line_splitter(&image);
    // println!("{lines:#?}");
    let mut image_with_rect = image.clone();
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

            let global_rect = Rect::at(
                b.left() + z.left(),
                b.top() + z.top(),
            )
            .of_size(z.width(), z.height());
            token_map.push((
                (r, c),
                global_rect,
                image::DynamicImage::ImageRgb8(image.view(global_rect.left() as u32, global_rect.top() as u32, global_rect.width(), global_rect.height()).to_image()).into_luma8(),
                image::DynamicImage::ImageRgb8(filtered.view(global_rect.left() as u32, global_rect.top() as u32, global_rect.width(), global_rect.height()).to_image()).into_luma8(),
            ));
        }
    }
    let _ = image_with_rect
        .save(Path::new("example_canvas_boxes.png"))
        .unwrap();

    // Actually, we only need lines, and the token map.
    things_with_token_map(&token_map);
}

fn crop_token_map(map: &TokenMap, only_width: bool) -> TokenMap
{
    let mut output: TokenMap = vec!();
    for (pos, input_rect, image, image_filtered) in map
    {
        let tmp = image.clone();
        let reduced_image = image::DynamicImage::ImageRgb8(filter_relevant(&image::DynamicImage::ImageLuma8(tmp).to_rgb8())).into_luma8();
        // Determine new bounds in this image.
        let mut min_y = u32::MAX;
        let mut max_y = 0;
        let mut min_x = u32::MAX;
        let mut max_x = 0;
        for (x, y, p) in reduced_image.enumerate_pixels() {
            if p.0[0] != 0u8
            {
                min_y = std::cmp::min(min_y, y);
                max_y = std::cmp::max(max_y, y);
                min_x = std::cmp::min(min_x, x);
                max_x = std::cmp::max(max_x, x);
            }
        };

        if only_width
        {
            min_y = 0;
            max_y = image.height();
        }
        max_x += 1; // What's up with this here!? Necessary to fix the histogram, but something feels off.
        if (min_y == u32::MAX) || (min_x == u32::MAX)
        {
            output.push((*pos, *input_rect, image.clone(), image_filtered.clone()));
            continue;
        }

        println!("min_y : {min_y:?}");
        println!("max_y : {max_y:?}");
        println!("min_x : {min_x:?}");
        println!("max_x : {max_x:?}");
        // Crop the actual template.
        let cropped_image = image.view(
            min_x,
            min_y,
            max_x - min_x,
            max_y - min_y,
        );
        let cropped_filtered_image = image_filtered.view(
            min_x,
            min_y,
            max_x - min_x,
            max_y - min_y,
        );

        // Now, we cook up the new rectangle.
        let new_rect = Rect::at(
            input_rect.left() + min_x as i32,
            input_rect.top() + min_y as i32,
        )
        .of_size(max_x + 1 - min_x, max_y + 1 - min_y);
        println!("Original rect: {input_rect:?}, new rect: {new_rect:?}");
    
        output.push((*pos, new_rect, cropped_image.to_image(), cropped_filtered_image.to_image()));
    }
    output
}

type Histogram = Vec<u8>;
fn image_to_histogram(image: &image::GrayImage) -> Histogram
{
    let mut hist : Histogram = vec!();
    for x in 0..image.width()
    {
        let mut s: u8 = 0;
        for y in 0..image.height()
        {
            if image.get_pixel(x,y).0[0] != 0u8 {
                s += 1;
            }
        }
        hist.push(s)
    }
    hist
}

type HistogramMap = Vec<((usize, usize), Rect, Histogram)>;
fn histogram_token_map(map: &TokenMap) -> HistogramMap
{
    let mut res: HistogramMap = vec!();
    for (pos, input_rect, image, image_filtered) in map
    {
        let mut hist = image_to_histogram(image_filtered);
        res.push((*pos, *input_rect, hist));
    }
    res
}

fn draw_histogram(image: &RgbImage, r: &Rect, hist: &Histogram, color: Rgb<u8>) -> RgbImage
{
    let mut c = image.clone();
    for x in 0..hist.len()
    {
        let img_x = r.left() as u32 + x as u32;
        for y in 0..hist[x]
        {
            *(c.get_pixel_mut(img_x, r.bottom() as u32 - (y as u32))) = color;
        }
    }
    c
}

fn things_with_token_map(map: &TokenMap) {
    let reduced_map = crop_token_map(map, true);

    let path = Path::new("./priv/example_canvas.png");
    let mut image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();
    let mut image = filter_relevant(&image);

    let hist_map = histogram_token_map(&reduced_map);

    for (i, (_, b, _, _)) in reduced_map.iter().enumerate() {
        image = imageproc::drawing::draw_hollow_rect(&image, grow_rect(&b), Rgb([255u8, 0u8, 255u8]));
        let hist = &hist_map[i].2;
        image = draw_histogram(&image, &b, hist, Rgb([255u8, 255u8, 0u8]));
    }
    let _ = image
        .save(Path::new("token_map_reduced.png"))
        .unwrap();

        

    let use_rows = std::collections::hash_set::HashSet::from([0usize, 2]);
    // 0 and 2 are the big font sizes.
    // 5 and 7 are the smaller font sizes.
    let path = Path::new("Screenshot167.png");
    // let path = Path::new("z.png");

    let mut image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();
    let mut image = filter_relevant(&image);

    // let mut image = filter_relevant(&image);
    let filtered = filter_relevant(&image);
    let _ = filtered
        .save(Path::new("token_map_test_filtered.png"))
        .unwrap();

    let lines = line_splitter(&image);
    // println!("{lines:#?}");
    let line = lines[0];
    println!("{line:?}");
    let sub_image = image::SubImage::new(
        &image,
        line.left() as u32,
        line.top() as u32,
        line.width(),
        line.height(),
    );
    let new_rect = Rect::at(
        line.left() as i32,
        line.top() as i32,
    )
    .of_size(line.width(), line.height());

    let mut sub_image_mut = sub_image.to_image();

    let sub_image_gray = image::DynamicImage::ImageRgb8(sub_image.to_image()).into_luma8();
    let sub_image_hist = image_to_histogram(&sub_image_gray);
    let sub_image_mut = draw_histogram(&image, &new_rect, &sub_image_hist, Rgb([255u8, 255u8, 0u8]));
    let _ = sub_image_mut
        .save(Path::new("token_map_line_histogram.png"))
        .unwrap();

    
    
}