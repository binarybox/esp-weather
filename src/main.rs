use chrono::Timelike;
#[cfg(target_os = "espidf")]
use chrono::{Days, NaiveTime};
use embedded_graphics::{
    prelude::*,
    primitives::{Line, PrimitiveStyle, StyledDrawable},
};
use epd_waveshare::color::TriColor;
#[cfg(target_os = "espidf")]
use epd_waveshare::{
    epd7in5b_v2::Display7in5 as Display, epd7in5b_v2::Epd7in5 as Epd, prelude::WaveshareDisplay,
};
#[cfg(target_os = "espidf")]
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
    sntp::{EspSntp, SyncStatus},
};
#[cfg(target_os = "espidf")]
use esp_weather::wifi;
use esp_weather::{
    constants::{DISPLAY_HEIGHT, DISPLAY_WIDTH, SECTION_WIDTH},
    weather::WeatherForecast,
};
use smol::Executor;
use std::time::Duration;
use u8g2_fonts::{
    fonts::{
        u8g2_font_helvB10_tr, u8g2_font_helvR08_tf, u8g2_font_helvR08_tr,
        u8g2_font_unifont_t_weather,
    },
    types::{FontColor, VerticalPosition},
    FontRenderer,
};

fn main() {
    #[cfg(target_os = "espidf")]
    {
        // It is necessary to call this function once. Otherwise some patches to the runtime
        // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
        esp_idf_svc::sys::link_patches();

        // Bind the log crate to the ESP Logging facilities
        esp_idf_svc::log::EspLogger::initialize_default();

        let config = esp_idf_svc::sys::esp_vfs_eventfd_config_t { max_fds: 1 };
        esp_idf_svc::sys::esp! { unsafe { esp_idf_svc::sys::esp_vfs_eventfd_register(&config) } }
            .unwrap();
    }

    #[cfg(target_os = "espidf")]
    let mut display = Box::new(Display::default());

    #[cfg(target_os = "linux")]
    tracing_subscriber::fmt().pretty().init();

    #[cfg(target_os = "linux")]
    let mut display = Box::new(
        embedded_graphics_simulator::SimulatorDisplay::<TriColor>::new(Size::new(
            DISPLAY_WIDTH,
            DISPLAY_HEIGHT,
        )),
    );

    display.clear(TriColor::White).unwrap();
    let executor = Executor::new();
    executor
        .spawn(async move {
            loop {
                match request_weather().await {
                    Err(err) => {
                        {
                            // center the error message
                            let error = err.to_string();
                            FontRenderer::new::<u8g2_font_helvB10_tr>()
                                .render_aligned(
                                    error.as_str(),
                                    Point::new(DISPLAY_WIDTH as i32 / 2, DISPLAY_HEIGHT as i32 / 2),
                                    VerticalPosition::Baseline,
                                    u8g2_fonts::types::HorizontalAlignment::Center,
                                    FontColor::Transparent(TriColor::Black),
                                    display.as_mut(),
                                )
                                .unwrap();
                            log::error!("error occured {}", error);
                            smol::Timer::after(Duration::from_secs(10)).await;
                            continue;
                        }
                        // log::error!("error: {err:?}")
                    }
                    Ok(weather) => {
                        // write the day
                        let today = chrono::Local::now();
                        let today = format!("{}", today.format("%e. %b %y"));
                        FontRenderer::new::<u8g2_font_helvB10_tr>()
                            .render_aligned(
                                today.as_str(),
                                Point::new(DISPLAY_WIDTH as i32 / 2, 30),
                                VerticalPosition::Baseline,
                                u8g2_fonts::types::HorizontalAlignment::Center,
                                FontColor::Transparent(TriColor::Black),
                                display.as_mut(),
                            )
                            .unwrap();

                        // weather
                        //     .weather
                        //     .sort_by(|a, b| a.date.value().cmp(&b.date.value()));

                        for (i, day) in weather.daily.time.iter().enumerate() {
                            let x = (SECTION_WIDTH * (i as i32)) + SECTION_WIDTH / 2;
                            FontRenderer::new::<u8g2_font_helvB10_tr>()
                                .render_aligned(
                                    format!("{}", day.0.format("%A")).as_str(),
                                    Point::new(x, 50 + 10),
                                    VerticalPosition::Baseline,
                                    u8g2_fonts::types::HorizontalAlignment::Center,
                                    FontColor::Transparent(TriColor::Black),
                                    display.as_mut(),
                                )
                                .unwrap();

                            FontRenderer::new::<u8g2_font_helvR08_tr>()
                                .render_aligned(
                                    format!("{}", day.0.format("%e. %b %y")).as_str(),
                                    Point::new(x, 50 + 25),
                                    VerticalPosition::Baseline,
                                    u8g2_fonts::types::HorizontalAlignment::Center,
                                    FontColor::Transparent(TriColor::Black),
                                    display.as_mut(),
                                )
                                .unwrap();
                        }

                        let mut last_rain_point = Point::zero();
                        let mut last_temp_point = Point::zero();

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

                        for (i, temp) in weather.hourly.temperature_2m.into_iter().enumerate() {
                            // temperature graph
                            let temperature = temp as i32 + 10;

                            // temperature
                            let current_point = Point::new(
                                (i as i32 * 10) + 60,
                                DISPLAY_HEIGHT as i32 - 135 - temperature,
                            );

                            if i % 3 == 0 {
                                let text_position = current_point - Point::new(0, 10);
                                let font = FontRenderer::new::<u8g2_font_helvR08_tf>();
                                font.render_aligned(
                                    format!("{}", temp as i32).as_str(),
                                    text_position,
                                    VerticalPosition::Center,
                                    u8g2_fonts::types::HorizontalAlignment::Center,
                                    FontColor::Transparent(TriColor::Black),
                                    display.as_mut(),
                                )
                                .unwrap();
                            }

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
                        let mut last_precipitation = 0.0f32;
                        let mut last_drawn = false;
                        for (i, precipitation) in
                            weather.hourly.precipitation.into_iter().enumerate()
                        {
                            // rain
                            let x = (i as i32 * 10) + 60;
                            let y = DISPLAY_HEIGHT as i32 - 75;
                            let rain_precipation = (precipitation * 10.0) as i32;
                            if last_precipitation != 0.0 && !last_drawn {
                                let current_precipation = last_precipitation.max(precipitation);
                                let x = if precipitation == 0.0 { x - 10 } else { x - 5 };
                                font.render_aligned(
                                    format!("{:?}", current_precipation).as_str(),
                                    Point::new(x, y - (current_precipation as i32 * 10) - 10),
                                    VerticalPosition::Center,
                                    u8g2_fonts::types::HorizontalAlignment::Center,
                                    FontColor::Transparent(TriColor::Black),
                                    display.as_mut(),
                                )
                                .unwrap();
                                last_drawn = true;
                            } else {
                                last_drawn = false;
                            }
                            last_precipitation = precipitation;

                            // rain graph
                            if precipitation <= 0.0 {
                                continue;
                            }

                            // rain probability
                            let current_point = Point::new(x, y - rain_precipation);

                            Line::new(Point::new(x, y), current_point)
                                .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 10))
                                .draw(display.as_mut())
                                .unwrap();
                        }
                        for (i, precipitation_probability) in weather
                            .hourly
                            .precipitation_probability
                            .into_iter()
                            .enumerate()
                        {
                            let rain_probability = (precipitation_probability as i32) / 2;
                            let x = (i as i32 * 10) + 60;
                            let y = DISPLAY_HEIGHT as i32 - 75 - rain_probability;
                            if last_rain_point == Point::zero() {
                                last_rain_point = Point::new(x, y);
                                continue;
                            }
                            // rain probability
                            let current_point = Point::new(x, y);

                            Line::new(last_rain_point, current_point)
                                .into_styled(PrimitiveStyle::with_stroke(TriColor::Chromatic, 1))
                                .draw(display.as_mut())
                                .unwrap();

                            last_rain_point = current_point;
                        }
                        for (i, (sunrise, sunset)) in (weather.daily.sunrise)
                            .iter()
                            .zip(&(weather.daily).sunset)
                            .enumerate()
                        {
                            // sunrist and sunset
                            let sunrise = sunrise.0.hour() * 10 + (sunrise.0.minute() / 10);
                            let sunset = sunset.0.hour() * 10 + (sunset.0.minute() / 10);

                            let offset = (i * 10 * 24) as i32 + (60 - 5);

                            let sunrise_point =
                                Point::new(sunrise as i32 + offset, DISPLAY_HEIGHT as i32 - 60);
                            let sunset_point =
                                Point::new(sunset as i32 + offset, DISPLAY_HEIGHT as i32 - 60);

                            Line::new(sunrise_point, sunset_point)
                                .draw_styled(
                                    &PrimitiveStyle::with_stroke(TriColor::Chromatic, 3),
                                    display.as_mut(),
                                )
                                .unwrap();

                            // Line::new(sunrise_point, sunset_point)
                            //     .into_styled(PrimitiveStyle::with_stroke(TriColor::Chromatic, 2))
                            //     .draw(display.as_mut())
                            //     .unwrap();
                        }

                        for (i, time) in weather.hourly.time.iter().enumerate() {
                            // hours
                            if i % 2 == 0 {
                                let text = time.0.hour().to_string();
                                font.render_aligned(
                                    text.as_str(),
                                    Point::new(
                                        (i as i32 * 10) + (60 - 5),
                                        DISPLAY_HEIGHT as i32 - 35,
                                    ),
                                    VerticalPosition::Bottom,
                                    u8g2_fonts::types::HorizontalAlignment::Center,
                                    FontColor::Transparent(TriColor::Black),
                                    display.as_mut(),
                                )
                                .unwrap();
                            }
                        }

                        for (i, ((code, time), cloud_coverage)) in weather
                            .hourly
                            .weather_code
                            .iter()
                            .zip(weather.hourly.time)
                            .zip(weather.hourly.cloud_cover)
                            .enumerate()
                        {
                            if i % 3 == 0 {
                                if let Some(((_date, sunrise), sunset)) = weather
                                    .daily
                                    .time
                                    .iter()
                                    .zip(&weather.daily.sunrise)
                                    .zip(&weather.daily.sunset)
                                    .find(|((date, _), _)| time.0.date().eq(&date.0))
                                {
                                    let is_day = time.0 >= sunrise.0 && time.0 <= sunset.0;
                                    code.draw_icon(
                                        display.as_mut(),
                                        (i as i32 * 10) + (60 - 5),
                                        DISPLAY_HEIGHT as i32 - 35,
                                        cloud_coverage,
                                        is_day,
                                    );
                                }
                            }
                        }
                    }
                };

                #[cfg(target_os = "espidf")]
                {
                    const SPI_FREQUENCY: u32 = 5_000_000;

                    let peripherals = Peripherals::take().unwrap();

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

                    let spi_driver = esp_idf_svc::hal::spi::SpiDriver::new(
                        spi,
                        sclk,
                        sdin,
                        AnyInputPin::none(),
                        &driver_config,
                    )
                    .unwrap();

                    let mut spi =
                        esp_idf_svc::hal::spi::SpiDeviceDriver::new(spi_driver, Some(cs), &config)
                            .unwrap();

                    let mut pwr = PinDriver::input_output(peripherals.pins.gpio0).unwrap();
                    pwr.set_high().unwrap();

                    let busy = PinDriver::input(peripherals.pins.gpio1.downgrade_input()).unwrap();
                    let rst = PinDriver::input_output(peripherals.pins.gpio2.downgrade()).unwrap();
                    let dc = PinDriver::input_output(peripherals.pins.gpio3.downgrade()).unwrap();

                    let mut delay = Delay::new_default();

                    println!("pre drawing");
                    let mut epd = Epd::new(&mut spi, busy, dc, rst, &mut delay, None).unwrap();
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

                    log::warn!("sleeping now for {}", until_tomorrow_time.to_string());
                    unsafe {
                        esp_idf_svc::sys::esp_deep_sleep(
                            until_tomorrow_time.num_microseconds().unwrap() as u64,
                        )
                    };
                }
                #[cfg(target_os = "linux")]
                {
                    log::info!("write to display");
                    let output_settings = embedded_graphics_simulator::OutputSettingsBuilder::new()
                        .scale(4)
                        .build();
                    embedded_graphics_simulator::Window::new("Hello World", &output_settings)
                        .show_static(&display);
                    std::thread::sleep(Duration::from_secs(100));
                }
            }
        })
        .detach();

    #[cfg(target_os = "linux")]
    {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                loop {
                    executor.tick().await
                }
            });
    }

    #[cfg(target_os = "espidf")]
    {
        loop {
            executor.try_tick();
        }
    }
}

#[derive(thiserror::Error, Debug)]
enum WeatherError {
    #[error("Got no weather")]
    NoWeather,
}

async fn request_weather() -> anyhow::Result<WeatherForecast> {
    let result;
    let url = "https://api.open-meteo.com/v1/forecast?latitude=50.1155&longitude=8.6842&hourly=temperature_2m,precipitation_probability,precipitation,weather_code,cloud_cover&daily=sunrise,sunset&timezone=Europe%2FBerlin&forecast_days=3".to_string();

    #[cfg(target_os = "espidf")]
    {
        const SSID: &str = env!("SSID");
        const PASS: &str = env!("PASS");
        let sysloop = EspSystemEventLoop::take().unwrap();

        let modem = unsafe { WifiModem::new() };
        println!("ssid: {}, pw: {}", SSID, PASS);
        let (_esp_wifi, _) = wifi::wifi(modem, sysloop, SSID, PASS).unwrap();

        let ntp = Box::new(EspSntp::new_default().unwrap());

        while ntp.get_sync_status() != SyncStatus::Completed {
            smol::Timer::after(Duration::from_millis(200)).await;
        }
        // setup epd done
        let config = esp_idf_svc::http::client::Configuration {
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            ..Default::default()
        };
        let connection = esp_idf_svc::http::client::EspHttpConnection::new(&config).unwrap();

        let mut client = embedded_svc::http::client::Client::wrap(connection);
        // Prepare headers and URL
        let headers = [("Accept", "application/json")];

        // Send request
        //
        // Note: If you don't want to pass in any headers, you can also use `client.get(url, headers)`.
        log::info!("starting request");
        let request = client.request(embedded_svc::http::Method::Get, url.as_str(), &headers)?;
        let mut response = request.submit()?;

        // Process response
        let status = response.status();
        log::info!("status: {status}");

        // log::info!("size {}", size);
        let mut buffer = Box::new([0; 1 << 16]);

        let bytes_read = response.read(buffer.as_mut())?;

        let string = std::str::from_utf8(&buffer[0..bytes_read])?;

        result = Some(serde_json::from_str::<WeatherForecast>(string)?);
    }

    #[cfg(target_os = "linux")]
    {
        let res = reqwest::get(url).await;
        let json = res.unwrap().text().await.unwrap();

        result = Some(serde_json::from_str::<WeatherForecast>(&json)?);
    }

    if let Some(result) = result {
        Ok(result)
    } else {
        Err(anyhow::Error::new(WeatherError::NoWeather))
    }
}
