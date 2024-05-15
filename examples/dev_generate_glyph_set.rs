use ab_glyph::{Font, FontVec, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
// use imageproc::drawing::draw_text_mut;
use clap::arg;
use histogram_text_matcher::glyphs::{Glyph, GlyphSet};
use histogram_text_matcher::image_support::image_to_histogram;
use std::path::{Path, PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = clap::Command::new("generate_glyph_set")
            .arg(arg!([fontpath] "The font to use, path to ttf.").required(true).value_parser(clap::value_parser!(std::path::PathBuf)))
            .arg(arg!([fontsize] "The size to use, integer, like 20.").required(true).value_parser(clap::value_parser!(f32)))
            .arg(
                clap::arg!(--"output-dir" <PATH>).value_parser(clap::value_parser!(std::path::PathBuf))
                .default_value("."),
            )
            .arg(
                clap::arg!(--"filename" <FILENAME> "Use this filename in the output directory instead of the input filename"),
            )
            .arg(
                clap::arg!(--"name" <NAME> "Defaults to the file name of the image.").value_parser(clap::builder::NonEmptyStringValueParser::new()),
            )
            .arg(
                clap::arg!(--"description" <DESCRIPTION> "A longer description of this pattern." ).value_parser(clap::builder::NonEmptyStringValueParser::new()),
            )
        .get_matches();

    let font_path = matches
        .get_one::<PathBuf>("fontpath")
        .expect("missing fontpath");
    let data = std::fs::read(font_path)?;
    let font = FontVec::try_from_vec(data).unwrap_or_else(|_| {
        panic!("error constructing a Font from data at {:?}", font_path);
    });

    let font_size = matches
        .get_one::<f32>("fontsize")
        .expect("missing fontsize");

    // let line_offset = 2.0 * font_size;

    let useful_range = 32..127;

    let scale = PxScale::from(*font_size);

    let scalefont = font.as_scaled(scale);

    let glyph_height = scalefont.height();
    let glyph_descent = scalefont.descent();
    let glyph_ascent = scalefont.ascent();

    let mut glyph_set: GlyphSet = Default::default();

    let mut tallest = 0;
    for i in useful_range {
        if let Some(c) = std::char::from_u32(i) {
            let glyph_id = scalefont.glyph_id(c);
            let glyph_h_advance = scalefont.h_advance(glyph_id);
            let glyph_h_side_bearing = scalefont.h_side_bearing(glyph_id);
            let w = (glyph_h_side_bearing + glyph_h_advance).ceil() as u32;
            let h = glyph_height.ceil() as u32 + 1;
            let mut image = RgbaImage::new(w, h);

            // Render the glyph, for inspection.
            let color = Rgba([0u8, 0u8, 0u8, 255u8]);
            draw_text_mut(&mut image, color, 0, 0, scale, &font, &format!("{}", c));
            image.save(format!("/tmp/glyph_{i:3>2}.png"))?;

            // Extract saturated black.
            let mut v = image.clone();
            for p in v.pixels_mut() {
                if *p != Rgba([0u8, 0u8, 0u8, 255u8]) {
                    *p = Rgba([255u8, 255u8, 255u8, 0u8]);
                }
            }
            v.save(format!("/tmp/glyph_{i:3>2}_masked.png"))?;

            // Now invert it such that we get white for fonts.
            image::imageops::colorops::invert(&mut v);
            let row_img_gray = image::DynamicImage::ImageRgba8(v).into_luma8();
            row_img_gray.save(format!("/tmp/glyph_{i:3>2}_gray.png"))?;
            let row_img_histogram = image_to_histogram(&row_img_gray);
            tallest = tallest.max(*row_img_histogram.iter().max().unwrap());
            glyph_set
                .entries
                .push(Glyph::new(&row_img_histogram, &format!("{c}")));
        }
    }

    glyph_set.line_height = tallest as u32;
    glyph_set.prepare();

    histogram_text_matcher::glyphs::write_glyph_set(
        &Path::new("/tmp/").join("glyph_set.json"),
        &glyph_set,
    )
    .expect("writing should succeed");

    histogram_text_matcher::glyphs::write_glyph_set(
        &Path::new("/tmp/").join("glyph_set.yaml"),
        &glyph_set,
    )
    .expect("writing should succeed");

    Ok(())
}
