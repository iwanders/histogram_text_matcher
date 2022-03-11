use image::{Rgb, Rgba, RgbImage, RgbaImage};

use imageproc::drawing::{Canvas, Blend};
 use image::Pixel;

// A trivial function which draws on a Canvas
fn draw_symbol(image: &mut RgbImage, x: u32, y: u32, stamp: &RgbaImage) {
    for yy in 0..stamp.height()
    {
        for xx in 0..stamp.width()
        {
            let mut base_pixel = image.get_pixel(xx + x, yy + y).to_rgba();
            let stamp_pixel = stamp.get_pixel(xx, yy);
            base_pixel.blend(stamp_pixel);
            *image.get_pixel_mut(xx + x, yy + y) = base_pixel.to_rgb();
        }
    }
}


fn letter_to_rgba(v: &str, color: Rgba<u8>) -> RgbaImage
{
    let lines = v.lines().map(|x|{x.trim()}).filter(|x| { !x.is_empty() }).collect::<Vec<&str>>();
    let mut image = RgbaImage::new(lines[0].len() as u32, lines.len() as u32);
    for (y, row) in lines.iter().enumerate()
    {
        for (x, v) in (**row).chars().enumerate()
        {
            match v{
                'x' => {
                    *image.get_pixel_mut(x as u32, y as u32) = color;
                },
                _ => {},
            }
        }
    }
    image
}
fn scale_letter(image: &RgbaImage, scaling: f32) -> RgbaImage
{
    use imageproc::geometric_transformations::*;
    let scale_projection = Projection::scale(scaling, scaling);

    let new_width = (image.width() as f32 * scaling) as u32;
    let new_height = (image.height() as f32 * scaling) as u32;

    let mut new_image = RgbaImage::new(new_width, new_height);

    imageproc::geometric_transformations::warp_into(
    &image,
    &scale_projection,
    imageproc::geometric_transformations::Interpolation::Nearest,
    Rgba([0u8,0,0,0]), 
    &mut new_image);
    new_image
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Letters shall be 7 high. Because that's odd and allows for center lines.
    // width can be variable.
    let a = "
        ..x..
        .x.x.
        x...x
        x...x
        xxxxx
        x...x
        x...x";
    let b = "
        xxxx.
        x...x
        x...x
        xxxxx
        x...x
        x...x
        xxxx.";
    let c = "
        .xxx.
        x...x
        x....
        x....
        x....
        x...x
        .xxx.";
    let w = "
        x.........x
        .x.......x.
        .x.......x.
        .x...x...x.
        ..x.x.x.x..
        ..x.x.x.x..
        ...x...x...";

    let mut image = RgbImage::new(512, 512);

    let stamp_a = letter_to_rgba(&a, Rgba([255u8, 255, 255, 255]));
    let stamp_b = letter_to_rgba(&b, Rgba([255u8, 255, 255, 255]));
    let stamp_c = letter_to_rgba(&c, Rgba([255u8, 255, 255, 255]));
    let stamp_w = letter_to_rgba(&w, Rgba([255u8, 255, 255, 255]));

    let scaled_a = scale_letter(&stamp_a, 4.01);

    draw_symbol(&mut image, 0, 0, &stamp_a);
    draw_symbol(&mut image, 1, 20, &scaled_a);
    draw_symbol(&mut image, 20, 0, &stamp_b);
    draw_symbol(&mut image, 60, 0, &stamp_c);
    draw_symbol(&mut image, 40, 0, &stamp_w);

    let modded_a = imageproc::filter::horizontal_filter(&scaled_a, &[0.75, 1.0, 0.75]);
    let modded_w = imageproc::filter::horizontal_filter(&stamp_w, &[0.75, 1.0, 0.75]);
    draw_symbol(&mut image, 1, 60, &modded_a);
    draw_symbol(&mut image, 40, 60, &modded_w);
    // draw_symbol(&mut image, 1, 60, &scaled_a);

    let _ = image.save("/tmp/dev_example_glyphs.png").unwrap();
    histogram_text_matcher::image_support::dev_histogram_on_image()?;
    Ok(())
}
