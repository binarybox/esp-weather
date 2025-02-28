use chrono::NaiveTime;
use embedded_graphics::prelude::Point;
use epd_waveshare::color::TriColor;
use epd_waveshare::epd7in5b_v2::Display7in5 as Display;
use u8g2_fonts::{
    fonts::{u8g2_font_helvR08_tr, u8g2_font_unifont_t_weather},
    types::{FontColor, HorizontalAlignment, VerticalPosition},
    Font, FontRenderer,
};

use crate::weather::WeatherHour;

pub fn write_centered_text<F: Font>(display: &mut Display, x: i32, y: i32, text: &str) {
    let font = FontRenderer::new::<F>();

    font.render_aligned(
        text,
        Point::new(x, y),
        VerticalPosition::Baseline,
        u8g2_fonts::types::HorizontalAlignment::Center,
        FontColor::Transparent(TriColor::Black),
        display,
    )
    .unwrap();
}

pub fn write_labeld_text(
    display: &mut Display,
    x: i32,
    y: i32,
    width: i32,
    label: &str,
    text: &str,
) {
    let font = FontRenderer::new::<u8g2_font_helvR08_tr>();
    font.render_aligned(
        label,
        Point::new(x - (width / 2), y),
        VerticalPosition::Baseline,
        HorizontalAlignment::Left,
        FontColor::Transparent(TriColor::Black),
        display,
    )
    .unwrap();

    font.render_aligned(
        text,
        Point::new(x + (width / 2), y),
        VerticalPosition::Baseline,
        HorizontalAlignment::Right,
        FontColor::Transparent(TriColor::Chromatic),
        display,
    )
    .unwrap();
}

pub fn draw_weather_icon(display: &mut Display, x: i32, y: i32, hour: &WeatherHour) {
    if hour.time.0 > NaiveTime::from_hms_opt(8, 0, 0).unwrap()
        && hour.time.0 < NaiveTime::from_hms_opt(20, 0, 0).unwrap()
    {
        if let Some(code) = hour.weather_code.to_day_icon() {
            let font = FontRenderer::new::<u8g2_font_unifont_t_weather>();

            font.render_aligned(
                String::from_utf8([code].to_vec()).unwrap().as_str(),
                Point::new(x, y),
                VerticalPosition::Baseline,
                u8g2_fonts::types::HorizontalAlignment::Center,
                FontColor::Transparent(TriColor::Black),
                display,
            )
            .unwrap();
        }
    } else if let Some(code) = hour.weather_code.to_night_icon() {
        let font = FontRenderer::new::<u8g2_font_unifont_t_weather>();

        font.render_aligned(
            String::from_utf8([code].to_vec()).unwrap().as_str(),
            Point::new(x, y),
            VerticalPosition::Baseline,
            u8g2_fonts::types::HorizontalAlignment::Center,
            FontColor::Transparent(TriColor::Black),
            display,
        )
        .unwrap();
    }
}
