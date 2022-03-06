use image::open;
use std::path::{Path, PathBuf};

fn main() {
    if std::env::args().len() <= 1 {
        println!("expected: ./binary glyph_set_file input_image_file");
        println!("glyph_set_file: File to load the glyph set from.");
        println!("input_image_file: File to search in.");
        std::process::exit(1);
    }

    let glyph_set_file = std::env::args()
        .nth(1)
        .expect("no glyph set file specified");

    let input_image_file = std::env::args()
        .nth(2)
        .expect("no image file specified");


    let glyph_set = histogram_text_matcher::glyphs::load_glyph_set(&PathBuf::from(&glyph_set_file)).expect(&format!("could not load image at {:?}", glyph_set_file));

    let path = Path::new(&input_image_file);
    let image = open(path)
        .expect(&format!("could not load image at {:?}", path))
        .to_rgb8();

    let image = histogram_text_matcher::image_support::rgb_image_to_view(&image);
    let labels = vec![(histogram_text_matcher::RGB::white(), 0)];

    let matches = histogram_text_matcher::moving_windowed_histogram(&image, &glyph_set, &labels);
    for m in matches.iter() {
        let location = &m.location;
        print!("{location:?} -> ");
        for t in m.tokens.iter() {
            let l = t.label;
            let g = t.glyph.glyph();
            print!(" {g:?}#{l}");
        }
        println!();
    }


}
