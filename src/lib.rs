
use image::{open, Rgb, RgbImage, DynamicImage};
use image::imageops::colorops::grayscale;

use imageproc::map::{map_colors};
use imageproc::rect::Rect;

use std::path::Path;



pub fn filter_relevant(image: &RgbImage) -> RgbImage
{
    let tooltip_border = Rgb([58u8, 58u8, 58u8]);
    let font_magic = Rgb([100u8, 100u8, 255u8]);
    let font_unique = Rgb([194u8, 172u8, 109u8]);
    let font_meta = Rgb([255u8, 255u8, 255u8]);
    let font_common = Rgb([238u8, 238u8, 238u8]);
    let font_ui_value = Rgb([204u8, 189u8, 110u8]);
    let font_ui_map = Rgb([192u8, 170u8, 107u8]);

    map_colors(image, |p| -> Rgb<u8> {
        match p {
            _ if p == tooltip_border => tooltip_border,
            _ if p == font_magic => font_magic,
            _ if p == font_unique => font_unique,
            _ if p == font_common => font_common,
            _ if p == font_meta => font_meta,
            _ if p == font_ui_value => font_ui_value,
            _ if p == font_ui_map => font_ui_map,
            _ => Rgb([0u8, 0u8, 0u8]),
        }
    })

}

pub fn line_splitter(image: &RgbImage) -> Vec<imageproc::rect::Rect>
{
    // let gray = DynamicImage::ImageRgb8(*image).into_luma8();
    let gray = grayscale(image);
    let height = image.height();

    let mut start: Option<u32> = None;

    let mut res: Vec<imageproc::rect::Rect> = vec!();

    for r in 0..height
    {
        let sum = (0..image.width()).map(|v|{gray.get_pixel(v, r)}).fold(0u32, |a, b| { a + ((*b).0[0] as u32) });
        let something = sum != 0;

        if start.is_none() && something
        {
            // start of new row.
            start = Some(r);
        } else if start.is_some() && !something
        {
            let begin_pos = start.unwrap();
            // finalize
            res.push(Rect::at(0, begin_pos as i32).of_size(image.width(), r - begin_pos));
            start = None;
        }
    }
    res
}

pub fn token_splitter(image: &RgbImage) -> Vec<imageproc::rect::Rect>
{
    // let gray = DynamicImage::ImageRgb8(*image).into_luma8();
    let gray = grayscale(image);
    let width = image.width();

    let mut start: Option<u32> = None;

    let mut res: Vec<imageproc::rect::Rect> = vec!();

    for c in 0..width
    {
        let sum = (0..image.height()).map(|v|{gray.get_pixel(c, v)}).fold(0u32, |a, b| { a + ((*b).0[0] as u32) });
        let something = sum != 0;

        if start.is_none() && something
        {
            // start of new row.
            start = Some(c);
        } else if start.is_some() && !something
        {
            let begin_pos = start.unwrap();
            // finalize
            res.push(Rect::at(begin_pos as i32, 0).of_size(c - begin_pos, image.height()));
            start = None;
        }
    }
    res
}

pub fn manipulate_canvas()
{
    let path = Path::new("./priv/example_canvas.png");
    let mut image = open(path).expect(&format!("Could not load image at {:?}", path)).to_rgb8();

    let filtered = filter_relevant(&image);
    let _ = filtered.save(Path::new("example_canvas_filtered.png")).unwrap();

    let lines = line_splitter(&image);
    println!("{lines:#?}");
    let mut image_with_rect = image.clone();
    for b in lines.iter()
    {
       image_with_rect = imageproc::drawing::draw_hollow_rect(&image_with_rect, *b, Rgb([255u8, 0u8, 255u8]));
    }
    let _ = image_with_rect.save(Path::new("example_canvas_boxes.png")).unwrap();

    for b in lines.iter()
    {
        let sub_img = image::SubImage::new(&image, b.left() as u32, b.top() as u32, b.width(), b.height());
        let sub_img = sub_img.to_image();
        let tokens = token_splitter(&sub_img);
        println!("{tokens:#?}");

        for z in tokens.iter()
        {

            let mut drawable = image::GenericImage::sub_image(&mut image_with_rect, b.left() as u32, b.top() as u32, b.width(), b.height());
            imageproc::drawing::draw_hollow_rect_mut(&mut drawable, *z, Rgb([0u8, 255u8, 255u8]));
        }
    }
    let _ = image_with_rect.save(Path::new("example_canvas_boxes.png")).unwrap();
}