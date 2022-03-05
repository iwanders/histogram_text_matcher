fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image = histogram_text_matcher::image_support::dev_create_example_glyphs()?;
    let _ = image.save("dev_example_glyphs.png").unwrap();
    histogram_text_matcher::image_support::dev_histogram_on_image();
    Ok(())
}
