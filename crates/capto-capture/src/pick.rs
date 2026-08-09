//! Cursor / screen hit-testing helpers for Captura-style window picking.

use crate::{CaptureError, Frame, Result, WindowInfo};

/// Top-level window under the OS cursor, skipping `skip_pid` (usually Capto itself).
pub fn window_under_cursor(skip_pid: Option<u32>) -> Result<Option<WindowInfo>> {
    #[cfg(windows)]
    {
        windows_impl::window_under_cursor(skip_pid)
    }
    #[cfg(not(windows))]
    {
        let _ = skip_pid;
        Ok(None)
    }
}

/// Capture a window by HWND-derived id (physical pixels, desktop BitBlt).
pub fn capture_window_by_id(id: u32) -> Result<Frame> {
    #[cfg(windows)]
    {
        windows_impl::capture_window_by_id(id)
    }
    #[cfg(not(windows))]
    {
        let _ = id;
        Err(CaptureError::Unsupported(
            "window HWND capture is Windows-only",
        ))
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use windows::Win32::Foundation::{HWND, POINT, RECT};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HGDIOBJ, SRCCOPY,
    };
    use windows::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetCursorPos, GetDesktopWindow, GetTopWindow, GetWindow, GetWindowRect,
        GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, GA_ROOT,
        GW_HWNDNEXT,
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

    pub fn window_under_cursor(skip_pid: Option<u32>) -> Result<Option<WindowInfo>> {
        let _guard = DpiGuard::per_monitor_v2();
        unsafe {
            let mut pt = POINT::default();
            GetCursorPos(&mut pt).map_err(|e| CaptureError::Failed(e.to_string()))?;

            let desktop = GetDesktopWindow();
            let mut hwnd = GetTopWindow(desktop).unwrap_or_default();
            while !hwnd.0.is_null() {
                let root = GetAncestor(hwnd, GA_ROOT);
                let candidate = if root.0.is_null() { hwnd } else { root };

                if is_pickable(candidate, pt, skip_pid) {
                    return Ok(Some(to_info(candidate)?));
                }

                hwnd = GetWindow(hwnd, GW_HWNDNEXT).unwrap_or_default();
            }
            Ok(None)
        }
    }

    pub fn capture_window_by_id(id: u32) -> Result<Frame> {
        let _guard = DpiGuard::per_monitor_v2();
        unsafe {
            let hwnd = HWND(id as isize as *mut _);
            if !IsWindow(hwnd).as_bool() {
                return Err(CaptureError::WindowNotFound(id.to_string()));
            }
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).map_err(|e| CaptureError::Failed(e.to_string()))?;
            let width = (rect.right - rect.left).max(2) as u32;
            let height = (rect.bottom - rect.top).max(2) as u32;

            let screen_dc = GetDC(HWND::default());
            if screen_dc.is_invalid() {
                return Err(CaptureError::Failed("GetDC failed".into()));
            }
            let mem_dc = CreateCompatibleDC(screen_dc);
            if mem_dc.is_invalid() {
                ReleaseDC(HWND::default(), screen_dc);
                return Err(CaptureError::Failed("CreateCompatibleDC failed".into()));
            }
            let bmp = CreateCompatibleBitmap(screen_dc, width as i32, height as i32);
            if bmp.is_invalid() {
                let _ = DeleteDC(mem_dc);
                ReleaseDC(HWND::default(), screen_dc);
                return Err(CaptureError::Failed("CreateCompatibleBitmap failed".into()));
            }
            let old = SelectObject(mem_dc, HGDIOBJ(bmp.0));
            if BitBlt(
                mem_dc,
                0,
                0,
                width as i32,
                height as i32,
                screen_dc,
                rect.left,
                rect.top,
                SRCCOPY,
            )
            .is_err()
            {
                SelectObject(mem_dc, old);
                let _ = DeleteObject(HGDIOBJ(bmp.0));
                let _ = DeleteDC(mem_dc);
                ReleaseDC(HWND::default(), screen_dc);
                return Err(CaptureError::Failed("BitBlt failed".into()));
            }

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32), // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0 as u32,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bgra = vec![0u8; (width * height * 4) as usize];
            let lines = GetDIBits(
                mem_dc,
                bmp,
                0,
                height,
                Some(bgra.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );
            SelectObject(mem_dc, old);
            let _ = DeleteObject(HGDIOBJ(bmp.0));
            let _ = DeleteDC(mem_dc);
            ReleaseDC(HWND::default(), screen_dc);
            if lines == 0 {
                return Err(CaptureError::Failed("GetDIBits failed".into()));
            }

            // BGRA → RGBA
            for px in bgra.chunks_exact_mut(4) {
                px.swap(0, 2);
            }

            let timestamp_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            Ok(Frame {
                width,
                height,
                rgba: bgra,
                timestamp_ms,
            })
        }
    }

    unsafe fn is_pickable(hwnd: HWND, pt: POINT, skip_pid: Option<u32>) -> bool {
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if let Some(skip) = skip_pid {
            if pid == skip {
                return false;
            }
        }
        let title = window_title(hwnd);
        if title.trim().is_empty() || title == "Program Manager" {
            return false;
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return false;
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width < 32 || height < 32 {
            return false;
        }
        pt.x >= rect.left && pt.x < rect.right && pt.y >= rect.top && pt.y < rect.bottom
    }

    unsafe fn to_info(hwnd: HWND) -> Result<WindowInfo> {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).map_err(|e| CaptureError::Failed(e.to_string()))?;
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        Ok(WindowInfo {
            id: hwnd.0 as usize as u32,
            title: window_title(hwnd),
            app_name: format!("pid:{pid}"),
            width: (rect.right - rect.left).max(0) as u32,
            height: (rect.bottom - rect.top).max(0) as u32,
            x: rect.left,
            y: rect.top,
        })
    }

    unsafe fn window_title(hwnd: HWND) -> String {
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..len as usize])
    }
}
