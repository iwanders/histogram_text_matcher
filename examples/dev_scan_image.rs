use image::open;
use std::path::PathBuf;
use std::time::Instant;

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

    let input_image_file = std::env::args().nth(2).expect("no image file specified");

    let glyph_path = PathBuf::from(&glyph_set_file);
    let glyph_set = histogram_text_matcher::glyphs::load_glyph_set(&glyph_path)
        .expect(&format!("could not load image at {:?}", glyph_set_file));

    let image_path = PathBuf::from(&input_image_file);
    let orig_image = open(&image_path)
        .expect(&format!("could not load image at {:?}", input_image_file))
        .to_rgb8();

    let image = histogram_text_matcher::image_support::rgb_image_to_view(&orig_image);
    use histogram_text_matcher::RGB;
    let labels = vec![
        (RGB::white(), 0),
        (RGB::rgb(238, 238, 238), 1),
        (RGB::rgb(100, 100, 255), 1),
        (RGB::rgb(194, 192, 107), 1),
        (RGB::rgb(194, 172, 109), 4),
    ];

    let matcher = histogram_text_matcher::matcher::LongestGlyphMatcher::new(&glyph_set.entries);

    let now = Instant::now();
    let matches = histogram_text_matcher::moving_windowed_histogram(
        &image,
        glyph_set.line_height,
        &matcher,
        &labels,
    );
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
    println!("Took {}", now.elapsed().as_secs_f64());

    use std::fs;
    histogram_text_matcher::util::write_match_html(
        orig_image.width(),
        orig_image.height(),
        &matches,
        &fs::canonicalize(image_path).expect("can be made absolute"),
        &PathBuf::from("/tmp/zzz.html"),
    )
    .expect("should succeed");
}
