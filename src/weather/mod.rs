use std::fmt::Debug;

use chrono::{NaiveDate, NaiveTime};
use serde::Deserialize;

use epd_waveshare::epd7in5b_v2::Display7in5 as Display;
use u8g2_fonts::fonts::{u8g2_font_helvB10_tr, u8g2_font_helvR08_tr};

use crate::{
    constants::{HEIGHT, SECTION_WIDTH},
    text::{draw_weather_icon, write_centered_text, write_labeld_text},
};

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "String")]
pub struct Date(NaiveDate);
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
#[derive(Deserialize, Clone)]
#[serde(try_from = "String")]
pub struct Temperature(i32);
impl Temperature {
    pub fn value(&self) -> i32 {
        self.0
    }
}
impl TryFrom<String> for Temperature {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Temperature(value.parse::<i32>()?))
    }
}
impl Debug for Temperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{} C", self.0).as_str())
    }
}

#[derive(Deserialize, Clone)]
#[serde(try_from = "String")]
pub struct SunHour(f32);
impl SunHour {
    pub fn value(&self) -> f32 {
        self.0
    }
}

impl Debug for SunHour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hours = self.0.round() as u32;
        let minutes = (self.0.fract() * 60.0) as u32;
        f.write_str(format!("{} hour {} minutes", hours, minutes).as_str())
    }
}

impl TryFrom<String> for SunHour {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(value.parse::<f32>()?))
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "String")]
pub struct Time(pub NaiveTime);
impl Time {
    pub fn value(&self) -> NaiveTime {
        self.0
    }
}
#[derive(Debug, thiserror::Error)]
enum TimeError {
    #[error("could not be parsed")]
    NotParsed,
}
impl TryFrom<String> for Time {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let time = value.parse::<u32>()?;
        let time = NaiveTime::from_hms_opt(time / 100, 0, 0)
            .ok_or(anyhow::Error::new(TimeError::NotParsed))?;
        Ok(Time(time))
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "String")]
pub struct DayTime(pub NaiveTime);
impl DayTime {
    pub fn value(&self) -> NaiveTime {
        self.0
    }
}
impl TryFrom<String> for DayTime {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(DayTime(NaiveTime::parse_from_str(
            value.as_str(),
            "%H:%M %p",
        )?))
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "String")]
pub struct WeatherCode(u32);
impl WeatherCode {
    pub fn value(&self) -> u32 {
        self.0
    }
    pub fn to_day_icon(&self) -> Option<u8> {
        match self.0 {
            284 => Some(50),
            113 => Some(51),
            116 | 185 => Some(52),
            119 | 122 => Some(53),
            263 | 176 => Some(54),
            293..320 | 353..377 | 386 | 389 | 266 | 281 => Some(55),
            323..350 | 179 | 182 | 392 | 395 => Some(56),
            200 => Some(57),
            230 => Some(58),
            143 | 248 | 260 => Some(59),
            227 => Some(60),
            _ => None,
        }
    }
    pub fn to_night_icon(&self) -> Option<u8> {
        match self.0 {
            284 => Some(50),
            113 => Some(41),
            116 | 185 => Some(52),
            119 | 122 => Some(53),
            263 | 176 => Some(54),
            293..320 | 353..377 | 386 | 389 | 266 | 281 => Some(55),
            323..350 | 179 | 182 | 392 | 395 => Some(56),
            200 => Some(57),
            230 => Some(58),
            143 | 248 | 260 => Some(59),
            227 => Some(60),
            _ => None,
        }
    }
}

impl TryFrom<String> for WeatherCode {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(value.parse::<u32>()?))
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

#[derive(Deserialize, Debug)]
pub struct WeatherForecast {
    pub weather: Vec<WeatherDay>,
}
#[derive(Deserialize, Debug, Clone)]
pub struct WeatherDay {
    #[serde(rename(deserialize = "avgtempC"))]
    pub average_temperature: Temperature,
    #[serde(rename(deserialize = "maxtempC"))]
    pub maximum_temperature: Temperature,
    #[serde(rename(deserialize = "mintempC"))]
    pub minimum_temperature: Temperature,
    #[serde(rename(deserialize = "sunHour"))]
    pub sun_hour: SunHour,
    #[serde(rename(deserialize = "date"))]
    pub date: Date,
    pub hourly: Vec<WeatherHour>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WeatherHour {
    #[serde(rename(deserialize = "chanceofrain"))]
    pub chance_of_rain: Factor,
    #[serde(rename(deserialize = "tempC"))]
    pub temperature: Temperature,
    #[serde(rename(deserialize = "weatherCode"))]
    pub weather_code: WeatherCode,
    #[serde(rename(deserialize = "time"))]
    pub time: Time,
    // pub astronomy: Vec<Astronomy>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Astronomy {
    #[serde(rename(deserialize = "sunrise"))]
    pub sunrise: DayTime,
    #[serde(rename(deserialize = "sunset"))]
    pub sunset: DayTime,
}

pub fn write_single_weather(
    offset: usize,
    title: &str,
    display: &mut Display,
    weather: &WeatherForecast,
) {
    let column_width = SECTION_WIDTH - 16;
    let x = (SECTION_WIDTH * (offset as i32)) + SECTION_WIDTH / 2;
    write_centered_text::<u8g2_font_helvB10_tr>(display, x, HEIGHT + 10, title);

    let mut day = weather.weather[offset].clone();

    write_centered_text::<u8g2_font_helvR08_tr>(
        display,
        x,
        HEIGHT + 25,
        format!("{}", day.date.value().format("%e. %b %y")).as_str(),
    );

    write_labeld_text(
        display,
        x,
        HEIGHT + 50,
        column_width,
        "average temperature",
        format!("{:?}", day.average_temperature).as_str(),
    );
    write_labeld_text(
        display,
        x,
        HEIGHT + 65,
        column_width,
        "maximum temperature",
        format!("{:?}", day.maximum_temperature).as_str(),
    );
    write_labeld_text(
        display,
        x,
        HEIGHT + 80,
        column_width,
        "minimum temperature",
        format!("{:?}", day.minimum_temperature).as_str(),
    );
    write_labeld_text(
        display,
        x,
        HEIGHT + 95,
        column_width,
        "sun for",
        format!("{:?}", day.sun_hour).as_str(),
    );

    day.hourly.sort_by(|a, b| a.time.0.cmp(&b.time.0));
    for (idx, hour) in day.hourly.into_iter().enumerate() {
        let height_offset = idx as i32 * (160 - 120);
        let start_time = hour.time.value();
        let end_time = start_time + chrono::TimeDelta::hours(3);
        write_centered_text::<u8g2_font_helvR08_tr>(
            display,
            x,
            HEIGHT + 115 + height_offset,
            format!(
                "{} - {}",
                start_time.format("%H:%M"),
                end_time.format("%H:%M"),
            )
            .as_str(),
        );

        draw_weather_icon(
            display,
            x + (column_width / 2) - 10,
            HEIGHT + 115 + height_offset,
            &hour,
        );

        write_labeld_text(
            display,
            x,
            HEIGHT + 130 + height_offset,
            column_width,
            "temperature",
            format!("{:?}", hour.temperature).as_str(),
        );
        write_labeld_text(
            display,
            x,
            HEIGHT + 140 + height_offset,
            column_width,
            "rain chance",
            format!("{:?}", hour.chance_of_rain).as_str(),
        );
    }
}
