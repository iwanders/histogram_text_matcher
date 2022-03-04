use image::open;
use std::path::Path;

fn main() {
    let file_path = std::env::args()
        .nth(1)
        .expect("no input image file specified");

    let only_line: Option<usize>;
    if let Some(line_index_as_str) = std::env::args().nth(2) {
        only_line = Some(
            line_index_as_str
                .parse::<usize>()
                .expect("second arg must be a number"),
        );
    } else {
        only_line = None;
    }

    let path = Path::new(&file_path);
    let image = open(path)
        .expect(&format!("could not load image at {:?}", path))
        .to_rgb8();
    let glyph_set = image_text_matcher::image_support::dev_image_to_glyph_set(&image, only_line);
    let j = serde_json::to_string(&glyph_set).expect("will succeed");
    println!("{j}");
}
