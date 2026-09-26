//! Where the League client window is on screen, so overlays can sit beside it. Only
//! top-level window geometry is read (Win32 window calls); the client process itself is
//! never touched.

/// Screen rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClientWindow {
    pub bounds: Rect,
    /// Work area (screen minus taskbar) of the monitor the client is on.
    pub work_area: Rect,
    pub minimized: bool,
    /// The user is looking at the client rather than another window.
    pub foreground: bool,
}

/// `None` when no League client window exists.
#[cfg(windows)]
pub fn find() -> Option<ClientWindow> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetForegroundWindow, GetWindowRect, IsIconic,
    };

    // The client's top-level window class, the same on every server.
    let class: Vec<u16> = "RCLIENT\0".encode_utf16().collect();
    // SAFETY: read-only Win32 window queries; every pointer passed outlives its call.
    unsafe {
        let hwnd = FindWindowW(class.as_ptr(), std::ptr::null());
        if hwnd.is_null() {
            return None;
        }
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return None;
        }
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let work = if GetMonitorInfoW(monitor, &mut info) != 0 {
            info.rcWork
        } else {
            rect
        };
        Some(ClientWindow {
            bounds: to_rect(rect),
            work_area: to_rect(work),
            minimized: IsIconic(hwnd) != 0,
            foreground: GetForegroundWindow() == hwnd,
        })
    }
}

#[cfg(not(windows))]
pub fn find() -> Option<ClientWindow> {
    None
}

#[cfg(windows)]
fn to_rect(r: windows_sys::Win32::Foundation::RECT) -> Rect {
    Rect {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    }
}
