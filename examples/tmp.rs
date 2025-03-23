use embedded_graphics::{
    image::{Image, ImageRaw, ImageRawBE},
    mono_font::{ascii::FONT_6X9, MonoTextStyle},
    pixelcolor::{
        raw::{BigEndian, LittleEndian},
        BinaryColor, Rgb565, Rgb888,
    },
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle, StyledDrawable},
    text::Text,
};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};
use epd_waveshare::color::TriColor;

fn convert_rgb565_to_binary(rgb565_data: &[u8]) -> Vec<u8> {
    let mut binary_data = Vec::new();

    for chunk in rgb565_data.chunks(2 * 8) {
        let mut byte = 0u8;
        for (index, pixel) in chunk.chunks(2).enumerate() {
            let pixel = ((pixel[0] as u16) << 8) | (pixel[1] as u16);
            let r = (pixel >> 11) & 0x1F;
            let g = (pixel >> 5) & 0x3F;
            let b = pixel & 0x1F;

            let luminance = (r as u32 * 30 + g as u32 * 59 + b as u32 * 11) / 100;

            if luminance > 30 {
                byte |= 1 << index;
            };
        }
        binary_data.push(byte);
    }

    binary_data.reverse(); // this must be done otherwise the images are weard

    binary_data
}

#[derive(PartialEq, Clone, Copy)]
struct TriColorRed(bool);

impl TriColorRed {
    fn color(&self) -> u8 {
        if self.0 {
            1
        } else {
            0
        }
    }
}

fn main() -> Result<(), core::convert::Infallible> {
    let mut display = SimulatorDisplay::<Rgb565>::new(Size::new(128, 64));
    let icon = embedded_weather_icons::wi_rain_mix_32x32().unwrap();

    let mut image = icon.image_data().to_vec();
    image.reverse();

    let raw_image = ImageRaw::<Rgb565>::new(&image, icon.width());
    // let line_style = PrimitiveStyle::with_stroke(TriColor::Black, 1);
    // let text_style = MonoTextStyle::new(&FONT_6X9, TriColor::Chromatic);
    Image::new(&raw_image, Point::new(0, 0))
        .draw(&mut display)
        .unwrap();

    // Raw big endian image data for demonstration purposes. A real image would likely be much
    // larger.
    // let data = [
    //     0x00, 0x00, 0xF8, 0x00, 0x07, 0xE0, 0xFF, 0xE0, //
    //     0x00, 0x1F, 0x07, 0xFF, 0xF8, 0x1F, 0xFF, 0xFF, //
    // ];

    // // Create a raw image instance. Other image formats will require different code to load them.
    // // All code after loading is the same for any image format.
    // let raw: ImageRawBE<BinaryColor> = ImageRaw::new(&data, 4);

    // // Create an `Image` object to position the image at `Point::zero()`.
    // let image = Image::new(&raw, Point::zero());
    // image.draw(&mut display).unwrap();
    // Circle::new(Point::new(72, 8), 48)
    //     .into_styled(line_style)
    //     .draw(&mut display)?;

    // Line::new(Point::new(48, 16), Point::new(8, 16))
    //     .into_styled(line_style)
    //     .draw(&mut display)?;

    // Line::new(Point::new(48, 16), Point::new(64, 32))
    //     .into_styled(line_style)
    //     .draw(&mut display)?;

    // Rectangle::new(Point::new(79, 15), Size::new(34, 34))
    //     .into_styled(line_style)
    //     .draw(&mut display)?;

    // Text::new("Hello World!", Point::new(5, 5), text_style).draw(&mut display)?;

    let output_settings = OutputSettingsBuilder::new().build();
    Window::new("Hello World", &output_settings).show_static(&display);

    Ok(())
}
