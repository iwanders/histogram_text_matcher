//! An example of drawing text. Writes to the user-provided target file.

use image::{open, Rgb};

use imageproc::map::{map_colors};


use std::path::Path;

fn main() {
    let path = Path::new("Screenshot167.png");
    // let path = Path::new("z.png");

    let image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();
    
    let _subtract = 0;

    let tooltip_border = Rgb([58u8, 58u8, 58u8]);
    let font_magic = Rgb([100u8, 100u8, 255u8]);
    let font_unique = Rgb([194u8, 172u8, 109u8]);
    let font_meta = Rgb([255u8, 255u8, 255u8]);
    let font_common = Rgb([238u8, 238u8, 238u8]);
    let font_ui_value = Rgb([204u8, 189u8, 110u8]);

    let relevant = map_colors(&image, |p| -> Rgb<u8> {
        match p {
            _ if p == tooltip_border => tooltip_border,
            _ if p == font_magic => font_magic,
            _ if p == font_unique => font_unique,
            _ if p == font_common => font_common,
            _ if p == font_meta => font_meta,
            _ if p == font_ui_value => font_ui_value,
            _ => Rgb([0u8, 0u8, 0u8]),
        }
    });

    let _ = relevant.save("result.png").unwrap();
}
