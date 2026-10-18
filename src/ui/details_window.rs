//! Modal device details dialog.

use std::sync::atomic::Ordering;

use native_windows_gui as nwg;
use ubnt_discovery::protocol::DiscoveredDevice;

use super::model::{address_rows, or_unknown};
use super::theme;

const FIELD_LABEL_WIDTH: i32 = 100;
const ROW_HEIGHT: i32 = 26;
const MARGIN: i32 = 12;
const DIALOG_SIZE: (i32, i32) = (460, 440);

struct Dialog {
    window: nwg::Window,
    _labels: Vec<nwg::Label>,
    _fields: Vec<nwg::TextInput>,
    _heading: nwg::Label,
    _list: nwg::ListView,
    close_btn: nwg::Button,
}

/// Show the modal device details dialog; returns once it is closed.
pub fn show(parent: &nwg::Window, device: &DiscoveredDevice) {
    let dialog = match build(parent, device) {
        Ok(d) => d,
        Err(err) => {
            nwg::error_message("Discovery", &format!("Could not open details: {err}"));
            return;
        }
    };
    let window_handle = dialog.window.handle;
    let close_handle = dialog.close_btn.handle;
    let handler = nwg::full_bind_event_handler(&window_handle, move |evt, data, handle| {
        use nwg::Event as E;
        let close = match evt {
            E::OnWindowClose if handle == window_handle => {
                // Defer destruction until `show` has re-enabled the owner, otherwise
                // Windows activates an unrelated top-level window.
                if let nwg::EventData::OnWindowClose(close_data) = data {
                    close_data.close(false);
                }
                true
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
        // The main window was closed while this dialog's nested loop was running and
        // consumed the single WM_QUIT; pass it on so `run()` exits too.
        nwg::stop_thread_dispatch();
    } else {
        parent.set_focus();
    }
}

fn build(parent: &nwg::Window, device: &DiscoveredDevice) -> Result<Dialog, nwg::NwgError> {
    let title = format!("Device Details \u{2014} {}", device.display_product());
    let mut window = nwg::Window::default();
    nwg::Window::builder()
        .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
        .size(DIALOG_SIZE)
        .center(true)
        .title(&title)
        .parent(Some(parent.handle))
        .build(&mut window)?;

    let uptime = device.display_uptime();
    let values: [(&str, String); 7] = [
        ("Product Name:", device.display_product().to_string()),
        ("HW Address:", device.mac.clone()),
        ("IP Address:", device.display_ip()),
        ("System Name:", or_unknown(device.hostname.as_deref())),
        ("SSID:", or_unknown(device.essid.as_deref())),
        ("Version:", or_unknown(device.firmware.as_deref())),
        ("Uptime:", or_unknown(Some(uptime.as_str()))),
    ];
    let mut labels = Vec::new();
    let mut fields = Vec::new();
    let field_width = DIALOG_SIZE.0 - FIELD_LABEL_WIDTH - MARGIN * 3 - 4;
    for (i, (name, value)) in values.iter().enumerate() {
        let y = MARGIN + i as i32 * ROW_HEIGHT;
        let mut label = nwg::Label::default();
        nwg::Label::builder()
            .parent(&window)
            .text(name)
            .position((MARGIN, y + 3))
            .size((FIELD_LABEL_WIDTH, 20))
            .build(&mut label)?;
        let mut field = nwg::TextInput::default();
        nwg::TextInput::builder()
            .parent(&window)
            .text(value)
            .readonly(true)
            .position((MARGIN * 2 + FIELD_LABEL_WIDTH, y))
            .size((field_width, 22))
            .build(&mut field)?;
        labels.push(label);
        fields.push(field);
    }

    let list_top = MARGIN + values.len() as i32 * ROW_HEIGHT + 8;
    let mut heading = nwg::Label::default();
    nwg::Label::builder()
        .parent(&window)
        .text("Device Network Addresses")
        .position((MARGIN, list_top))
        .size((DIALOG_SIZE.0 - MARGIN * 2, 20))
        .build(&mut heading)?;

    let list_height = DIALOG_SIZE.1 - list_top - 20 - 48 - MARGIN - 36;
    let list = build_address_list(&window, device, list_top + 24, list_height)?;

    let mut close_btn = nwg::Button::default();
    nwg::Button::builder()
        .parent(&window)
        .text("Close")
        .position((DIALOG_SIZE.0 - MARGIN - 100, DIALOG_SIZE.1 - 48 - MARGIN))
        .size((100, 30))
        .build(&mut close_btn)?;

    theme::attach(&window, None);
    theme::style_window(&window);
    for field in &fields {
        theme::style_edit(field);
    }
    theme::style_list(&list);
    theme::style_button(&close_btn);

    Ok(Dialog {
        window,
        _labels: labels,
        _fields: fields,
        _heading: heading,
        _list: list,
        close_btn,
    })
}

fn build_address_list(
    window: &nwg::Window,
    device: &DiscoveredDevice,
    top: i32,
    height: i32,
) -> Result<nwg::ListView, nwg::NwgError> {
    let mut list = nwg::ListView::default();
    nwg::ListView::builder()
        .parent(window)
        .position((MARGIN, top))
        .size((DIALOG_SIZE.0 - MARGIN * 2 - 4, height))
        .list_style(nwg::ListViewStyle::Detailed)
        .ex_flags(nwg::ListViewExFlags::FULL_ROW_SELECT | nwg::ListViewExFlags::GRID)
        .build(&mut list)?;
    for (index, (title, width)) in [("Hardware Address", 200), ("IP Address", 200)]
        .iter()
        .enumerate()
    {
        list.insert_column(nwg::InsertListViewColumn {
            index: Some(index as i32),
            fmt: Some(nwg::ListViewColumnFlags::LEFT),
            width: Some(*width),
            text: Some((*title).to_string()),
        });
    }
    list.set_headers_enabled(true);
    for (mac, ip) in address_rows(device) {
        list.insert_items_row(None, &[mac, ip]);
    }
    Ok(list)
}
