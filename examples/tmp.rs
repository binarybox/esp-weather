use embedded_graphics::{
    mono_font::{ascii::FONT_6X9, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle},
    text::Text,
};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};
use epd_waveshare::color::TriColor;

fn main() -> Result<(), core::convert::Infallible> {
    let mut display = SimulatorDisplay::new(Size::new(128, 64));

    let line_style = PrimitiveStyle::with_stroke(TriColor::Black, 1);
    let text_style = MonoTextStyle::new(&FONT_6X9, TriColor::Chromatic);

    Circle::new(Point::new(72, 8), 48)
        .into_styled(line_style)
        .draw(&mut display)?;

    Line::new(Point::new(48, 16), Point::new(8, 16))
        .into_styled(line_style)
        .draw(&mut display)?;

    Line::new(Point::new(48, 16), Point::new(64, 32))
        .into_styled(line_style)
        .draw(&mut display)?;

    Rectangle::new(Point::new(79, 15), Size::new(34, 34))
        .into_styled(line_style)
        .draw(&mut display)?;

    Text::new("Hello World!", Point::new(5, 5), text_style).draw(&mut display)?;

    let output_settings = OutputSettingsBuilder::new().scale(4).build();
    Window::new("Hello World", &output_settings).show_static(&display);

    Ok(())
}
