use image::imageops::colorops::grayscale;
use image::{GenericImage, GenericImageView, Rgb, RgbImage};
use imageproc::map::map_colors;
use imageproc::rect::Rect;

use std::path::Path;

use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};

use crate::glyphs::{Glyph, GlyphSet};

pub type Histogram = Vec<u8>;

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

pub fn render_font_image(
    canvas: (u32, u32),
    font: &Font,
    fontsize: f32,
    elements: &[((u32, u32), String, Rgb<u8>)],
) -> RgbImage {
    let mut image = RgbImage::new(canvas.0, canvas.1);
    let scale = Scale {
        x: fontsize,
        y: fontsize,
    };

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
    hist: &Histogram,
    color: Rgb<u8>,
    alpha: f32,
) {
    for x in 0..hist.len() {
        let img_x = left + x as u32;
        for y in 0..hist[x] {
            let orig = image.get_pixel(img_x, bottom - (y as u32));
            let c = color;
            let res = imageproc::pixelops::interpolate(c, *orig, alpha);
            *(image.get_pixel_mut(img_x, bottom - (y as u32))) = res;
        }
    }
}

/// Something that creates a dummy glyph image.
pub fn dev_create_example_glyphs() -> Result<RgbImage, Box<dyn std::error::Error>> {
    let font_size = 30.0;
    let symbols = vec!['a', 'b', 'c', 'z'];

    let mut drawables = vec![];
    for (i, z) in symbols.iter().enumerate() {
        drawables.push((
            (((i as f32 + 0.5) * font_size) as u32, font_size as u32),
            String::from(*z),
            Rgb([255u8, 255u8, 255u8]),
        ));
    }

    let font = std::fs::read("/usr/share/fonts/truetype/ttf-bitstream-vera/Vera.ttf")?;
    let font = Font::try_from_vec(font).unwrap();

    let size = (
        (font_size * (symbols.len() + 1) as f32) as u32,
        (font_size * 2.0) as u32,
    );

    Ok(render_font_image(size, &font, font_size, &drawables))
}

/// Something that analyses an image with glyphs on it, creating the glyphset with histograms.
pub fn dev_image_to_glyph_set(image: &RgbImage, only_line: Option<usize>) -> GlyphSet {
    let mut result: GlyphSet = Default::default();

    let filtered = filter_white(image);
    let _ = filtered
        .save(Path::new("dev_histogram_filter_white.png"))
        .unwrap();

    let mut lines = line_splitter(image);

    let image_with_rect = image.clone();

    let mut image_with_rect = filter_white(&image_with_rect);
    for b in lines.iter() {
        image_with_rect =
            imageproc::drawing::draw_hollow_rect(&image_with_rect, *b, Rgb([255u8, 0u8, 255u8]));
    }
    let _ = image_with_rect
        .save(Path::new("dev_histogram_lines.png"))
        .unwrap();

    if let Some(index) = only_line {
        lines = vec![lines[index]];
    }

    for (r, b) in lines.iter().enumerate() {
        let sub_img = image::SubImage::new(
            image,
            b.left() as u32,
            b.top() as u32,
            b.width(),
            b.height(),
        );
        result.line_height = std::cmp::max(result.line_height, b.height() as u8);
        let sub_img = sub_img.to_image();
        let tokens = token_splitter(&sub_img);

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

            let mut drawable = image::GenericImage::sub_image(
                &mut image_with_rect,
                b.left() as u32,
                b.top() as u32,
                b.width(),
                b.height(),
            );
            imageproc::drawing::draw_hollow_rect_mut(&mut drawable, *z, Rgb([0u8, 255u8, 255u8]));
            draw_histogram_mut_xy_a(
                &mut image_with_rect,
                z.left() as u32,
                b.bottom() as u32 - 1,
                &sub_img_histogram,
                Rgb([255u8, 0u8, 0u8]),
                0.5,
            );

            let _global_rect =
                Rect::at(b.left() + z.left(), b.top() + z.top()).of_size(z.width(), z.height());

            result.entries.push(Glyph {
                hist: sub_img_histogram,
                glyph: format!("{r}-{c}"),
            });
        }
    }
    let _ = image_with_rect
        .save(Path::new("dev_histogram_boxes.png"))
        .unwrap();
    result
}

pub fn dev_histogram_on_image() -> Result<(), Box<dyn std::error::Error>> {
    let image = dev_create_example_glyphs()?;
    let z =
        crate::image_buffer_view_rgb(image.width(), image.height(), image.as_raw()); // reference
    let x = crate::ImageBufferView::<[u8], 3>::from_raw_ref(
        image.width(),
        image.height(),
        &image.as_raw(),
    ); // reference
    let y = crate::ImageBufferView::<Vec<u8>, 3>::from_raw_ref(
        image.width(),
        image.height(),
        &image.as_raw(),
    ); // copy
    use crate::Image;
    let p = z.pixel(27, 45);
    let px = x.pixel(27, 45);
    let py = y.pixel(27, 45);
    println!("{p:?}");
    println!("{px:?}");
    println!("{py:?}");
    // x + 3
    // z + 3
    // y + 3
    Ok(())
}
