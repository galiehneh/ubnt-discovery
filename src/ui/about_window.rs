//! Modal "About" dialog: avatar, version, repository link and licence.

use std::sync::atomic::Ordering;

use native_windows_gui as nwg;

use super::model::{APP_NAME, APP_VERSION, AUTHOR, LICENSE_TEXT, ORIGINAL_TOOL, REPO_URL};

const AVATAR_PNG: &[u8] = include_bytes!("../../resources/avatar.png");
const AVATAR_SIZE: i32 = 96;
const MARGIN: i32 = 16;
const DIALOG_SIZE: (i32, i32) = (460, 300);
const TEXT_LEFT: i32 = MARGIN * 2 + AVATAR_SIZE;
const TEXT_WIDTH: i32 = DIALOG_SIZE.0 - TEXT_LEFT - MARGIN;

struct Dialog {
    window: nwg::Window,
    _avatar: Option<nwg::Bitmap>,
    _avatar_frame: nwg::ImageFrame,
    _labels: Vec<nwg::Label>,
    _repo_field: nwg::TextInput,
    open_btn: nwg::Button,
    close_btn: nwg::Button,
}

/// Show the modal About dialog; returns once it is closed.
pub fn show(parent: &nwg::Window) {
    let dialog = match build(parent) {
        Ok(d) => d,
        Err(err) => {
            nwg::error_message(APP_NAME, &format!("Could not open About: {err}"));
            return;
        }
    };
    let window_handle = dialog.window.handle;
    let close_handle = dialog.close_btn.handle;
    let open_handle = dialog.open_btn.handle;
    let handler = nwg::full_bind_event_handler(&window_handle, move |evt, data, handle| {
        use nwg::Event as E;
        let close = match evt {
            E::OnWindowClose if handle == window_handle => {
                // Defer destruction until `show` has re-enabled the owner.
                if let nwg::EventData::OnWindowClose(close_data) = data {
                    close_data.close(false);
                }
                true
            }
            E::OnButtonClick if handle == open_handle => {
                // A failed launch is harmless: the URL stays selectable in the field.
                let _ = opener::open(REPO_URL);
                false
            }
            E::OnButtonClick => handle == close_handle,
            E::OnKeyPress => matches!(data.on_key(), nwg::keys::RETURN | nwg::keys::ESCAPE),
            _ => false,
        };
        if close {
            nwg::stop_thread_dispatch();
        }
    });

    parent.set_enabled(false);
    dialog.close_btn.set_focus();
    nwg::dispatch_thread_events();
    parent.set_enabled(true);
    nwg::unbind_event_handler(&handler);
    dialog.window.close();
    if super::QUIT_REQUESTED.load(Ordering::SeqCst) {
        // Same WM_QUIT hand-off as the details dialog.
        nwg::stop_thread_dispatch();
    } else {
        parent.set_focus();
    }
}

fn build(parent: &nwg::Window) -> Result<Dialog, nwg::NwgError> {
    let mut window = nwg::Window::default();
    nwg::Window::builder()
        .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
        .size(DIALOG_SIZE)
        .center(true)
        .title(&format!("About {APP_NAME}"))
        .parent(Some(parent.handle))
        .build(&mut window)?;

    let avatar = decode_avatar();
    let mut avatar_frame = nwg::ImageFrame::default();
    nwg::ImageFrame::builder()
        .parent(&window)
        .position((MARGIN, MARGIN))
        .size((AVATAR_SIZE, AVATAR_SIZE))
        .bitmap(avatar.as_ref())
        .build(&mut avatar_frame)?;

    let lines: [(&str, i32, i32); 5] = [
        (APP_NAME, MARGIN, 22),
        (&format!("Version {APP_VERSION}"), MARGIN + 26, 20),
        (&format!("by {AUTHOR}"), MARGIN + 48, 20),
        (&format!("Rust rewrite of {ORIGINAL_TOOL}"), MARGIN + 70, 20),
        (LICENSE_TEXT, MARGIN + 92, 20),
    ];
    let mut labels = Vec::new();
    for (text, y, height) in lines {
        let mut label = nwg::Label::default();
        nwg::Label::builder()
            .parent(&window)
            .text(text)
            .position((TEXT_LEFT, y))
            .size((TEXT_WIDTH, height))
            .build(&mut label)?;
        labels.push(label);
    }

    let mut repo_field = nwg::TextInput::default();
    nwg::TextInput::builder()
        .parent(&window)
        .text(REPO_URL)
        .readonly(true)
        .position((MARGIN, MARGIN * 2 + AVATAR_SIZE + 4))
        .size((DIALOG_SIZE.0 - MARGIN * 2 - 4, 24))
        .build(&mut repo_field)?;

    let mut disclaimer = nwg::Label::default();
    nwg::Label::builder()
        .parent(&window)
        .text("Unofficial; not affiliated with Ubiquiti Inc.")
        .position((MARGIN, MARGIN * 2 + AVATAR_SIZE + 36))
        .size((DIALOG_SIZE.0 - MARGIN * 2, 20))
        .build(&mut disclaimer)?;
    labels.push(disclaimer);

    let button_y = DIALOG_SIZE.1 - 48 - MARGIN;
    let mut open_btn = nwg::Button::default();
    nwg::Button::builder()
        .parent(&window)
        .text("Open GitHub")
        .position((DIALOG_SIZE.0 - MARGIN - 100 - 112, button_y))
        .size((104, 30))
        .build(&mut open_btn)?;
    let mut close_btn = nwg::Button::default();
    nwg::Button::builder()
        .parent(&window)
        .text("Close")
        .position((DIALOG_SIZE.0 - MARGIN - 100, button_y))
        .size((100, 30))
        .build(&mut close_btn)?;

    Ok(Dialog {
        window,
        _avatar: avatar,
        _avatar_frame: avatar_frame,
        _labels: labels,
        _repo_field: repo_field,
        open_btn,
        close_btn,
    })
}

/// Decode the embedded PNG avatar; the dialog still works without it.
fn decode_avatar() -> Option<nwg::Bitmap> {
    let decoder = nwg::ImageDecoder::new().ok()?;
    let frame = decoder.from_stream(AVATAR_PNG).ok()?.frame(0).ok()?;
    frame.as_bitmap().ok()
}
