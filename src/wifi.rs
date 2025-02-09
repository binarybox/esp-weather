use esp_idf_svc::sys::{esp, EspError};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    handle::RawHandle,
    ipv4::IpInfo,
    wifi::{AccessPointConfiguration, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};

/// connect to wifi. This is just copied from the espressif example. Modify the `PASSWORD` and `SSID` to connect to your wifi. Could be changed to run as access point.
pub fn wifi(
    modem: esp_idf_svc::hal::modem::WifiModem,
    sysloop: EspSystemEventLoop,
    ssid: &str,
    password: &str,
) -> anyhow::Result<(Box<EspWifi<'static>>, Option<IpInfo>)> {
    let mut esp_wifi = EspWifi::new(modem, sysloop.clone(), None)?;

    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

    log::info!("Starting wifi...");

    wifi.start()?;

    log::info!("Scanning...");

    let ap_infos = wifi.scan()?;

    let ours = ap_infos.into_iter().find(|a| a.ssid == ssid);

    let channel = if let Some(ours) = ours {
        log::info!(
            "Found configured access point {} on channel {}",
            ssid,
            ours.channel
        );
        Some(ours.channel)
    } else {
        log::info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            ssid
        );
        None
    };

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: ssid.try_into().unwrap(),
            password: password.try_into().unwrap(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".try_into().unwrap(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    let ip_info = {
        log::info!("Connecting wifi...");
        wifi.connect()?;

        log::info!("Waiting for DHCP lease...");
        wifi.wait_netif_up()?;

        if esp!(unsafe {
            esp_idf_svc::sys::esp_netif_create_ip6_linklocal(wifi.wifi().sta_netif().handle())
        })
        .is_err()
        {
            log::error!("failed to create iplink local address")
        }
        let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

        log::info!("Wifi DHCP info: {:?}", ip_info);
        Ok::<_, EspError>(ip_info)
    };

    Ok((Box::new(esp_wifi), ip_info.ok()))
}

pub fn ping(ip: esp_idf_svc::ipv4::Ipv4Addr) -> anyhow::Result<()> {
    let ping_summary = esp_idf_svc::ping::EspPing::default().ping(ip, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        log::error!("Pinging IP {} resulted in timeouts", ip);
    }

    Ok(())
}
