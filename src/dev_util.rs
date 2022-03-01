use image::imageops::colorops::grayscale;
use image::{open, GenericImage, GenericImageView, Rgb, RgbImage};
use imageproc::map::map_colors;
use imageproc::rect::Rect;
use imageproc::rect::Region;
use std::path::Path;

pub type Histogram = Vec<u8>;

pub type TokenIndex = (usize, usize);
pub type Token = (TokenIndex, Rect, image::GrayImage, image::GrayImage);
pub type TokenMap = Vec<Token>;

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
