
use image_text_matcher::image_support::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image = dev_create_example_glyphs()?;
    let _ = image.save("dev_example_glyphs.png").unwrap();
    Ok(())
}
