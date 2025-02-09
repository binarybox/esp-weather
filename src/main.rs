use std::time::Duration;

use chrono::{Datelike, Days, NaiveTime};
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
    text::write_centered_text,
    weather::{write_single_weather, WeatherForecast},
    wifi,
};

const SSID: &str = "WIFI";
const PASS: &str = "PASSWORD";

use epd_waveshare::epd7in5b_v2::Display7in5 as Display;
use epd_waveshare::epd7in5b_v2::Epd7in5 as Epd;
use u8g2_fonts::fonts::u8g2_font_helvB10_tr;

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

                let height = 50;

                weather
                    .weather
                    .sort_by(|a, b| a.date.value().cmp(&b.date.value()));

                Line::new(Point::new(0, DISPLAY_HEIGHT as i32), Point::new(0, height))
                    .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
                    .draw(display.as_mut())
                    .unwrap();

                write_single_weather(
                    0,
                    &chrono::Local::now().weekday().to_string(),
                    display.as_mut(),
                    &weather,
                );

                Line::new(
                    Point::new((SECTION_WIDTH) - 1, DISPLAY_HEIGHT as i32),
                    Point::new((SECTION_WIDTH) - 1, height),
                )
                .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 2))
                .draw(display.as_mut())
                .unwrap();
                // tomorrow
                write_single_weather(
                    1,
                    &chrono::Local::now().weekday().succ().to_string(),
                    display.as_mut(),
                    &weather,
                );
                Line::new(
                    Point::new((SECTION_WIDTH * 2) - 1, DISPLAY_HEIGHT as i32),
                    Point::new((SECTION_WIDTH * 2) - 1, height),
                )
                .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 2))
                .draw(display.as_mut())
                .unwrap();
                // tomorrow + 1
                write_single_weather(
                    2,
                    &chrono::Local::now().weekday().succ().succ().to_string(),
                    display.as_mut(),
                    &weather,
                );
                Line::new(
                    Point::new((SECTION_WIDTH * 3) - 1, DISPLAY_HEIGHT as i32),
                    Point::new((SECTION_WIDTH * 3) - 1, height),
                )
                .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
                .draw(display.as_mut())
                .unwrap();
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
    let url = "https://wttr.in/Frankfurt?format=j1";

    // Send request
    //
    // Note: If you don't want to pass in any headers, you can also use `client.get(url, headers)`.
    log::info!("starting request");
    let request = client.request(embedded_svc::http::Method::Get, url, &headers)?;
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
