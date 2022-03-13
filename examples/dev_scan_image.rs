use image::open;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    if std::env::args().len() <= 1 {
        println!("expected: ./binary glyph_set_file input_image_file labels_json [output_file]");
        println!("glyph_set_file: File to load the glyph set from.");
        println!("input_image_file: File to search in.");
        std::process::exit(1);
    }

    let glyph_set_file = std::env::args()
        .nth(1)
        .expect("no glyph set file specified");

    let input_image_file = std::env::args().nth(2).expect("no image file specified");

    let labels = histogram_text_matcher::util::parse_json_labels(
        &std::env::args().nth(3).expect("no label_json"),
    )
    .expect("could not parse labels");

    let glyph_path = PathBuf::from(&glyph_set_file);
    let glyph_set = histogram_text_matcher::glyphs::load_glyph_set(&glyph_path)
        .expect(&format!("could not load glyph set at {:?}", glyph_set_file));

    let image_path = PathBuf::from(&input_image_file);
    let orig_image = open(&image_path)
        .expect(&format!("could not load image at {:?}", input_image_file))
        .to_rgb8();

    let output_file;
    if let Some(output_file_specified) = std::env::args().nth(4) {
        output_file = output_file_specified;
    } else {
        // do something smart based on the file name.
        let old_ext = image_path
            .extension()
            .expect("should have extension")
            .to_str()
            .expect("extension not valid string");
        let filename = image_path
            .file_name()
            .expect("image should have file")
            .to_str()
            .expect("image file not valid string")
            .to_owned();
        output_file = String::from("/tmp/") + &filename.replace(old_ext, "html");
    }

    let image = histogram_text_matcher::image_support::rgb_image_to_view(&orig_image);

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
        println!(" -> {}", m.to_string());
    }
    println!("Took {}", now.elapsed().as_secs_f64());

    use std::fs;
    histogram_text_matcher::util::write_match_html(
        orig_image.width(),
        orig_image.height(),
        &matches,
        &fs::canonicalize(image_path).expect("can be made absolute"),
        &PathBuf::from(output_file),
    )
    .expect("should succeed");
}
