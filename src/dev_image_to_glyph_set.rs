use image::open;
use std::path::Path;

mod dev_util;
use dev_util::*;

fn main() {
    let file_path = std::env::args()
        .nth(1)
        .expect("No input argument specified.");

    let only_line: Option<usize>;
    if let Some(line_index_as_str) = std::env::args().nth(2) {
        only_line = Some(
            line_index_as_str
                .parse::<usize>()
                .expect("Second arg must be a number."),
        );
    } else {
        only_line = None;
    }

    let path = Path::new(&file_path);
    let image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();
    let glyph_set = dev_image_to_glyph_set(&image, only_line);
    let j = serde_json::to_string(&glyph_set).expect("Will succeed.");
    println!("{j}");
}
