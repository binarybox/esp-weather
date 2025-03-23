use std::fmt::Debug;

use chrono::{NaiveDate, NaiveDateTime};

use embedded_graphics::{image::ImageDrawable, prelude::Point};
use epd_waveshare::color::TriColor;
use serde::Deserialize;

use crate::{icons::convert_rgb565_to_binary, image_tri_color::ImageTriColor};

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "String")]
pub struct Date(pub NaiveDate);
impl Date {
    pub fn value(&self) -> NaiveDate {
        self.0
    }
}
impl TryFrom<String> for Date {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Date(NaiveDate::parse_from_str(&value, "%Y-%m-%d")?))
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "String")]
pub struct DateTime(pub NaiveDateTime);
impl DateTime {
    pub fn value(&self) -> NaiveDateTime {
        self.0
    }
}

impl TryFrom<String> for DateTime {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(DateTime(
            NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M").unwrap(),
        ))
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "u32")]
pub struct WeatherCode(u32);
impl WeatherCode {
    pub fn value(&self) -> u32 {
        self.0
    }
    pub fn to_clouded_icon(&self) -> (Vec<u8>, u32) {
        let image = match self.0 {
            3 | 4 => embedded_weather_icons::wi_cloudy_32x32(),
            45 | 48 => embedded_weather_icons::wi_fog_32x32(),
            51..56 => embedded_weather_icons::wi_raindrops_32x32(),
            61..66 => embedded_weather_icons::wi_rain_32x32(),
            66 | 67 => embedded_weather_icons::wi_rain_mix_32x32(),
            71..77 => embedded_weather_icons::wi_snow_32x32(),
            80..82 => embedded_weather_icons::wi_showers_32x32(),
            85 | 86 => embedded_weather_icons::wi_snow_32x32(),
            95..100 => embedded_weather_icons::wi_thunderstorm_32x32(),

            _ => return (Vec::new(), 0),
        }
        .unwrap();

        (convert_rgb565_to_binary(image.image_data()), image.width())
    }
    pub fn to_day_icon(&self) -> (Vec<u8>, u32) {
        let image = match self.0 {
            0..3 => embedded_weather_icons::wi_day_sunny_32x32(),
            3 | 4 => embedded_weather_icons::wi_day_cloudy_32x32(),
            45 | 48 => embedded_weather_icons::wi_day_fog_32x32(),
            51..56 => embedded_weather_icons::wi_raindrops_32x32(),
            61..66 => embedded_weather_icons::wi_day_rain_32x32(),
            66 | 67 => embedded_weather_icons::wi_day_rain_mix_32x32(),
            71..77 => embedded_weather_icons::wi_day_snow_32x32(),
            80..82 => embedded_weather_icons::wi_day_showers_32x32(),
            85 | 86 => embedded_weather_icons::wi_day_snow_32x32(),
            95..100 => embedded_weather_icons::wi_day_thunderstorm_32x32(),

            _ => return (Vec::new(), 0),
        }
        .unwrap();

        (convert_rgb565_to_binary(image.image_data()), image.width())
    }
    pub fn to_night_icon(&self) -> (Vec<u8>, u32) {
        let image = match self.0 {
            0..3 => embedded_weather_icons::wi_night_clear_32x32(),
            3..9 => embedded_weather_icons::wi_night_cloudy_32x32(),
            45 | 48 => embedded_weather_icons::wi_night_fog_32x32(),
            51..56 => embedded_weather_icons::wi_raindrops_32x32(),
            61..66 => embedded_weather_icons::wi_night_rain_32x32(),
            66 | 67 => embedded_weather_icons::wi_night_rain_mix_32x32(),
            71..77 => embedded_weather_icons::wi_night_snow_32x32(),
            80..82 => embedded_weather_icons::wi_night_showers_32x32(),
            85 | 86 => embedded_weather_icons::wi_night_snow_32x32(),
            95..100 => embedded_weather_icons::wi_night_thunderstorm_32x32(),
            _ => return (Vec::new(), 0),
        }
        .unwrap();

        (convert_rgb565_to_binary(image.image_data()), image.width())
    }

    pub fn draw_icon<Display>(
        &self,
        display: &mut Display,
        x: i32,
        y: i32,
        cloud_coverage: u32,
        is_day: bool,
    ) where
        Display: embedded_graphics::draw_target::DrawTarget<Color = TriColor>,
        <Display as embedded_graphics::draw_target::DrawTarget>::Error: std::fmt::Debug,
    {
        let (icon, width) = if cloud_coverage > 80 {
            self.to_clouded_icon()
        } else if is_day {
            self.to_day_icon()
        } else {
            self.to_night_icon()
        };

        let raw_image = ImageTriColor {
            background: TriColor::White,
            color: TriColor::Black,
            data: icon,
            point: Point::new(x - 16, y),
            width,
        };

        raw_image.draw(display).unwrap();
    }
}

impl TryFrom<u32> for WeatherCode {
    type Error = anyhow::Error;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

#[derive(Deserialize, Clone)]
#[serde(try_from = "String")]
pub struct Factor(f32);

impl Factor {
    pub fn value(&self) -> f32 {
        self.0
    }
}

impl TryFrom<String> for Factor {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(value.parse::<f32>()?))
    }
}

impl Debug for Factor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:.0} %", self.0).as_str())
    }
}

#[derive(Deserialize, Clone)]
#[serde(try_from = "String")]
pub struct Millimeter(f32);

impl Millimeter {
    pub fn value(&self) -> f32 {
        self.0
    }
}

impl TryFrom<String> for Millimeter {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(value.parse::<f32>()?))
    }
}

impl Debug for Millimeter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:.2} mm", self.0).as_str())
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct WeatherForecast {
    pub utc_offset_seconds: u32,
    pub timezone: String,
    pub timezone_abbreviation: String,
    pub hourly: WeatherHourly,
    pub daily: WeatherDaily,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct WeatherHourly {
    pub time: Vec<DateTime>,
    pub temperature_2m: Vec<f32>,
    pub precipitation_probability: Vec<u32>,
    pub precipitation: Vec<f32>,
    pub weather_code: Vec<WeatherCode>,
    pub cloud_cover: Vec<u32>,
}
#[derive(Deserialize, Debug, Clone, Default)]
pub struct WeatherDaily {
    pub time: Vec<Date>,
    pub sunrise: Vec<DateTime>,
    pub sunset: Vec<DateTime>,
}
