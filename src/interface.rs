use std::ops::Deref;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
/// Struct to represent a single pixel.
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}


impl From<u8> for RGB {
    fn from(v: u8) -> Self {
        RGB { r: v, g: v, b: v}
    }
}


pub trait Image
{
    /// Returns the width of the image.
    fn width(&self) -> u32;
    /// Returns the height of the image.
    fn height(&self) -> u32;
    /// Returns a specific pixel's value. The x must be less then width, y less than height.
    fn pixel(&self, x: u32, y: u32) -> RGB;
}

/// A view into a consecutive slice of pixel-esque things, like `image::ImageBuffer::as_raw`
pub struct ImageBufferView<'a , Container>
{
    data: &'a Container,
    width: u32,
    height: u32,
    // stride: usize,
}

impl<Container> ImageBufferView<'_, Container> where
    Container: Deref<Target = [u8]>
{
    pub fn from_raw_ref(width: u32, height: u32, data: &Container) -> ImageBufferView<'_, Container>
    {
        ImageBufferView{data , width, height}
    }
}

impl<'a, Container> Image for ImageBufferView<'a, Container> where
    Container: std::ops::Index<usize>,
    <Container as std::ops::Index<usize>>::Output: Copy + Sized,
    RGB: From<<Container as std::ops::Index<usize>>::Output>
{
    fn width(&self) -> u32
    {
        self.width
    }
    fn height(&self) -> u32
    {
        self.width
    }
    fn pixel(&self, x: u32, y: u32) -> RGB
    {
        self.data[(y * self.width + x) as usize].into()
    }
}