use image::open;
use std::path::Path;

fn main() {
    if std::env::args().len() <= 1 {
        println!("expected: ./binary path_to_file line_number [color_labels] [output_dir]");
        println!("path_to_file: Path to png to run line and token matcher on.");
        println!("line_number: Line number to produce output for, -1 for all.");
        println!("color_labels: Color labels json \"[[r, g, b, 0], [r2,g2,b2, 0]], like scan image, white if unset.\" ");
        println!("output_dir: Output directory to write files to defaults to /tmp/");
        std::process::exit(1);
    }

    let file_path = std::env::args()
        .nth(1)
        .expect("no input image file specified");

    let only_line: Option<usize>;
    if let Some(line_index_as_str) = std::env::args().nth(2) {
        let parsed = line_index_as_str
            .parse::<isize>()
            .expect("second arg must be a number");
        only_line = if parsed != -1 {
            Some(parsed as usize)
        } else {
            None
        };
    } else {
        only_line = None;
    }

    let colors = histogram_text_matcher::util::parse_json_labels(
        &std::env::args()
            .nth(3)
            .unwrap_or("[[255, 255, 255, 0]]".to_owned()),
    )
    .expect("could not parse labels")
    .iter()
    .map(|x| x.0)
    .collect::<Vec<image::Rgb<u8>>>();

    let out_dir = std::env::args()
        .nth(4)
        .or_else(|| Option::Some("/tmp/".to_owned()));

    let path = Path::new(&file_path);
    let image = open(path)
        .expect(&format!("could not load image at {:?}", path))
        .to_rgb8();
    let glyph_set = histogram_text_matcher::image_support::dev_image_to_glyph_set(
        &image,
        only_line,
        &colors,
        &out_dir.as_deref(),
    );

    histogram_text_matcher::glyphs::write_glyph_set(
        &Path::new(out_dir.as_ref().unwrap()).join("glyph_set.json"),
        &glyph_set,
    )
    .expect("writing should succeed");

    histogram_text_matcher::glyphs::write_glyph_set(
        &Path::new(out_dir.as_ref().unwrap()).join("glyph_set.yaml"),
        &glyph_set,
    )
    .expect("writing should succeed");

    println!(
        "Please inspect {} for the output files.",
        out_dir.as_ref().unwrap()
    );
}
