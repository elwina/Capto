fn main() {
    match capto_audio::list_devices() {
        Ok(devices) => {
            for device in devices {
                println!(
                    "{:?}\tdefault={}\t{}\t{}",
                    device.kind, device.is_default, device.name, device.id
                );
            }
        }
        Err(error) => {
            eprintln!("audio device enumeration failed: {error}");
            std::process::exit(1);
        }
    }
}
