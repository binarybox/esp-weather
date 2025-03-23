fn main() {
    #[cfg(target_os = "espidf")]
    println!("cargo::warning=This is for esp32");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS");
    if let Ok(os) = target_os {
        if os == "espidf" {
            // #[cfg(feature = "espidf")]
            embuild::espidf::sysenv::output();
        }
    }
}
