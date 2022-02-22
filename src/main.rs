//! An example of drawing text. Writes to the user-provided target file.

use image::{open, Rgb, RgbImage};

use std::path::Path;

use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};

fn get_relevant() {
    let path = Path::new("Screenshot014.png");
    // let path = Path::new("z.png");

    let image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();

    let relevant = image_text_matcher::filter_relevant(&image);
    let _ = relevant.save("result_14.png").unwrap();
}

fn get_relevant_font() {
    let path = Path::new("font_25_with_gimp.png");
    // let path = Path::new("z.png");

    let image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();

    let relevant = image_text_matcher::filter_relevant(&image);
    let _ = relevant.save("font_25_with_gimp_reduced.png").unwrap();
}

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

    get_relevant();
    get_relevant_font();
    get_relevant_example();
    image_text_matcher::manipulate_canvas();
}
