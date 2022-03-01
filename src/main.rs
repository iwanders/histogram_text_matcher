//! An example of drawing text. Writes to the user-provided target file.

use image::open;

use std::path::Path;

fn get_relevant_example() {
    let path = Path::new("./priv/example_canvas.png");
    // let path = Path::new("z.png");

    let image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();

    let relevant = image_text_matcher::filter_relevant(&image);
    let _ = relevant.save("example_canvas_reduced.png").unwrap();
}

fn main() {
    image_text_matcher::manipulate_canvas();
}
