//! Virtual desktop geometry for multi-monitor capture.
//!
//! Coordinates must match FFmpeg `gdigrab` (physical pixels). On mixed-DPI
//! setups, naive `GetSystemMetrics` from a DPI-unaware thread returns a
//! squashed virtual desktop and breaks window/region crops.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualScreen {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl VirtualScreen {
    pub fn contains_point(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && py >= self.y
            && px < self.x + self.width as i32
            && py < self.y + self.height as i32
    }

    /// Clamp a screen-space rect into the virtual desktop; returns None if empty.
    pub fn clamp_rect(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Option<(i32, i32, u32, u32)> {
        let left = x.max(self.x);
        let top = y.max(self.y);
        let right = (x + width as i32).min(self.x + self.width as i32);
        let bottom = (y + height as i32).min(self.y + self.height as i32);
        let w = (right - left).max(0) as u32;
        let h = (bottom - top).max(0) as u32;
        if w < 2 || h < 2 {
            None
        } else {
            Some((left, top, w, h))
        }
    }

    pub fn intersection_area(&self, x: i32, y: i32, width: u32, height: u32) -> i64 {
        let left = x.max(self.x);
        let top = y.max(self.y);
        let right = (x + width as i32).min(self.x + self.width as i32);
        let bottom = (y + height as i32).min(self.y + self.height as i32);
        let w = (right - left).max(0) as i64;
        let h = (bottom - top).max(0) as i64;
        w * h
    }

    /// Convert screen-space rect to crop coords inside a full-desktop bitmap
    /// whose (0,0) pixel maps to (self.x, self.y).
    pub fn to_crop(&self, x: i32, y: i32, width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
        let (cx, cy, w, h) = self.clamp_rect(x, y, width, height)?;
        let crop_x = (cx - self.x).max(0) as u32;
        let crop_y = (cy - self.y).max(0) as u32;
        Some((even(w), even(h), crop_x, crop_y))
    }
}

fn even(v: u32) -> u32 {
    if v % 2 == 0 {
        v.max(2)
    } else {
        v.saturating_sub(1).max(2)
    }
}

/// OS virtual desktop (union of all monitors) in physical pixels.
pub fn virtual_screen() -> VirtualScreen {
    #[cfg(windows)]
    {
        windows_impl::virtual_screen()
    }
    #[cfg(not(windows))]
    {
        VirtualScreen {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }
}

/// Physical rectangles for each monitor (same space as `gdigrab`).
pub fn list_monitor_rects() -> Vec<VirtualScreen> {
    #[cfg(windows)]
    {
        windows_impl::list_monitor_rects()
    }
    #[cfg(not(windows))]
    {
        vec![virtual_screen()]
    }
}

/// Prefer the monitor with the largest overlap for a screen-space rect.
/// Falls back to the monitor containing the top-left, then index 0.
pub fn monitor_index_for_rect(x: i32, y: i32, width: u32, height: u32) -> u32 {
    let rects = list_monitor_rects();
    let mut best_idx = 0u32;
    let mut best_area = 0i64;
    for (idx, r) in rects.iter().enumerate() {
        let area = r.intersection_area(x, y, width, height);
        if area > best_area {
            best_area = area;
            best_idx = idx as u32;
        }
    }
    if best_area > 0 {
        return best_idx;
    }
    for (idx, r) in rects.iter().enumerate() {
        if r.contains_point(x, y) {
            return idx as u32;
        }
    }
    0
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

/// Current cursor position in physical screen pixels (matches `gdigrab`).
pub fn cursor_position() -> Result<ScreenPoint, crate::CaptureError> {
    #[cfg(windows)]
    {
        windows_impl::cursor_position()
    }
    #[cfg(not(windows))]
    {
        Ok(ScreenPoint { x: 0, y: 0 })
    }
}

/// Resolve a window previously identified by HWND-derived id.
pub fn window_by_id(id: u32) -> Result<Option<crate::WindowInfo>, crate::CaptureError> {
    #[cfg(windows)]
    {
        windows_impl::window_by_id(id)
    }
    #[cfg(not(windows))]
    {
        let _ = id;
        Ok(None)
    }
}

/// Enumerate visible top-level windows using HWND identifiers.
///
/// Unlike xcap's window enumeration, this tolerates protected Windows/UWP
/// surfaces which can reject metadata access for the entire enumeration.
pub fn list_windows() -> Result<Vec<crate::WindowInfo>, crate::CaptureError> {
    #[cfg(windows)]
    {
        windows_impl::list_windows()
    }
    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::VirtualScreen;
    use crate::{CaptureError, Result, WindowInfo};
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };
    use windows::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetCursorPos, GetSystemMetrics, GetWindowRect, GetWindowTextW,
        GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, SM_CXVIRTUALSCREEN,
        SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    };

    struct DpiGuard {
        prev: windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT,
    }

    impl DpiGuard {
        fn per_monitor_v2() -> Self {
            unsafe {
                let prev = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
                Self { prev }
            }
        }
    }

    impl Drop for DpiGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = SetThreadDpiAwarenessContext(self.prev);
            }
        }
    }

    pub fn virtual_screen() -> VirtualScreen {
        let rects = list_monitor_rects();
        union_rects(&rects).unwrap_or_else(|| unsafe {
            let _guard = DpiGuard::per_monitor_v2();
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(2) as u32;
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(2) as u32;
            VirtualScreen {
                x,
                y,
                width,
                height,
            }
        })
    }

    pub fn list_monitor_rects() -> Vec<VirtualScreen> {
        let _guard = DpiGuard::per_monitor_v2();
        unsafe {
            let mut out: Vec<VirtualScreen> = Vec::new();
            let _ = EnumDisplayMonitors(
                HDC::default(),
                None,
                Some(monitor_list_proc),
                LPARAM(&mut out as *mut Vec<VirtualScreen> as isize),
            );
            out
        }
    }

    fn union_rects(rects: &[VirtualScreen]) -> Option<VirtualScreen> {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        let mut count = 0u32;
        for r in rects {
            min_x = min_x.min(r.x);
            min_y = min_y.min(r.y);
            max_x = max_x.max(r.x + r.width as i32);
            max_y = max_y.max(r.y + r.height as i32);
            count += 1;
        }
        if count == 0 || min_x >= max_x || min_y >= max_y {
            None
        } else {
            Some(VirtualScreen {
                x: min_x,
                y: min_y,
                width: (max_x - min_x) as u32,
                height: (max_y - min_y) as u32,
            })
        }
    }

    unsafe extern "system" fn monitor_list_proc(
        monitor: HMONITOR,
        _hdc: HDC,
        _lprc: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let out = &mut *(data.0 as *mut Vec<VirtualScreen>);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let r = info.rcMonitor;
            out.push(VirtualScreen {
                x: r.left,
                y: r.top,
                width: (r.right - r.left).max(0) as u32,
                height: (r.bottom - r.top).max(0) as u32,
            });
        }
        BOOL(1)
    }

    pub fn cursor_position() -> Result<super::ScreenPoint> {
        let _guard = DpiGuard::per_monitor_v2();
        unsafe {
            let mut pt = POINT::default();
            GetCursorPos(&mut pt).map_err(|e| CaptureError::Failed(e.to_string()))?;
            Ok(super::ScreenPoint { x: pt.x, y: pt.y })
        }
    }

    pub fn window_by_id(id: u32) -> Result<Option<WindowInfo>> {
        let _guard = DpiGuard::per_monitor_v2();
        unsafe {
            let hwnd = HWND(id as isize as *mut _);
            if !IsWindow(hwnd).as_bool()
                || !IsWindowVisible(hwnd).as_bool()
                || IsIconic(hwnd).as_bool()
            {
                return Ok(None);
            }
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).map_err(|e| CaptureError::Failed(e.to_string()))?;
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut buf);
            let title = if len > 0 {
                String::from_utf16_lossy(&buf[..len as usize])
            } else {
                String::new()
            };
            Ok(Some(WindowInfo {
                id,
                title,
                app_name: format!("pid:{pid}"),
                width: (rect.right - rect.left).max(0) as u32,
                height: (rect.bottom - rect.top).max(0) as u32,
                x: rect.left,
                y: rect.top,
            }))
        }
    }

    pub fn list_windows() -> Result<Vec<WindowInfo>> {
        let _guard = DpiGuard::per_monitor_v2();
        unsafe {
            let mut windows = Vec::new();
            EnumWindows(
                Some(window_list_proc),
                LPARAM(&mut windows as *mut Vec<WindowInfo> as isize),
            )
            .map_err(|e| CaptureError::Failed(e.to_string()))?;
            Ok(windows)
        }
    }

    unsafe extern "system" fn window_list_proc(hwnd: HWND, data: LPARAM) -> BOOL {
        let windows = &mut *(data.0 as *mut Vec<WindowInfo>);
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return BOOL(1);
        }

        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return BOOL(1);
        }
        let width = (rect.right - rect.left).max(0) as u32;
        let height = (rect.bottom - rect.top).max(0) as u32;
        if width < 32 || height < 32 {
            return BOOL(1);
        }

        let mut title_buf = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buf);
        if title_len <= 0 {
            return BOOL(1);
        }
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);
        if title.trim().is_empty() || title == "Program Manager" {
            return BOOL(1);
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        windows.push(WindowInfo {
            id: hwnd.0 as usize as u32,
            title,
            app_name: format!("pid:{pid}"),
            width,
            height,
            x: rect.left,
            y: rect.top,
        });
        BOOL(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_crop_uses_virtual_origin() {
        let screen = VirtualScreen {
            x: -2560,
            y: 0,
            width: 5120,
            height: 1600,
        };
        // Primary monitor content.
        assert_eq!(
            screen.to_crop(0, 0, 2560, 1600),
            Some((2560, 1600, 2560, 0))
        );
        // Window on secondary (negative X).
        assert_eq!(
            screen.to_crop(-2000, 200, 800, 600),
            Some((800, 600, 560, 200))
        );
    }
}
