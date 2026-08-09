//! Print virtual desktop geometry as Capto sees it, to compare against what
//! FFmpeg's gdigrab reports for `-i desktop`.

fn main() {
    let vs = capto_capture::virtual_screen();
    println!(
        "virtual_screen: x={} y={} {}x{}",
        vs.x, vs.y, vs.width, vs.height
    );

    for (i, r) in capto_capture::list_monitor_rects().iter().enumerate() {
        println!("monitor[{i}]: x={} y={} {}x{}", r.x, r.y, r.width, r.height);
    }

    // Sanity: a full-desktop crop must be a no-op, and an offscreen rect must clamp.
    println!(
        "to_crop(full) = {:?}",
        vs.to_crop(vs.x, vs.y, vs.width, vs.height)
    );
    println!("to_crop(0,0,800,600) = {:?}", vs.to_crop(0, 0, 800, 600));
}
