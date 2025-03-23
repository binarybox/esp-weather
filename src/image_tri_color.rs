use embedded_graphics::{
    image::ImageDrawable,
    prelude::{OriginDimensions, Point},
    primitives::Rectangle,
};
use epd_waveshare::color::TriColor;

pub struct ImageTriColor {
    pub data: Vec<u8>,
    pub width: u32,
    pub color: TriColor,
    pub background: TriColor,
    pub point: Point,
}

pub struct ContiguousPixelsTriColor<'a> {
    iter: core::slice::Iter<'a, u8>,
    color: TriColor,
    background: TriColor,
    offset: usize,
    current: Option<&'a u8>,
}

impl<'a> ContiguousPixelsTriColor<'a> {
    fn new(image: &'a ImageTriColor) -> Self {
        let mut iter = image.data.iter();
        let current = iter.next();
        Self {
            color: image.color,
            background: image.background,
            iter,
            current,
            offset: 0,
        }
    }
}
impl Iterator for ContiguousPixelsTriColor<'_> {
    type Item = TriColor;
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == 8 {
            self.offset = 0;
            self.current = self.iter.next();
        };
        if let Some(value) = self.current {
            let color = if (value >> (7 - self.offset)) & 1 == 1 {
                Some(self.background)
            } else {
                Some(self.color)
            };
            self.offset += 1;
            color
        } else {
            None
        }
    }
}

impl OriginDimensions for ImageTriColor {
    fn size(&self) -> embedded_graphics::prelude::Size {
        embedded_graphics::prelude::Size::new(self.width, (self.data.len() * 8) as u32 / self.width)
    }
}

impl ImageDrawable for ImageTriColor {
    type Color = TriColor;
    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: embedded_graphics::prelude::DrawTarget<Color = Self::Color>,
    {
        target.fill_contiguous(
            &Rectangle::new(self.point, self.size()),
            ContiguousPixelsTriColor::new(self),
        )
    }
    fn draw_sub_image<D>(
        &self,
        _target: &mut D,
        _area: &embedded_graphics::primitives::Rectangle,
    ) -> Result<(), D::Error>
    where
        D: embedded_graphics::prelude::DrawTarget<Color = Self::Color>,
    {
        panic!("not implemented")
    }
}
