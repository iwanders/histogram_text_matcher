//! An example of drawing text. Writes to the user-provided target file.

use image::{open, Rgb, RgbImage};

use imageproc::map::map_colors;

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

fn draw_font() {
    // let path = Path::new("result.png");
    // let mut image = open(path).expect(&format!("Could not load image at {:?}", path)).to_rgb8();
    let mut image = RgbImage::new(1920, 1080);

    let font = Vec::from(include_bytes!("../priv/font.otf") as &[u8]);
    let font = Font::try_from_vec(font).unwrap();

    let height = 25.0;
    let scale = Scale {
        x: height,
        y: height,
    };

    let text = "4605 Gold";
    draw_text_mut(
        &mut image,
        Rgb([238u8, 238u8, 238u8]),
        826,
        392,
        scale,
        &font,
        text,
    );

    let relevant = image_text_matcher::filter_relevant(&image);
    // let relevant = image;
    let _ = relevant.save(Path::new("font_render.png")).unwrap();
}

fn main() {
    draw_font();
    get_relevant();
    get_relevant_font();
    get_relevant_example();
    image_text_matcher::manipulate_canvas();
}
