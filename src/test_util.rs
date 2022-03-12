use image::Pixel;
use image::{Rgb, RgbImage, Rgba, RgbaImage};

/// A trivial function which an rgba stamp on the image at x,y position.
pub fn apply_stamp(image: &mut RgbImage, x: u32, y: u32, stamp: &RgbaImage) {
    for yy in 0..stamp.height() {
        for xx in 0..stamp.width() {
            let mut base_pixel = image.get_pixel(xx + x, yy + y).to_rgba();
            let stamp_pixel = stamp.get_pixel(xx, yy);
            base_pixel.blend(stamp_pixel);
            *image.get_pixel_mut(xx + x, yy + y) = base_pixel.to_rgb();
        }
    }
}

/// Scale an rgba image.
pub fn scale_image_rgba(image: &RgbaImage, scaling: f32) -> RgbaImage {
    use imageproc::geometric_transformations::*;
    let scale_projection = Projection::scale(scaling, scaling);

    let new_width = (image.width() as f32 * scaling) as u32;
    let new_height = (image.height() as f32 * scaling) as u32;

    let mut new_image = RgbaImage::new(new_width, new_height);

    imageproc::geometric_transformations::warp_into(
        &image,
        &scale_projection,
        imageproc::geometric_transformations::Interpolation::Nearest,
        Rgba([0u8, 0, 0, 0]),
        &mut new_image,
    );
    new_image
}

pub mod test_alphabet {
    use super::*;

    /// Helper function to convert letters from the ascii format below to an rgba image.
    fn letter_to_rgba(v: &str, color: Rgba<u8>) -> RgbaImage {
        let lines = v
            .lines()
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .collect::<Vec<&str>>();
        let mut image = RgbaImage::new(lines[0].len() as u32, lines.len() as u32);
        for (y, row) in lines.iter().enumerate() {
            for (x, v) in (**row).chars().enumerate() {
                match v {
                    'x' => {
                        *image.get_pixel_mut(x as u32, y as u32) = color;
                    }
                    _ => {}
                }
            }
        }
        image
    }

    const LETTER_HEIGHT: u32 = 7;
    // Letters shall be 7 high. Because that's odd and allows for center lines.
    // width can be variable.
    const A: &'static str = "
        ..x..
        .x.x.
        x...x
        x...x
        xxxxx
        x...x
        x...x";
    const B: &'static str = "
        xxxx.
        x...x
        x...x
        xxxxx
        x...x
        x...x
        xxxx.";
    const C: &'static str = "
        .xxx.
        x...x
        x....
        x....
        x....
        x...x
        .xxx.";
    const D: &'static str = "
        xxxx.
        x...x
        x...x
        x...x
        x...x
        x...x
        xxxx.";
    const E: &'static str = "
        xxxxx
        x....
        x....
        xxxxx
        x....
        x....
        xxxxx";
    const W: &'static str = "
        x.........x
        .x.......x.
        .x.......x.
        .x...x...x.
        ..x.x.x.x..
        ..x.x.x.x..
        ...x...x...";
    const SPACE: &'static str = "
        .....
        .....
        .....
        .....
        .....
        .....
        .....";

    pub fn white_a() -> RgbaImage {
        letter_to_rgba(&A, Rgba([255u8, 255, 255, 255]))
    }
    pub fn white_b() -> RgbaImage {
        letter_to_rgba(&B, Rgba([255u8, 255, 255, 255]))
    }
    pub fn white_c() -> RgbaImage {
        letter_to_rgba(&C, Rgba([255u8, 255, 255, 255]))
    }
    pub fn white_d() -> RgbaImage {
        letter_to_rgba(&D, Rgba([255u8, 255, 255, 255]))
    }
    pub fn white_e() -> RgbaImage {
        letter_to_rgba(&E, Rgba([255u8, 255, 255, 255]))
    }
    pub fn white_w() -> RgbaImage {
        letter_to_rgba(&W, Rgba([255u8, 255, 255, 255]))
    }
    pub fn white_space() -> RgbaImage {
        letter_to_rgba(&SPACE, Rgba([255u8, 255, 255, 255]))
    }

    pub fn render_standard(image: &mut RgbImage, x: u32, y: u32, text: &str) -> u32 {
        render_standard_color(image, x, y, text, Rgb([255u8, 255, 255]))
    }

    pub fn render_standard_color(
        image: &mut RgbImage,
        x: u32,
        y: u32,
        text: &str,
        color: Rgb<u8>,
    ) -> u32 {
        let lsb = 1;
        let rsb = 1;
        let mut x = x + lsb;

        let r = color.channels()[0];
        let g = color.channels()[1];
        let b = color.channels()[2];
        for c in text.chars() {
            let l = match c {
                'a' => letter_to_rgba(&A, Rgba([r, g, b, 255])),
                'b' => letter_to_rgba(&B, Rgba([r, g, b, 255])),
                'c' => letter_to_rgba(&C, Rgba([r, g, b, 255])),
                'd' => letter_to_rgba(&D, Rgba([r, g, b, 255])),
                'e' => letter_to_rgba(&E, Rgba([r, g, b, 255])),
                'w' => letter_to_rgba(&W, Rgba([r, g, b, 255])),
                ' ' => white_space(),
                _ => {
                    panic!("letter does not exist in alphabet")
                }
            };
            apply_stamp(image, x, y, &l);
            x += l.width() + rsb;
        }
        x
    }

    const TEXT_STANDARD_ALPHABET: &'static str = "abcdew";
    pub fn render_standard_alphabet() -> RgbImage {
        let mut image = RgbImage::new(512, 512);
        let width = render_standard(&mut image, 0, 1, &TEXT_STANDARD_ALPHABET);
        let mut image = RgbImage::new(width, LETTER_HEIGHT + 2);
        render_standard(&mut image, 0, 0, &TEXT_STANDARD_ALPHABET);
        image
    }

    pub fn standard_alphabet() -> (RgbImage, &'static str) {
        (render_standard_alphabet(), TEXT_STANDARD_ALPHABET)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_test_alphabet() {
        let mut image = RgbImage::new(512, 512);

        let stamp_a = test_alphabet::white_a();
        let stamp_b = test_alphabet::white_b();
        let stamp_c = test_alphabet::white_c();
        let stamp_d = test_alphabet::white_d();
        let stamp_e = test_alphabet::white_e();
        let stamp_w = test_alphabet::white_w();

        let scaled_a = scale_image_rgba(&stamp_a, 2.01);

        apply_stamp(&mut image, 0, 0, &stamp_a);
        apply_stamp(&mut image, 1, 20, &scaled_a);
        apply_stamp(&mut image, 20, 0, &stamp_b);
        apply_stamp(&mut image, 60, 0, &stamp_c);
        apply_stamp(&mut image, 80, 0, &stamp_d);
        apply_stamp(&mut image, 100, 0, &stamp_e);
        apply_stamp(&mut image, 40, 0, &stamp_w);

        let modded_a = imageproc::filter::horizontal_filter(&scaled_a, &[0.75, 1.0, 0.75]);
        let modded_w = imageproc::filter::horizontal_filter(&stamp_w, &[0.75, 1.0, 0.75]);
        apply_stamp(&mut image, 1, 60, &modded_a);
        apply_stamp(&mut image, 40, 60, &modded_w);
        apply_stamp(&mut image, 1, 60, &scaled_a);

        test_alphabet::render_standard(&mut image, 50, 100, "abcdew abcdew");

        let _ = image.save("/tmp/dev_example_glyphs.png").unwrap();
    }
}
