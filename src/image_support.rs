//! Functionality for the image_support feature
use image::imageops::colorops::grayscale;
use image::{GenericImage, Rgb, RgbImage};
use imageproc::map::map_colors;
use imageproc::rect::Rect;

use std::path::Path;

use ab_glyph::{Font, PxScale};
use imageproc::drawing::draw_text_mut;

use crate::glyphs::{Glyph, GlyphSet};

pub use crate::SimpleHistogram as Histogram;

pub fn filter_white(image: &RgbImage) -> RgbImage {
    let white = Rgb([255u8, 255u8, 255u8]);
    filter_colors(image, &vec![white])
}

pub fn filter_colors(image: &RgbImage, colors: &[Rgb<u8>]) -> RgbImage {
    map_colors(image, |p| -> Rgb<u8> {
        for c in colors.iter() {
            if *c == p {
                return p;
            }
        }
        Rgb([0u8, 0u8, 0u8])
    })
}

pub fn filter_primary(image: &RgbImage) -> RgbImage {
    let white = Rgb([255u8, 255u8, 255u8]);
    let red = Rgb([255u8, 0u8, 0u8]);
    let green = Rgb([0u8, 255u8, 0u8]);
    let blue = Rgb([0u8, 0u8, 255u8]);

    map_colors(image, |p| -> Rgb<u8> {
        match p {
            _ if p == white => white,
            _ if p == red => red,
            _ if p == green => green,
            _ if p == blue => blue,
            _ => Rgb([0u8, 0u8, 0u8]),
        }
    })
}

pub fn line_splitter(image: &RgbImage) -> Vec<imageproc::rect::Rect> {
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
    // finalize if we still have to conclude this row.
    if start.is_some() {
        let begin_pos = start.unwrap();
        res.push(Rect::at(0, begin_pos as i32).of_size(image.width(), image.height() - begin_pos));
    }
    res
}

pub fn token_splitter(image: &RgbImage) -> Vec<imageproc::rect::Rect> {
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
    // finalize if we still have to conclude this character.
    if start.is_some() {
        let begin_pos = start.unwrap();
        res.push(Rect::at(begin_pos as i32, 0).of_size(width - begin_pos, image.height()));
    }
    res
}

pub fn image_to_histogram(image: &image::GrayImage) -> Histogram {
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

pub fn render_font_image<F: Font>(
    canvas: (u32, u32),
    font: &F,
    fontsize: f32,
    elements: &[((i32, i32), String, Rgb<u8>)],
) -> RgbImage {
    let mut image = RgbImage::new(canvas.0, canvas.1);
    let scale = PxScale::from(fontsize);

    for ((x, y), s, c) in elements.iter() {
        draw_text_mut(&mut image, *c, *x, *y, scale, font, s);
    }
    image
}

pub fn draw_histogram(image: &RgbImage, r: &Rect, hist: &Histogram, color: Rgb<u8>) -> RgbImage {
    let mut c = image.clone();
    for x in 0..hist.len() {
        let img_x = r.left() as u32 + x as u32;
        for y in 0..hist[x] {
            *(c.get_pixel_mut(img_x, r.bottom() as u32 - (y as u32))) = color;
        }
    }
    c
}

pub fn draw_histogram_mut_xy_a(
    image: &mut image::RgbImage,
    left: u32,
    bottom: u32,
    hist: &[u8],
    color: Rgb<u8>,
    alpha: f32,
) {
    for x in 0..hist.len() {
        let img_x = left + x as u32;
        for y in 0..hist[x] {
            let orig = image.get_pixel(img_x, bottom + 1 - (y as u32));
            let c = color;
            let res = imageproc::pixelops::interpolate(c, *orig, alpha);
            *(image.get_pixel_mut(img_x, bottom + 1 - (y as u32))) = res;
        }
    }
}

/// Scale an image with an integer factor.
pub fn scale_image<I>(image: &I, scaling: u32) -> imageproc::definitions::Image<I::Pixel>
where
    I: image::GenericImage,
    I::Pixel: 'static,
{
    let mut new_image = image::ImageBuffer::new(image.width() * scaling, image.height() * scaling);
    for y in 0..image.height() {
        for x in 0..image.width() {
            let nx = x * scaling;
            let ny = y * scaling;
            imageproc::drawing::draw_filled_rect_mut(
                &mut new_image,
                Rect::at(nx as i32, ny as i32).of_size(scaling, scaling),
                image.get_pixel(x, y),
            );
        }
    }
    new_image
}

fn optionally_save_image(image: &RgbImage, out_dir: &Option<&str>, name: &str) {
    if let Some(path) = out_dir {
        image
            .save(Path::new(path).join(Path::new(name)))
            .expect("may not fail");
    }
}

/// Something that analyses an image with glyphs on it, creating the glyphset with histograms.
pub fn dev_image_to_glyph_set(
    image: &RgbImage,
    only_line: Option<usize>,
    colors: &[image::Rgb<u8>],
    out_dir: &Option<&str>,
) -> GlyphSet {
    let mut result: GlyphSet = Default::default();

    optionally_save_image(&image, out_dir, "dev_histogram_input.png");

    // let filtered = filter_white(image);
    let image_colors = colors.to_vec();
    let filtered = filter_colors(image, &image_colors);

    optionally_save_image(&filtered, out_dir, "dev_histogram_filter_white.png");

    let mut lines = line_splitter(&filtered);

    let image_with_rect = image.clone();

    // let mut image_with_rect = filter_white(&image_with_rect);
    let mut image_with_rect = filter_colors(&image_with_rect, &image_colors);
    for b in lines.iter() {
        image_with_rect =
            imageproc::drawing::draw_hollow_rect(&image_with_rect, *b, Rgb([255u8, 0u8, 255u8]));
    }
    optionally_save_image(&image_with_rect, out_dir, "dev_histogram_lines.png");

    if let Some(index) = only_line {
        lines = vec![lines[index]];
    }

    for (r, b) in lines.iter().enumerate() {
        let sub_img = image::SubImage::new(
            &filtered,
            b.left() as u32,
            b.top() as u32,
            b.width(),
            b.height(),
        );
        result.line_height = std::cmp::max(result.line_height, b.height());
        let sub_img = sub_img.to_image();
        let tokens = token_splitter(&sub_img);
        {
            let row_img_gray = image::DynamicImage::ImageRgb8(sub_img).into_luma8();
            let row_img_histogram = image_to_histogram(&row_img_gray);
            println!("row {r} -> {row_img_histogram:?}");
        }

        for (c, z) in tokens.iter().enumerate() {
            let filtered_token = image::GenericImageView::view(
                &filtered,
                z.left() as u32,
                b.top() as u32,
                z.width(),
                z.height(),
            );
            let sub_img_gray =
                image::DynamicImage::ImageRgb8(filtered_token.to_image()).into_luma8();
            let sub_img_histogram = image_to_histogram(&sub_img_gray);

            let mut drawable =
                image_with_rect.sub_image(b.left() as u32, b.top() as u32, b.width(), b.height());
            imageproc::drawing::draw_hollow_rect_mut(&mut *drawable, *z, Rgb([0u8, 255u8, 255u8]));
            draw_histogram_mut_xy_a(
                &mut image_with_rect,
                z.left() as u32,
                b.bottom() as u32 - if b.bottom() > 1 { 1 } else { 0 },
                &sub_img_histogram,
                Rgb([255u8, 0u8, 0u8]),
                0.5,
            );
            let _global_rect =
                Rect::at(b.left() + z.left(), b.top() + z.top()).of_size(z.width(), z.height());

            result
                .entries
                .push(Glyph::new(&sub_img_histogram, &format!("{r}-{c}")));
        }
    }
    optionally_save_image(&image_with_rect, out_dir, "dev_histogram_boxes.png");

    result.prepare();
    result
}
