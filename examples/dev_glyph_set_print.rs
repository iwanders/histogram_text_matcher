use image::{Rgb, RgbImage};
use imageproc::drawing::draw_text_mut;
use ab_glyph::{FontVec, PxScale};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = std::env::args()
        .nth(1)
        .expect("No glyph set file specified.");
    let output_path = std::env::args().nth(2).expect("No output file specified.");

    let glyph_set = histogram_text_matcher::glyphs::load_glyph_set(&PathBuf::from(&file_path))?;
    let matcher = histogram_text_matcher::matcher::LongestGlyphMatcher::new(&glyph_set.entries);

    let line_offset = 10;
    let line_height = glyph_set.line_height as i32 + line_offset;

    let mut image = RgbImage::new(100, glyph_set.entries.len() as u32 * line_height as u32);

    let font_path = "/usr/share/fonts/truetype/ttf-bitstream-vera/Vera.ttf";
    let data = std::fs::read(font_path)?;
    let font = FontVec::try_from_vec(data).unwrap_or_else(|_| {
        panic!("error constructing a Font from data at {:?}", font_path);
    });


    let height = 10.0;
    let scale = PxScale::from(height);
    for (i, g) in glyph_set.entries.iter().enumerate() {
        let y = i as i32 * line_height;
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
            y as u32 + (glyph_set.line_height as u32 / 2),
            &g.hist(),
            Rgb([255u8, 255u8, 0u8]),
            1.0,
        );
    }
    let _ = image.save(&output_path).unwrap();

    use std::fs::File;
    use std::io::Write;
    let mut file = File::create("/tmp/glyphs.dot")?;
    file.write(matcher.matcher().to_dot(&glyph_set.entries).as_bytes())?;

    let mut file = File::create("/tmp/glyphs_lstrip.dot")?;
    file.write(
        matcher
            .lstrip_matcher()
            .to_dot(&glyph_set.entries)
            .as_bytes(),
    )?;

    Ok(())
}
