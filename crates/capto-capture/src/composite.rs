//! Blit webcam PiP onto a screen frame (BGRA or RGBA) before encode / JPEG.

use crate::Frame;
use capto_overlay::{resolve_pixel_position, WebcamPip};

/// Composite `cam` onto `base` in-place. Both frames use 4-byte pixels in the
/// same channel order (BGRA for DXGI/rawvideo, RGBA for preview JPEG).
pub fn composite_webcam_pip(base: &mut Frame, cam: &Frame, pip: &WebcamPip) {
    if !pip.enabled || cam.width < 2 || cam.height < 2 || base.width < 2 || base.height < 2 {
        return;
    }

    let box_w = pip.width.max(2).min(base.width);
    let box_h = pip.height.max(2).min(base.height);
    let (ox, oy) = resolve_pixel_position(&pip.position, base.width, base.height, box_w, box_h);

    let radius = pip.corner_radius.min(box_w.min(box_h) / 2);

    // Fast path: cam already matches PiP box, no rounded corners.
    if radius == 0
        && cam.width == box_w
        && cam.height == box_h
        && ox >= 0
        && oy >= 0
        && ox as u32 + box_w <= base.width
        && oy as u32 + box_h <= base.height
    {
        let row_bytes = (box_w * 4) as usize;
        for dy in 0..box_h {
            let di = (((oy as u32 + dy) * base.width + ox as u32) * 4) as usize;
            if pip.mirrored {
                let src_row = (dy * box_w * 4) as usize;
                for dx in 0..box_w {
                    let sx = box_w - 1 - dx;
                    let si = src_row + (sx * 4) as usize;
                    let doi = di + (dx * 4) as usize;
                    base.rgba[doi..doi + 4].copy_from_slice(&cam.rgba[si..si + 4]);
                }
            } else {
                let si = (dy * box_w * 4) as usize;
                base.rgba[di..di + row_bytes].copy_from_slice(&cam.rgba[si..si + row_bytes]);
            }
        }
        return;
    }

    let radius_sq = u64::from(radius) * u64::from(radius);

    for dy in 0..box_h {
        let by = oy + dy as i32;
        if by < 0 || by as u32 >= base.height {
            continue;
        }
        for dx in 0..box_w {
            let bx = ox + dx as i32;
            if bx < 0 || bx as u32 >= base.width {
                continue;
            }

            if radius > 0 && !inside_rounded(dx, dy, box_w, box_h, radius, radius_sq) {
                continue;
            }

            let sx = if pip.mirrored { box_w - 1 - dx } else { dx };
            // Nearest-neighbor sample from cam (already sized near target).
            let cx = (u64::from(sx) * u64::from(cam.width) / u64::from(box_w.max(1))) as u32;
            let cy = (u64::from(dy) * u64::from(cam.height) / u64::from(box_h.max(1))) as u32;
            let si = ((cy * cam.width + cx) * 4) as usize;
            let di = ((by as u32 * base.width + bx as u32) * 4) as usize;
            if si + 4 > cam.rgba.len() || di + 4 > base.rgba.len() {
                continue;
            }
            base.rgba[di..di + 4].copy_from_slice(&cam.rgba[si..si + 4]);
        }
    }
}

fn inside_rounded(x: u32, y: u32, w: u32, h: u32, r: u32, r_sq: u64) -> bool {
    let in_mid_x = x >= r && x < w.saturating_sub(r);
    let in_mid_y = y >= r && y < h.saturating_sub(r);
    if in_mid_x || in_mid_y {
        return true;
    }
    let cx = if x < r { r - x } else { x - (w - r - 1) };
    let cy = if y < r { r - y } else { y - (h - r - 1) };
    u64::from(cx) * u64::from(cx) + u64::from(cy) * u64::from(cy) <= r_sq
}

/// Swap R/B channels in-place (BGRA ↔ RGBA).
pub fn swap_rb_inplace(frame: &mut Frame) {
    for px in frame.rgba.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
}
