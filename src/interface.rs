#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
/// Struct to represent a single pixel.
pub struct RGB {
    /// The red channel
    pub r: u8,
    /// The green channel
    pub g: u8,
    /// The blue channel
    pub b: u8,
}

#[cfg(test)]
use image::Rgb;
#[cfg(test)]
impl PartialEq<&RGB> for &Rgb<u8> {
    fn eq(&self, other: &&RGB) -> bool {
        use image::Pixel;
        self.channels() == &[other.r, other.g, other.b]
    }
}

impl RGB {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        RGB { r, g, b }
    }

    pub fn white() -> Self {
        RGB {
            r: 255,
            g: 255,
            b: 255,
        }
    }
    pub fn black() -> Self {
        RGB { r: 0, g: 0, b: 0 }
    }
    pub fn red() -> Self {
        RGB { r: 255, g: 0, b: 0 }
    }
    pub fn green() -> Self {
        RGB { r: 0, g: 255, b: 0 }
    }
    pub fn blue() -> Self {
        RGB { r: 0, g: 0, b: 255 }
    }


    pub fn to_rgb(&self) -> image::Rgb<u8> {
        image::Rgb([self.r, self.g, self.b])
    }
}

impl From<&[u8]> for RGB {
    fn from(v: &[u8]) -> Self {
        if v.len() == 3 {
            return RGB {
                r: v[0],
                g: v[1],
                b: v[2],
            };
        } else if v.len() == 1 {
            return RGB {
                r: v[0],
                g: v[0],
                b: v[0],
            };
        }
        panic!("Slice must have 1 or 3 values for rgb conversion.");
    }
}

/// Trait representing an image.
pub trait Image {
    /// Returns the width of the image.
    fn width(&self) -> u32;
    /// Returns the height of the image.
    fn height(&self) -> u32;
    /// Returns a specific pixel's value. The x must be less then width, y less than height.
    fn pixel(&self, x: u32, y: u32) -> RGB;
}

/// A view into a consecutive slice of pixel-esque things, like `image::ImageBuffer::as_raw`
pub struct ImageBufferView<'a, Container: ?Sized, const PIXEL_SIZE: usize> {
    data: &'a Container,
    width: u32,
    height: u32,
}

impl<'a, Container: ?Sized, const PIXEL_SIZE: usize> ImageBufferView<'a, Container, PIXEL_SIZE> {
    pub fn from_raw_ref(
        width: u32,
        height: u32,
        data: &'a Container,
    ) -> ImageBufferView<'a, Container, PIXEL_SIZE> {
        ImageBufferView::<'a, Container, PIXEL_SIZE> {
            data,
            width,
            height,
        }
    }
}

impl<'a, Container: ?Sized, const PIXEL_SIZE: usize> Image
    for ImageBufferView<'a, Container, PIXEL_SIZE>
where
    Container: std::ops::Index<usize> + std::ops::Index<std::ops::Range<usize>>,
    <Container as std::ops::Index<usize>>::Output: Sized,
    RGB: From<&'a <Container as std::ops::Index<std::ops::Range<usize>>>::Output>,
{
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn pixel(&self, x: u32, y: u32) -> RGB {
        assert!(x < self.width(), "requested x coordinate exceeds width");
        assert!(y < self.height(), "requested y coordinate exceeds height");
        let s = (y * self.width + x) as usize * PIXEL_SIZE;
        let e = (y * self.width + x + 1) as usize * PIXEL_SIZE;
        self.data[s..e].into()
    }
}

/// Interpret a byte slice as an rgb image of certain size.
pub fn image_buffer_view_rgb<'a>(
    width: u32,
    height: u32,
    data: &'a [u8],
) -> ImageBufferView<'a, [u8], 3> {
    ImageBufferView::<'a, [u8], 3> {
        data,
        width,
        height,
    }
}
