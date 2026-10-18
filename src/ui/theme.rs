//! Applies the light/dark palette to the native controls and persists the choice.

use std::cell::RefCell;
use std::collections::HashMap;
use winapi::ctypes::c_void;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};

use native_windows_gui as nwg;
use winapi::shared::minwindef::{BOOL, DWORD, LPARAM, LRESULT, WPARAM};
use winapi::shared::windef::{HBRUSH, HDC, HWND, RECT};
use winapi::um::commctrl::{LVM_GETHEADER, LVM_SETTEXTBKCOLOR};
use winapi::um::dwmapi::DwmSetWindowAttribute;
use winapi::um::uxtheme::SetWindowTheme;
use winapi::um::wingdi::{CreateSolidBrush, SetBkColor, SetTextColor, RGB};
use winapi::um::winreg::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use winapi::um::winuser::{
    FillRect, GetClassNameW, GetClientRect, RedrawWindow, SendMessageW, RDW_ALLCHILDREN, RDW_ERASE,
    RDW_INVALIDATE, RDW_UPDATENOW, WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_ERASEBKGND,
};

use super::palette::{Rgb, Theme};

const RAW_HANDLER_ID: usize = 0x1_0001;
const DWMWA_USE_IMMERSIVE_DARK_MODE: DWORD = 20;
const SETTINGS_DIR: &str = "UbntDiscovery";
const SETTINGS_FILE: &str = "theme.txt";
const PERSONALIZE_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";

static DARK: AtomicBool = AtomicBool::new(false);

thread_local! {
    static BRUSHES: RefCell<HashMap<Rgb, usize>> = RefCell::new(HashMap::new());
}

pub fn current() -> Theme {
    if DARK.load(Ordering::Relaxed) {
        Theme::Dark
    } else {
        Theme::Light
    }
}

pub fn set_current(theme: Theme) {
    DARK.store(theme == Theme::Dark, Ordering::Relaxed);
}

/// Saved choice, else the system "apps" theme, else light.
pub fn load() -> Theme {
    settings_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| Theme::parse(&text))
        .unwrap_or_else(system_theme)
}

pub fn save(theme: Theme) -> std::io::Result<()> {
    let path = settings_path().ok_or_else(|| std::io::Error::other("%APPDATA% is not set"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, theme.as_str())
}

fn settings_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(base).join(SETTINGS_DIR).join(SETTINGS_FILE))
}

fn system_theme() -> Theme {
    let key = wide(PERSONALIZE_KEY);
    let name = wide("AppsUseLightTheme");
    let mut value: DWORD = 1;
    let mut size = std::mem::size_of::<DWORD>() as DWORD;
    // SAFETY: all pointers are valid for the call; `value`/`size` describe a DWORD.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            (&mut value as *mut DWORD).cast::<c_void>(),
            &mut size,
        )
    };
    if status == 0 && value == 0 {
        Theme::Dark
    } else {
        Theme::Light
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn colorref((r, g, b): Rgb) -> u32 {
    RGB(r, g, b)
}

fn brush(color: Rgb) -> HBRUSH {
    BRUSHES.with(|cache| {
        *cache
            .borrow_mut()
            .entry(color)
            // SAFETY: plain GDI allocation; the brush lives for the whole process.
            .or_insert_with(|| unsafe { CreateSolidBrush(colorref(color)) } as usize)
            as HBRUSH
    })
}

fn is_edit(control: HWND) -> bool {
    let mut class = [0u16; 8];
    // SAFETY: the buffer length passed matches the buffer.
    let len = unsafe { GetClassNameW(control, class.as_mut_ptr(), class.len() as i32) };
    String::from_utf16_lossy(&class[..len.max(0) as usize]).eq_ignore_ascii_case("edit")
}

/// Paint the window background and tint its static/edit children from the active palette.
/// `secondary` is a label drawn in the secondary text colour.
pub fn attach(window: &nwg::Window, secondary: Option<&nwg::Label>) {
    let secondary = secondary.and_then(|l| l.handle.hwnd()).map(|h| h as usize);
    let handler = nwg::bind_raw_event_handler(
        &window.handle,
        RAW_HANDLER_ID,
        move |hwnd, msg, wparam, lparam| -> Option<LRESULT> {
            let palette = current().palette();
            match msg {
                WM_ERASEBKGND => {
                    // SAFETY: `hwnd` is the window being painted and `wparam` its HDC.
                    unsafe {
                        let mut rect: RECT = std::mem::zeroed();
                        GetClientRect(hwnd, &mut rect);
                        FillRect(wparam as HDC, &rect, brush(palette.background));
                    }
                    Some(1)
                }
                WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT => {
                    let control = lparam as HWND;
                    let (text, ground) = if msg == WM_CTLCOLOREDIT || is_edit(control) {
                        (palette.text, palette.surface)
                    } else if Some(control as usize) == secondary {
                        (palette.secondary, palette.background)
                    } else {
                        (palette.text, palette.background)
                    };
                    // SAFETY: `wparam` is the control's HDC for this message.
                    unsafe {
                        SetTextColor(wparam as HDC, colorref(text));
                        SetBkColor(wparam as HDC, colorref(ground));
                    }
                    Some(brush(ground) as LRESULT)
                }
                _ => None,
            }
        },
    );
    // A failed bind only leaves the default colours; nothing else depends on it.
    drop(handler);
}

/// Re-style a top-level window (title bar) and repaint it with its children.
pub fn style_window(window: &nwg::Window) {
    let Some(hwnd) = window.handle.hwnd() else {
        return;
    };
    let dark = BOOL::from(current() == Theme::Dark);
    // SAFETY: `hwnd` is a live window; the attribute value is a BOOL of the stated size.
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark as *const BOOL).cast::<c_void>(),
            std::mem::size_of::<BOOL>() as DWORD,
        );
        RedrawWindow(
            hwnd,
            null(),
            null_mut(),
            RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW,
        );
    }
}

fn set_visual_style(hwnd: HWND, dark_class: &str) {
    let class = wide(dark_class);
    let dark = current() == Theme::Dark;
    // SAFETY: `hwnd` is a live control; a null class name restores the default style.
    unsafe {
        SetWindowTheme(hwnd, if dark { class.as_ptr() } else { null() }, null());
    }
}

pub fn style_button(button: &nwg::Button) {
    if let Some(hwnd) = button.handle.hwnd() {
        set_visual_style(hwnd, "DarkMode_Explorer");
    }
}

pub fn style_edit(input: &nwg::TextInput) {
    if let Some(hwnd) = input.handle.hwnd() {
        set_visual_style(hwnd, "DarkMode_CFD");
    }
}

pub fn style_list(list: &nwg::ListView) {
    let Some(hwnd) = list.handle.hwnd() else {
        return;
    };
    let palette = current().palette();
    set_visual_style(hwnd, "DarkMode_Explorer");
    let (r, g, b) = palette.background;
    list.set_background_color(r, g, b);
    let (r, g, b) = palette.text;
    list.set_text_color(r, g, b);
    // SAFETY: `hwnd` is a live list view; LVM_GETHEADER returns its header window or null.
    unsafe {
        SendMessageW(
            hwnd,
            LVM_SETTEXTBKCOLOR,
            0 as WPARAM,
            colorref(palette.background) as LPARAM,
        );
        let header = SendMessageW(hwnd, LVM_GETHEADER, 0, 0) as HWND;
        if !header.is_null() {
            set_visual_style(header, "DarkMode_ItemsView");
        }
    }
}
