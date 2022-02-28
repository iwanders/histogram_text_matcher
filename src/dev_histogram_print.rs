

use std::path::Path;
use image::imageops::colorops::grayscale;
use image::{open, GenericImage, GenericImageView, Rgb, RgbImage};
use imageproc::rect::Region;
use imageproc::map::map_colors;
use imageproc::rect::Rect;

mod dev_util;
use dev_util::*;

fn main() {
    let file_path = std::env::args().nth(1).expect("No input argument specified.");
    let path = Path::new(&file_path);
    let image = open(path)
        .expect(&format!("Could not load image at {:?}", path))
        .to_rgb8();

    let mut image_relevant = image_text_matcher::filter_relevant(&image);
    let _ = image_relevant
        .save(Path::new("dev_histogram_filter_relevant.png"))
        .unwrap();


    let filtered = filter_white(&image);
    let _ = filtered
        .save(Path::new("dev_histogram_filter_white.png"))
        .unwrap();

    let lines = line_splitter(&filtered);


    let image_with_rect = image.clone();

    let mut image_with_rect = filter_white(&image_with_rect);
    for b in lines.iter() {
        image_with_rect =
            imageproc::drawing::draw_hollow_rect(&image_with_rect, *b, Rgb([255u8, 0u8, 255u8]));
    }
    let _ = image_with_rect
        .save(Path::new("dev_histogram_lines.png"))
        .unwrap();

    let mut token_map: TokenMap = vec![];
    for (r, b) in lines.iter().enumerate() {
        let sub_img = image::SubImage::new(
            &image,
            b.left() as u32,
            b.top() as u32,
            b.width(),
            b.height(),
        );
        let sub_img = sub_img.to_image();
        let tokens = token_splitter(&sub_img);

        for (c, z) in tokens.iter().enumerate() {

            let filtered_token = image::GenericImageView::view(
                &filtered,
                z.left() as u32,
                b.top() as u32,
                z.width(),
                z.height(),
            );
            let sub_img_gray = image::DynamicImage::ImageRgb8(filtered_token.to_image()).into_luma8();
            let sub_img_histogram = image_to_histogram(&sub_img_gray);

            println!("{r},{c} -> {sub_img_histogram:?}");

            let mut drawable = image::GenericImage::sub_image(
                &mut image_with_rect,
                b.left() as u32,
                b.top() as u32,
                b.width(),
                b.height(),
            );
            imageproc::drawing::draw_hollow_rect_mut(&mut drawable, *z, Rgb([0u8, 255u8, 255u8]));
            draw_histogram_mut_xy_a(&mut image_with_rect, z.left() as u32, b.bottom() as u32 -1, &sub_img_histogram, Rgb([255u8, 0u8, 0u8]), 0.5);

            let global_rect =
                Rect::at(b.left() + z.left(), b.top() + z.top()).of_size(z.width(), z.height());

            

            token_map.push((
                (r, c),
                global_rect,
                image::DynamicImage::ImageRgb8(
                    image
                        .view(
                            global_rect.left() as u32,
                            global_rect.top() as u32,
                            global_rect.width(),
                            global_rect.height(),
                        )
                        .to_image(),
                )
                .into_luma8(),
                image::DynamicImage::ImageRgb8(
                    filtered
                        .view(
                            global_rect.left() as u32,
                            global_rect.top() as u32,
                            global_rect.width(),
                            global_rect.height(),
                        )
                        .to_image(),
                )
                .into_luma8(),
            ));
        }
    }
    let _ = image_with_rect
        .save(Path::new("dev_histogram_boxes.png"))
        .unwrap();

}
