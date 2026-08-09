//! Smoke: list cameras and grab a few continuous MF frames.
//!
//! ```bash
//! cargo run -p capto-capture --example webcam_smoke
//! ```

fn main() {
    let devices = capto_capture::list_webcams().expect("list_webcams");
    println!("webcams: {}", devices.len());
    for d in &devices {
        println!("  - {} ({})", d.name, d.id);
    }
    if devices.is_empty() {
        eprintln!("no cameras — skip capture");
        return;
    }

    let cam = capto_capture::WebcamCapture::start(None, 320, 240).expect("start webcam");
    let slot = cam.slot();
    let mut got = 0u32;
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 3 {
        if let Some(f) = slot.latest() {
            got += 1;
            if got == 1 || got % 30 == 0 {
                println!(
                    "frame#{got}: {}x{} bytes={} t={}ms",
                    f.width,
                    f.height,
                    f.rgba.len(),
                    f.timestamp_ms
                );
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(33));
    }
    drop(cam);
    println!("done: {got} slot reads with frames in ~3s");
    assert!(got > 10, "expected continuous frames, got {got}");
}
