use std::time::Duration;

use chrono::{Datelike, Days, NaiveTime, Timelike};
use embedded_graphics::{
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};
use epd_waveshare::{color::TriColor, prelude::WaveshareDisplay};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        delay::Delay,
        gpio::{AnyInputPin, IOPin, InputPin, OutputPin, PinDriver},
        modem::WifiModem,
        prelude::Peripherals,
        spi::config::{Config, DriverConfig},
        units::Hertz,
    },
    http::client::EspHttpConnection,
    sntp::{EspSntp, SyncStatus},
};
use esp_weather::{
    constants::{DISPLAY_HEIGHT, DISPLAY_WIDTH, SECTION_WIDTH},
    text::{draw_weather_icon, write_centered_text},
    weather::{write_single_weather, WeatherForecast},
    wifi,
};

const SSID: &str = env!("SSID");
const PASS: &str = env!("PASS");

use epd_waveshare::epd7in5b_v2::Display7in5 as Display;
use epd_waveshare::epd7in5b_v2::Epd7in5 as Epd;
use u8g2_fonts::{
    fonts::{
        u8g2_font_helvB10_tr, u8g2_font_helvR08_tf, u8g2_font_helvR08_tr,
        u8g2_font_unifont_t_weather,
    },
    types::{FontColor, VerticalPosition},
    FontRenderer,
};

const SPI_FREQUENCY: u32 = 5_000_000;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let config = esp_idf_svc::sys::esp_vfs_eventfd_config_t { max_fds: 1 };
    esp_idf_svc::sys::esp! { unsafe { esp_idf_svc::sys::esp_vfs_eventfd_register(&config) } }
        .unwrap();

    log::info!("STARTED");

    let sysloop = EspSystemEventLoop::take().unwrap();

    let modem = unsafe { WifiModem::new() };

    let (_wifi, _) = wifi::wifi(modem, sysloop, SSID, PASS).unwrap();

    let peripherals = Peripherals::take().unwrap();
    let ntp = Box::new(EspSntp::new_default().unwrap());

    while ntp.get_sync_status() != SyncStatus::Completed {
        std::thread::sleep(Duration::from_millis(200));
    }

    // setup display
    let spi = peripherals.spi2;
    let sclk = peripherals.pins.gpio21.downgrade_output();
    let sdin = peripherals.pins.gpio19.downgrade_output();
    let cs = peripherals.pins.gpio18.downgrade_output();

    let driver_config = DriverConfig::default();
    let config = Config {
        baudrate: Hertz(SPI_FREQUENCY),
        // bit_order: BitOrder::MsbFirst,
        // write_only: true,
        ..Default::default()
    };

    let spi_driver =
        esp_idf_svc::hal::spi::SpiDriver::new(spi, sclk, sdin, AnyInputPin::none(), &driver_config)
            .unwrap();

    let mut spi =
        esp_idf_svc::hal::spi::SpiDeviceDriver::new(spi_driver, Some(cs), &config).unwrap();

    let mut pwr = PinDriver::input_output(peripherals.pins.gpio0).unwrap();
    pwr.set_high().unwrap();

    let busy = PinDriver::input(peripherals.pins.gpio1.downgrade_input()).unwrap();
    let rst = PinDriver::input_output(peripherals.pins.gpio2.downgrade()).unwrap();
    let dc = PinDriver::input_output(peripherals.pins.gpio3.downgrade()).unwrap();

    let mut delay = Delay::new_default();

    let mut epd = Epd::new(&mut spi, busy, dc, rst, &mut delay, None).unwrap();

    // setup epd done

    let config = esp_idf_svc::http::client::Configuration {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    };
    let connection = esp_idf_svc::http::client::EspHttpConnection::new(&config).unwrap();

    let mut client = embedded_svc::http::client::Client::wrap(connection);

    let mut display = Box::new(Display::default());
    display.clear(TriColor::White).unwrap();
    loop {
        match request_weather(&mut client) {
            Err(err) => {
                {
                    // center the error message
                    let error = err.to_string();
                    write_centered_text::<u8g2_font_helvB10_tr>(
                        display.as_mut(),
                        DISPLAY_WIDTH as i32 / 2,
                        DISPLAY_HEIGHT as i32 / 2,
                        error.as_str(),
                    );
                    log::error!("error occured");
                    std::thread::sleep(Duration::from_secs(10));
                    continue;
                }
                // log::error!("error: {err:?}")
            }
            Ok(mut weather) => {
                // write the day
                let today = chrono::Local::now();
                let today = format!("{}", today.format("%e. %b %y"));
                write_centered_text::<u8g2_font_helvB10_tr>(
                    display.as_mut(),
                    DISPLAY_WIDTH as i32 / 2,
                    30,
                    today.as_str(),
                );

                weather
                    .weather
                    .sort_by(|a, b| a.date.value().cmp(&b.date.value()));

                write_single_weather(
                    0,
                    &chrono::Local::now().weekday().to_string(),
                    display.as_mut(),
                    &weather,
                );

                write_single_weather(
                    1,
                    &chrono::Local::now().weekday().succ().to_string(),
                    display.as_mut(),
                    &weather,
                );

                write_single_weather(
                    2,
                    &chrono::Local::now().weekday().succ().succ().to_string(),
                    display.as_mut(),
                    &weather,
                );

                let mut last_rain_point = Point::zero();
                let mut last_temp_point = Point::zero();

                for i in 0..3 {
                    let mut day = weather.weather[i].clone();
                    day.hourly.sort_by(|a, b| a.time.0.cmp(&b.time.0));
                    let font = FontRenderer::new::<u8g2_font_helvR08_tr>();

                    FontRenderer::new::<u8g2_font_unifont_t_weather>()
                        .render_aligned(
                            String::from_utf8([49].to_vec()).unwrap().as_str(),
                            Point::new(0, DISPLAY_HEIGHT as i32 - 160),
                            VerticalPosition::Center,
                            u8g2_fonts::types::HorizontalAlignment::Left,
                            FontColor::Transparent(TriColor::Black),
                            display.as_mut(),
                        )
                        .unwrap();
                    font.render_aligned(
                        "C",
                        Point::new(18, DISPLAY_HEIGHT as i32 - 165),
                        VerticalPosition::Center,
                        u8g2_fonts::types::HorizontalAlignment::Left,
                        FontColor::Transparent(TriColor::Black),
                        display.as_mut(),
                    )
                    .unwrap();

                    // temperature graph
                    for (idx_day, day) in day.hourly.iter().enumerate() {
                        let temperature = day.temperature.value() + 10;

                        // temperature
                        let current_point = Point::new(
                            (i as i32 * SECTION_WIDTH) + (idx_day as i32 * 33) + 33,
                            DISPLAY_HEIGHT as i32 - 135 - temperature,
                        );

                        let text_position = current_point - Point::new(0, 10);

                        let font = FontRenderer::new::<u8g2_font_helvR08_tf>();

                        font.render_aligned(
                            format!("{:?}", day.temperature.value()).as_str(),
                            text_position,
                            VerticalPosition::Center,
                            u8g2_fonts::types::HorizontalAlignment::Center,
                            FontColor::Transparent(TriColor::Black),
                            display.as_mut(),
                        )
                        .unwrap();

                        if last_temp_point == Point::zero() {
                            last_temp_point = current_point;
                            continue;
                        }

                        Line::new(last_temp_point, current_point)
                            .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
                            .draw(display.as_mut())
                            .unwrap();

                        last_temp_point = current_point;
                    }

                    // rain

                    FontRenderer::new::<u8g2_font_unifont_t_weather>()
                        .render_aligned(
                            String::from_utf8([55].to_vec()).unwrap().as_str(),
                            Point::new(0, DISPLAY_HEIGHT as i32 - 110),
                            VerticalPosition::Center,
                            u8g2_fonts::types::HorizontalAlignment::Left,
                            FontColor::Transparent(TriColor::Chromatic),
                            display.as_mut(),
                        )
                        .unwrap();
                    font.render_aligned(
                        "%",
                        Point::new(18, DISPLAY_HEIGHT as i32 - 115),
                        VerticalPosition::Center,
                        u8g2_fonts::types::HorizontalAlignment::Left,
                        FontColor::Transparent(TriColor::Chromatic),
                        display.as_mut(),
                    )
                    .unwrap();

                    FontRenderer::new::<u8g2_font_unifont_t_weather>()
                        .render_aligned(
                            String::from_utf8([55].to_vec()).unwrap().as_str(),
                            Point::new(0, DISPLAY_HEIGHT as i32 - 90),
                            VerticalPosition::Center,
                            u8g2_fonts::types::HorizontalAlignment::Left,
                            FontColor::Transparent(TriColor::Black),
                            display.as_mut(),
                        )
                        .unwrap();
                    font.render_aligned(
                        "mm",
                        Point::new(18, DISPLAY_HEIGHT as i32 - 95),
                        VerticalPosition::Center,
                        u8g2_fonts::types::HorizontalAlignment::Left,
                        FontColor::Transparent(TriColor::Black),
                        display.as_mut(),
                    )
                    .unwrap();

                    // rain graph
                    for (idx_day, day) in day.hourly.iter().enumerate() {
                        if day.precipaction.value() <= 0.0 {
                            continue;
                        }
                        let rain_precipation = (day.precipaction.value() * 10.0) as i32;

                        let x = (i as i32 * SECTION_WIDTH) + (idx_day as i32 * 33) + 33;
                        let y = DISPLAY_HEIGHT as i32 - 75;
                        // rain probability
                        let current_point = Point::new(x, y - rain_precipation);

                        Line::new(Point::new(x, y), current_point)
                            .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 34))
                            .draw(display.as_mut())
                            .unwrap();

                        font.render_aligned(
                            format!("{:?}", day.precipaction.value()).as_str(),
                            Point::new(x, y - rain_precipation - 10),
                            VerticalPosition::Center,
                            u8g2_fonts::types::HorizontalAlignment::Center,
                            FontColor::Transparent(TriColor::Black),
                            display.as_mut(),
                        )
                        .unwrap();
                    }
                    for (idx_day, day) in day.hourly.iter().enumerate() {
                        let rain_probability = (day.chance_of_rain.value() as i32) / 2;
                        if last_rain_point == Point::zero() {
                            last_rain_point = Point::new(
                                (i as i32 * SECTION_WIDTH) + (idx_day as i32 * 33) + 33,
                                DISPLAY_HEIGHT as i32 - 75 - rain_probability,
                            );
                            continue;
                        }
                        // rain probability
                        let current_point = Point::new(
                            (i as i32 * SECTION_WIDTH) + (idx_day as i32 * 33) + 33,
                            DISPLAY_HEIGHT as i32 - 75 - rain_probability,
                        );

                        Line::new(last_rain_point, current_point)
                            .into_styled(PrimitiveStyle::with_stroke(TriColor::Chromatic, 1))
                            .draw(display.as_mut())
                            .unwrap();

                        last_rain_point = current_point;
                    }

                    // sunrist and sunset
                    let astronomy = day.astronomy.first().unwrap();
                    let sunrise =
                        astronomy.sunrise.0.hour() * 11 + (astronomy.sunrise.0.minute() / 11) + 16;
                    let sunset =
                        astronomy.sunset.0.hour() * 11 + (astronomy.sunset.0.minute() / 11) + 16;
                    let sunrise = Point::new(
                        sunrise as i32 + (i as i32 * SECTION_WIDTH),
                        DISPLAY_HEIGHT as i32 - 50,
                    );
                    let sunset = Point::new(
                        sunset as i32 + (i as i32 * SECTION_WIDTH),
                        DISPLAY_HEIGHT as i32 - 50,
                    );

                    Line::new(sunrise, sunset)
                        .into_styled(PrimitiveStyle::with_stroke(TriColor::Chromatic, 2))
                        .draw(display.as_mut())
                        .unwrap();

                    for (idx_day, hour) in day.hourly.iter().enumerate() {
                        // hours
                        let text = hour.time.0.hour().to_string();
                        font.render_aligned(
                            text.as_str(),
                            Point::new(
                                (i as i32 * SECTION_WIDTH) + (idx_day as i32 * 33) + 16,
                                DISPLAY_HEIGHT as i32 - 35,
                            ),
                            VerticalPosition::Bottom,
                            u8g2_fonts::types::HorizontalAlignment::Center,
                            FontColor::Transparent(TriColor::Black),
                            display.as_mut(),
                        )
                        .unwrap();

                        draw_weather_icon(
                            display.as_mut(),
                            (i as i32 * SECTION_WIDTH) + (idx_day as i32 * 33) + 16,
                            DISPLAY_HEIGHT as i32 - 10,
                            hour,
                        );
                    }
                }
            }
        };

        epd.update_and_display_frame(&mut spi, display.buffer(), &mut delay)
            .unwrap();

        log::info!("finished drawing");

        epd.sleep(&mut spi, &mut delay).unwrap();

        log::info!("going to sleep for ");

        let tomorrow = chrono::Local::now()
            .checked_add_days(Days::new(1))
            .unwrap()
            .with_time(NaiveTime::default())
            .unwrap();

        let until_tomorrow_time = tomorrow.signed_duration_since(chrono::Local::now());

        log::warn!("sleeping not for {}", until_tomorrow_time.to_string());
        unsafe {
            esp_idf_svc::sys::esp_deep_sleep(until_tomorrow_time.num_microseconds().unwrap() as u64)
        };
    }
}

fn request_weather(
    client: &mut embedded_svc::http::client::Client<EspHttpConnection>,
) -> anyhow::Result<WeatherForecast> {
    // Prepare headers and URL
    let headers = [("Accept", "application/json")];
    let url = format!("https://wttr.in/{}?format=j1", env!("LOCATION"));

    // Send request
    //
    // Note: If you don't want to pass in any headers, you can also use `client.get(url, headers)`.
    log::info!("starting request");
    let request = client.request(embedded_svc::http::Method::Get, url.as_str(), &headers)?;
    let mut response = request.submit()?;

    // Process response
    let status = response.status();
    log::info!("status: {status}");

    let _size = response
        .header("content-length")
        .unwrap_or_default()
        .parse::<usize>()?;

    let mut buffer = Box::new([0u8; 1 << 16]);

    let bytes_read = response.read(buffer.as_mut())?;

    let string = std::str::from_utf8(&buffer[0..bytes_read])?;
    // log::info!("string: {:?}", string);

    let result = serde_json::from_str::<WeatherForecast>(string)?;

    Ok(result)
}
