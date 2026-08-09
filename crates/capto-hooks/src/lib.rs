use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyAction {
    StartRecording,
    PauseRecording,
    StopRecording,
    TakeScreenshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBinding {
    pub action: HotkeyAction,
    /// Tauri / keyboard shortcut string, e.g. "CommandOrControl+Shift+R"
    pub shortcut: String,
    pub enabled: bool,
}

impl HotkeyBinding {
    pub fn new(action: HotkeyAction, shortcut: impl Into<String>) -> Self {
        Self {
            action,
            shortcut: shortcut.into(),
            enabled: true,
        }
    }
}

pub fn default_hotkeys() -> Vec<HotkeyBinding> {
    // Alt+F4 is reserved by Windows (close window). F5–F8 is a compact cluster.
    vec![
        HotkeyBinding::new(HotkeyAction::StartRecording, "Alt+F5"),
        HotkeyBinding::new(HotkeyAction::PauseRecording, "Alt+F6"),
        HotkeyBinding::new(HotkeyAction::StopRecording, "Alt+F7"),
        HotkeyBinding::new(HotkeyAction::TakeScreenshot, "Alt+F8"),
    ]
}

/// Ensure settings always expose exactly the four supported actions, in order.
/// Migrates the previous Ctrl+Shift defaults to Alt+F5–F8 when still unchanged.
pub fn normalize_hotkeys(hotkeys: &mut Vec<HotkeyBinding>) {
    const LEGACY: &[(HotkeyAction, &str, &str)] = &[
        (
            HotkeyAction::StartRecording,
            "CommandOrControl+Shift+R",
            "Alt+F5",
        ),
        (
            HotkeyAction::PauseRecording,
            "CommandOrControl+Shift+P",
            "Alt+F6",
        ),
        (
            HotkeyAction::StopRecording,
            "CommandOrControl+Shift+E",
            "Alt+F7",
        ),
        (
            HotkeyAction::TakeScreenshot,
            "CommandOrControl+Shift+S",
            "Alt+F8",
        ),
    ];

    for (action, old, new) in LEGACY {
        if let Some(b) = hotkeys.iter_mut().find(|h| h.action == *action) {
            if b.shortcut.eq_ignore_ascii_case(old) {
                b.shortcut = (*new).into();
            }
        }
    }

    let defaults = default_hotkeys();
    let mut ordered = Vec::with_capacity(4);
    for d in defaults {
        if let Some(existing) = hotkeys.iter().find(|h| h.action == d.action) {
            ordered.push(existing.clone());
        } else {
            ordered.push(d);
        }
    }
    *hotkeys = ordered;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    MouseButton {
        button: MouseButton,
        x: i32,
        y: i32,
        down: bool,
    },
    Key {
        /// Display label, e.g. "Ctrl+C" or "A".
        label: String,
        down: bool,
    },
}

/// Low-level mouse/keyboard hooks used by recording overlays.
pub trait InputHook: Send {
    fn start(&mut self, tx: Sender<InputEvent>) -> Result<(), String>;
    fn stop(&mut self);
}

#[derive(Default)]
pub struct NullInputHook;

impl InputHook for NullInputHook {
    fn start(&mut self, _tx: Sender<InputEvent>) -> Result<(), String> {
        Ok(())
    }

    fn stop(&mut self) {}
}

/// Create the platform input hook (Windows LL hooks; elsewhere null).
pub fn create_input_hook() -> Box<dyn InputHook> {
    #[cfg(windows)]
    {
        Box::new(windows_impl::WindowsInputHook::default())
    }
    #[cfg(not(windows))]
    {
        Box::new(NullInputHook)
    }
}

pub fn channel() -> (Sender<InputEvent>, Receiver<InputEvent>) {
    mpsc::channel()
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::collections::HashSet;
    use std::thread::{self, JoinHandle};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyNameTextW, MapVirtualKeyW, MAPVK_VK_TO_VSC, VIRTUAL_KEY, VK_CONTROL,
        VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
        WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_QUIT, WM_RBUTTONDOWN,
        WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    static TX: Mutex<Option<Sender<InputEvent>>> = Mutex::new(None);
    static MOUSE_HOOK: Mutex<Option<isize>> = Mutex::new(None);
    static KEY_HOOK: Mutex<Option<isize>> = Mutex::new(None);
    /// Tracks currently-held VKs so OS auto-repeat KEYDOWNs are ignored.
    static KEYS_DOWN: Mutex<Option<HashSet<u16>>> = Mutex::new(None);

    #[derive(Default)]
    pub struct WindowsInputHook {
        thread: Option<JoinHandle<()>>,
        thread_id: Arc<Mutex<Option<u32>>>,
    }

    impl InputHook for WindowsInputHook {
        fn start(&mut self, tx: Sender<InputEvent>) -> Result<(), String> {
            self.stop();
            {
                let mut guard = TX.lock().map_err(|e| e.to_string())?;
                *guard = Some(tx);
            }
            if let Ok(mut keys) = KEYS_DOWN.lock() {
                *keys = Some(HashSet::new());
            }
            let thread_id = Arc::clone(&self.thread_id);
            let handle = thread::Builder::new()
                .name("capto-input-hooks".into())
                .spawn(move || {
                    unsafe {
                        let _ = SetThreadDpiAwarenessContext(
                            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
                        );
                        if let Ok(mut id) = thread_id.lock() {
                            *id = Some(windows::Win32::System::Threading::GetCurrentThreadId());
                        }

                        match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0) {
                            Ok(h) => {
                                if let Ok(mut g) = MOUSE_HOOK.lock() {
                                    *g = Some(h.0 as isize);
                                }
                            }
                            Err(e) => {
                                eprintln!("capto-hooks: WH_MOUSE_LL failed: {e}");
                            }
                        }
                        match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) {
                            Ok(h) => {
                                if let Ok(mut g) = KEY_HOOK.lock() {
                                    *g = Some(h.0 as isize);
                                }
                            }
                            Err(e) => {
                                eprintln!("capto-hooks: WH_KEYBOARD_LL failed: {e}");
                            }
                        }

                        let mut msg = MSG::default();
                        while GetMessageW(&mut msg, None, 0, 0).into() {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }

                        unhook_all();
                    }
                })
                .map_err(|e| e.to_string())?;
            self.thread = Some(handle);
            Ok(())
        }

        fn stop(&mut self) {
            if let Ok(mut id) = self.thread_id.lock() {
                if let Some(tid) = id.take() {
                    unsafe {
                        let _ = windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                            tid,
                            WM_QUIT,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                }
            }
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
            if let Ok(mut g) = TX.lock() {
                *g = None;
            }
            if let Ok(mut keys) = KEYS_DOWN.lock() {
                *keys = None;
            }
            unhook_all();
        }
    }

    fn unhook_all() {
        unsafe {
            if let Ok(mut g) = MOUSE_HOOK.lock() {
                if let Some(h) = g.take() {
                    let _ = UnhookWindowsHookEx(HHOOK(h as *mut _));
                }
            }
            if let Ok(mut g) = KEY_HOOK.lock() {
                if let Some(h) = g.take() {
                    let _ = UnhookWindowsHookEx(HHOOK(h as *mut _));
                }
            }
        }
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let button = match wparam.0 as u32 {
                WM_LBUTTONDOWN => Some(MouseButton::Left),
                WM_RBUTTONDOWN => Some(MouseButton::Right),
                WM_MBUTTONDOWN => Some(MouseButton::Middle),
                _ => None,
            };
            if let Some(button) = button {
                emit(InputEvent::MouseButton {
                    button,
                    x: info.pt.x,
                    y: info.pt.y,
                    down: true,
                });
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            let up = matches!(wparam.0 as u32, WM_KEYUP | WM_SYSKEYUP);
            if down || up {
                let vk = info.vkCode as u16;
                if is_modifier_vk(vk) {
                    // Modifiers only participate as chord prefixes on other keys.
                } else if down {
                    // Drop OS auto-repeat: only the first KEYDOWN per hold counts.
                    let first_down = KEYS_DOWN
                        .lock()
                        .ok()
                        .and_then(|mut g| g.as_mut().map(|set| set.insert(vk)))
                        .unwrap_or(true);
                    if first_down {
                        if let Some(label) = format_key_label(vk) {
                            emit(InputEvent::Key {
                                label,
                                down: true,
                            });
                        }
                    }
                } else if up {
                    if let Ok(mut g) = KEYS_DOWN.lock() {
                        if let Some(set) = g.as_mut() {
                            set.remove(&vk);
                        }
                    }
                }
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    fn is_modifier_vk(vk: u16) -> bool {
        matches!(
            vk,
            0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5
        )
    }

    fn emit(event: InputEvent) {
        if let Ok(guard) = TX.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(event);
            }
        }
    }

    unsafe fn format_key_label(vk: u16) -> Option<String> {
        let mut parts = Vec::new();
        if key_down(VK_CONTROL) {
            parts.push("Ctrl");
        }
        if key_down(VK_MENU) {
            parts.push("Alt");
        }
        if key_down(VK_SHIFT) {
            parts.push("Shift");
        }
        if key_down(VK_LWIN) || key_down(VK_RWIN) {
            parts.push("Win");
        }

        let name = key_name(vk)?;
        // Avoid "Ctrl+Control" style duplicates.
        if parts.iter().any(|p| p.eq_ignore_ascii_case(&name)) {
            return None;
        }
        parts.push(name.as_str());
        Some(parts.join("+"))
    }

    unsafe fn key_down(vk: VIRTUAL_KEY) -> bool {
        GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000 != 0
    }

    unsafe fn key_name(vk: u16) -> Option<String> {
        let scan = MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC);
        if scan == 0 {
            return Some(format!("0x{vk:X}"));
        }
        let lparam = (scan as i32) << 16;
        let mut buf = [0u16; 64];
        let n = GetKeyNameTextW(lparam, &mut buf);
        if n <= 0 {
            return Some(format!("0x{vk:X}"));
        }
        String::from_utf16(&buf[..n as usize]).ok()
    }
}
