use image::{Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = std::env::args()
        .nth(1)
        .expect("No glyph set file specified.");
    let output_path = std::env::args().nth(2).expect("No output file specified.");

    let glyph_set = histogram_text_matcher::glyphs::load_glyph_set(&PathBuf::from(&file_path))?;

    let line_offset: u32 = 10;
    let line_height = glyph_set.line_height as u32 + line_offset;

    let mut image = RgbImage::new(100, glyph_set.entries.len() as u32 * line_height);

    let font = std::fs::read("/usr/share/fonts/truetype/ttf-bitstream-vera/Vera.ttf")?;
    let font = Font::try_from_vec(font).unwrap();

    let height = 10.0;
    let scale = Scale {
        x: height,
        y: height,
    };
    for (i, g) in glyph_set.entries.iter().enumerate() {
        let y = i as u32 * line_height;
        draw_text_mut(
            &mut image,
            Rgb([255u8, 255u8, 255u8]),
            0,
            y,
            scale,
            &font,
            &g.glyph(),
        );
        histogram_text_matcher::image_support::draw_histogram_mut_xy_a(
            &mut image,
            10,
            y + (glyph_set.line_height as u32 / 2),
            &g.hist(),
            Rgb([255u8, 255u8, 0u8]),
            1.0,
        );
    }
    let _ = image.save(&output_path).unwrap();

    Ok(())
}
