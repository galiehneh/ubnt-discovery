//! Main window: toolbar, device list, status bar and scan orchestration.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use native_windows_gui as nwg;
use nwg::ListViewColumnSortArrow as Arrow;
use ubnt_discovery::protocol::{client, DiscoveredDevice};

use super::model::{
    autostart_text, build_tsv, countdown_text, filter_and_sort, row_cells, total_text, SortSpec,
    COLUMNS, SCAN_SECONDS,
};
use super::{about_window, details_window, theme};

/// Seconds to wait after launch before the first scan starts by itself.
const AUTOSTART_DELAY_SECS: u64 = 4;
const TITLE: &str = "LAN Device Discovery";
const MESSAGE_TITLE: &str = "Discovery";
const TOOLBAR_HEIGHT: u32 = 44;
const STATUS_HEIGHT: u32 = 26;
const MIN_SIZE: (i32, i32) = (960, 360);
const BUTTON_WIDTH: i32 = 88;
/// Toolbar buttons (theme toggle, About) never sit left of this; the Search box ends before it.
const ABOUT_MIN_X: i32 = 850;
const ABOUT_MARGIN: i32 = 8;
const THEME_GAP: i32 = 6;
const STATUS_TEXT_INSET: i32 = 8;

/// Messages from the scan thread to the UI thread.
enum ScanMsg {
    Device(Box<DiscoveredDevice>),
    Finished,
}

struct ActiveScan {
    cancel: Arc<AtomicBool>,
    started: Instant,
}

struct State {
    devices: HashMap<String, DiscoveredDevice>,
    rows: Vec<DiscoveredDevice>,
    sort: SortSpec,
    scan: Option<ActiveScan>,
}

struct App {
    window: nwg::Window,
    _icon: Option<nwg::Icon>,
    scan_btn: nwg::Button,
    clear_btn: nwg::Button,
    details_btn: nwg::Button,
    copy_btn: nwg::Button,
    web_btn: nwg::Button,
    about_btn: nwg::Button,
    theme_btn: nwg::Button,
    _search_label: nwg::Label,
    search: nwg::TextInput,
    list: nwg::ListView,
    status: nwg::Label,
    timer: nwg::AnimationTimer,
    autostart_timer: nwg::AnimationTimer,
    autostart_left: Cell<u64>,
    notice: nwg::Notice,
    tx: Sender<ScanMsg>,
    rx: Receiver<ScanMsg>,
    state: RefCell<State>,
    rebuilding: Cell<bool>,
}

/// Build and run the main window; returns when it closes.
pub fn run() {
    if let Err(err) = nwg::init() {
        fatal(&format!("Failed to initialise the UI: {err}"));
    }
    let mut font = nwg::Font::default();
    if nwg::Font::builder()
        .family("Segoe UI")
        .size(18)
        .build(&mut font)
        .is_ok()
    {
        nwg::Font::set_global_default(Some(font));
    }

    theme::set_current(theme::load());
    let app = match App::build() {
        Ok(app) => Rc::new(app),
        Err(err) => fatal(&format!("Failed to create the main window: {err}")),
    };
    let handler = bind_events(&app);
    app.layout();
    app.apply_theme();
    app.refresh_list();
    app.begin_autostart();
    nwg::dispatch_thread_events();
    nwg::unbind_event_handler(&handler);
}

fn fatal(message: &str) -> ! {
    nwg::fatal_message(MESSAGE_TITLE, message)
}

fn bind_events(app: &Rc<App>) -> nwg::EventHandler {
    let app = Rc::clone(app);
    let window = app.window.handle;
    nwg::full_bind_event_handler(&window, move |evt, data, handle| {
        use nwg::Event as E;
        match evt {
            E::OnWindowClose if handle == app.window.handle => app.shutdown(),
            E::OnMinMaxInfo => data.on_min_max().set_min_size(MIN_SIZE.0, MIN_SIZE.1),
            E::OnResize | E::OnWindowMaximize => app.layout(),
            E::OnNotice => app.drain_scan_messages(),
            E::OnTimerTick if handle == app.autostart_timer.handle => app.on_autostart_tick(),
            E::OnTimerTick => app.on_tick(),
            E::OnTextInput if handle == app.search.handle => app.refresh_list(),
            E::OnButtonClick => app.on_button(handle),
            E::OnListViewColumnClick => app.on_column_click(data.on_list_view_item_index().1),
            E::OnListViewDoubleClick => app.show_details(),
            E::OnListViewItemChanged | E::OnListViewClick => app.update_buttons(),
            _ => {}
        }
    })
}

impl App {
    fn build() -> Result<App, nwg::NwgError> {
        let icon = load_icon();
        let mut window = nwg::Window::default();
        nwg::Window::builder()
            .flags(nwg::WindowFlags::MAIN_WINDOW | nwg::WindowFlags::VISIBLE)
            .size((970, 560))
            .center(true)
            .title(TITLE)
            .icon(icon.as_ref())
            .build(&mut window)?;

        let button = |text: &str, x: i32| -> Result<nwg::Button, nwg::NwgError> {
            let mut b = nwg::Button::default();
            nwg::Button::builder()
                .parent(&window)
                .text(text)
                .position((x, 8))
                .size((BUTTON_WIDTH, 28))
                .build(&mut b)?;
            Ok(b)
        };
        let scan_btn = button("Scan", 8)?;
        let clear_btn = button("Clear", 102)?;
        let details_btn = button("Details...", 196)?;
        let copy_btn = button("Copy", 290)?;
        let web_btn = button("Web UI", 384)?;
        let about_btn = button("About", ABOUT_MIN_X)?;
        let theme_btn = button(
            theme::current().toggle_label(),
            ABOUT_MIN_X - BUTTON_WIDTH - THEME_GAP,
        )?;

        let mut search_label = nwg::Label::default();
        nwg::Label::builder()
            .parent(&window)
            .text("Search:")
            .position((490, 14))
            .size((52, 20))
            .build(&mut search_label)?;
        let mut search = nwg::TextInput::default();
        nwg::TextInput::builder()
            .parent(&window)
            .position((546, 10))
            .size((200, 24))
            .build(&mut search)?;

        let list = build_list(&window)?;
        let mut status = nwg::Label::default();
        nwg::Label::builder()
            .parent(&window)
            .text(&total_text(0))
            .size((300, STATUS_HEIGHT as i32 - 6))
            .build(&mut status)?;
        theme::attach(&window, Some(&status));
        let mut timer = nwg::AnimationTimer::default();
        nwg::AnimationTimer::builder()
            .parent(&window)
            .interval(Duration::from_secs(1))
            .build(&mut timer)?;
        let mut autostart_timer = nwg::AnimationTimer::default();
        nwg::AnimationTimer::builder()
            .parent(&window)
            .interval(Duration::from_secs(1))
            .build(&mut autostart_timer)?;
        let mut notice = nwg::Notice::default();
        nwg::Notice::builder().parent(&window).build(&mut notice)?;

        let (tx, rx) = mpsc::channel();
        Ok(App {
            window,
            _icon: icon,
            scan_btn,
            clear_btn,
            details_btn,
            copy_btn,
            web_btn,
            about_btn,
            theme_btn,
            _search_label: search_label,
            search,
            list,
            status,
            timer,
            autostart_timer,
            autostart_left: Cell::new(0),
            notice,
            tx,
            rx,
            state: RefCell::new(State {
                devices: HashMap::new(),
                rows: Vec::new(),
                sort: SortSpec::DEFAULT,
                scan: None,
            }),
            rebuilding: Cell::new(false),
        })
    }

    fn layout(&self) {
        let (w, h) = self.window.size();
        self.list.set_position(0, TOOLBAR_HEIGHT as i32);
        let list_h = h.saturating_sub(TOOLBAR_HEIGHT + STATUS_HEIGHT);
        self.list.set_size(w, list_h);
        let about_x = (w as i32 - BUTTON_WIDTH - ABOUT_MARGIN).max(ABOUT_MIN_X);
        self.about_btn.set_position(about_x, 8);
        self.theme_btn
            .set_position(about_x - BUTTON_WIDTH - THEME_GAP, 8);
        self.status.set_position(
            STATUS_TEXT_INSET,
            (h.saturating_sub(STATUS_HEIGHT) + 4) as i32,
        );
        self.status.set_size(
            w.saturating_sub(STATUS_TEXT_INSET as u32),
            STATUS_HEIGHT - 6,
        );
    }

    fn apply_theme(&self) {
        let theme = theme::current();
        self.theme_btn.set_text(theme.toggle_label());
        for button in [
            &self.scan_btn,
            &self.clear_btn,
            &self.details_btn,
            &self.copy_btn,
            &self.web_btn,
            &self.about_btn,
            &self.theme_btn,
        ] {
            theme::style_button(button);
        }
        theme::style_edit(&self.search);
        theme::style_list(&self.list);
        theme::style_window(&self.window);
    }

    fn toggle_theme(&self) {
        let theme = theme::current().toggled();
        theme::set_current(theme);
        // Failing to persist only loses the choice on next launch.
        let _ = theme::save(theme);
        self.apply_theme();
    }

    fn on_button(&self, handle: nwg::ControlHandle) {
        if handle == self.scan_btn.handle {
            self.toggle_scan();
        } else if handle == self.clear_btn.handle {
            self.state.borrow_mut().devices.clear();
            self.refresh_list();
        } else if handle == self.details_btn.handle {
            self.show_details();
        } else if handle == self.copy_btn.handle {
            self.copy_selection();
        } else if handle == self.web_btn.handle {
            self.open_web_ui();
        } else if handle == self.about_btn.handle {
            about_window::show(&self.window);
        } else if handle == self.theme_btn.handle {
            self.toggle_theme();
        }
    }

    // ---- scanning -------------------------------------------------------

    /// Count down on the Scan button, then start the first scan automatically.
    fn begin_autostart(&self) {
        self.autostart_left.set(AUTOSTART_DELAY_SECS);
        self.scan_btn
            .set_text(&autostart_text(AUTOSTART_DELAY_SECS));
        self.autostart_timer.start();
    }

    fn on_autostart_tick(&self) {
        let left = self.autostart_left.get().saturating_sub(1);
        self.autostart_left.set(left);
        if left > 0 {
            self.scan_btn.set_text(&autostart_text(left));
            return;
        }
        self.autostart_timer.stop();
        if self.state.borrow().scan.is_none() {
            self.start_scan();
        }
    }

    /// Abort a pending automatic scan (the user took over, or the app is closing).
    fn cancel_autostart(&self) {
        self.autostart_timer.stop();
        self.autostart_left.set(0);
    }

    fn toggle_scan(&self) {
        self.cancel_autostart();
        if let Some(scan) = &self.state.borrow().scan {
            scan.cancel.store(true, Ordering::SeqCst);
            return;
        }
        self.start_scan();
    }

    fn start_scan(&self) {
        let cancel = Arc::new(AtomicBool::new(false));
        let device_tx = Mutex::new(self.tx.clone());
        let done_tx = self.tx.clone();
        let sender = self.notice.sender();
        let thread_cancel = Arc::clone(&cancel);
        let spawned = std::thread::Builder::new()
            .name("scan".into())
            .spawn(move || {
                client::scan(Duration::from_secs(SCAN_SECONDS), thread_cancel, move |d| {
                    if let Ok(tx) = device_tx.lock() {
                        let _ = tx.send(ScanMsg::Device(Box::new(d)));
                    }
                    sender.notice();
                });
                let _ = done_tx.send(ScanMsg::Finished);
                sender.notice();
            });
        if let Err(err) = spawned {
            self.warn(&format!("Scan failed: {err}"));
            return;
        }
        self.state.borrow_mut().scan = Some(ActiveScan {
            cancel,
            started: Instant::now(),
        });
        self.scan_btn.set_text(&countdown_text(0));
        self.timer.start();
    }

    fn end_scan(&self) {
        self.state.borrow_mut().scan = None;
        self.timer.stop();
        self.scan_btn.set_text("Scan");
    }

    fn on_tick(&self) {
        let elapsed = self
            .state
            .borrow()
            .scan
            .as_ref()
            .map(|s| s.started.elapsed().as_secs());
        if let Some(elapsed) = elapsed {
            self.scan_btn.set_text(&countdown_text(elapsed));
        }
    }

    fn drain_scan_messages(&self) {
        let mut changed = false;
        let mut finished = false;
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                ScanMsg::Device(d) => {
                    let key = d.mac.to_uppercase();
                    self.state.borrow_mut().devices.insert(key, *d);
                    changed = true;
                }
                ScanMsg::Finished => finished = true,
            }
        }
        if changed {
            self.refresh_list();
        }
        if finished {
            self.end_scan();
        }
    }

    fn shutdown(&self) {
        if let Some(scan) = &self.state.borrow().scan {
            scan.cancel.store(true, Ordering::SeqCst);
        }
        self.timer.stop();
        self.cancel_autostart();
        super::QUIT_REQUESTED.store(true, Ordering::SeqCst);
        nwg::stop_thread_dispatch();
    }

    // ---- list ------------------------------------------------------------

    fn on_column_click(&self, column: usize) {
        {
            let mut st = self.state.borrow_mut();
            st.sort = st.sort.toggled(column);
        }
        self.refresh_list();
    }

    fn update_sort_arrows(&self, sort: SortSpec) {
        for column in 0..COLUMNS.len() {
            let arrow = (column == sort.column).then_some(if sort.ascending {
                Arrow::Up
            } else {
                Arrow::Down
            });
            self.list.set_column_sort_arrow(column, arrow);
        }
    }

    fn selected_macs(&self) -> Vec<String> {
        self.selected_devices().into_iter().map(|d| d.mac).collect()
    }

    fn selected_devices(&self) -> Vec<DiscoveredDevice> {
        let st = self.state.borrow();
        let mut indices = self.list.selected_items();
        indices.sort_unstable();
        indices
            .into_iter()
            .filter_map(|i| st.rows.get(i).cloned())
            .collect()
    }

    fn refresh_list(&self) {
        let selected = self.selected_macs();
        let rows = {
            let st = self.state.borrow();
            filter_and_sort(st.devices.values(), &self.search.text(), st.sort)
        };
        let sort = self.state.borrow().sort;
        self.rebuilding.set(true);
        self.list.set_redraw(false);
        self.list.clear();
        for (i, device) in rows.iter().enumerate() {
            let cells = row_cells(device);
            self.list.insert_items_row(None, &cells);
            if selected.contains(&device.mac) {
                self.list.select_item(i, true);
            }
        }
        self.list.set_redraw(true);
        self.rebuilding.set(false);
        self.status.set_text(&total_text(rows.len()));
        self.state.borrow_mut().rows = rows;
        self.update_sort_arrows(sort);
        self.update_buttons();
    }

    fn update_buttons(&self) {
        if self.rebuilding.get() {
            return;
        }
        let selected = self.selected_devices();
        let has_rows = !self.state.borrow().rows.is_empty();
        let first = selected.first();
        self.details_btn.set_enabled(first.is_some());
        self.copy_btn.set_enabled(first.is_some());
        self.web_btn
            .set_enabled(first.is_some_and(|d| d.ip.is_some()));
        self.clear_btn.set_enabled(has_rows);
    }

    // ---- actions ---------------------------------------------------------

    fn show_details(&self) {
        let selected = self.selected_devices();
        if let Some(device) = selected.first() {
            details_window::show(&self.window, device);
        }
    }

    fn copy_selection(&self) {
        let rows: Vec<[String; 6]> = self.selected_devices().iter().map(row_cells).collect();
        if rows.is_empty() {
            return;
        }
        let text = build_tsv(&rows);
        nwg::Clipboard::set_data_text(&self.window, &text);
        if nwg::Clipboard::data_text(&self.window).as_deref() != Some(text.as_str()) {
            self.warn("Copy failed: the clipboard did not accept the text.");
        }
    }

    fn open_web_ui(&self) {
        let selected = self.selected_devices();
        let Some(ip) = selected.first().and_then(|d| d.ip) else {
            return;
        };
        let url = format!("http://{ip}");
        if opener::open(&url).is_err() {
            self.warn(&format!(
                "Failed launching web browser. Please open {url} manually."
            ));
        }
    }

    fn warn(&self, content: &str) {
        nwg::modal_message(
            &self.window,
            &nwg::MessageParams {
                title: MESSAGE_TITLE,
                content,
                buttons: nwg::MessageButtons::Ok,
                icons: nwg::MessageIcons::Warning,
            },
        );
    }
}

fn build_list(window: &nwg::Window) -> Result<nwg::ListView, nwg::NwgError> {
    let mut list = nwg::ListView::default();
    nwg::ListView::builder()
        .parent(window)
        .position((0, TOOLBAR_HEIGHT as i32))
        .size((900, 400))
        .list_style(nwg::ListViewStyle::Detailed)
        .flags(
            nwg::ListViewFlags::VISIBLE
                | nwg::ListViewFlags::TAB_STOP
                | nwg::ListViewFlags::ALWAYS_SHOW_SELECTION,
        )
        .ex_flags(nwg::ListViewExFlags::FULL_ROW_SELECT | nwg::ListViewExFlags::GRID)
        .build(&mut list)?;
    for (index, (title, width)) in COLUMNS.iter().enumerate() {
        list.insert_column(nwg::InsertListViewColumn {
            index: Some(index as i32),
            fmt: Some(nwg::ListViewColumnFlags::LEFT),
            width: Some(*width),
            text: Some((*title).to_string()),
        });
    }
    list.set_headers_enabled(true);
    Ok(list)
}

/// Icon from the exe's embedded resource (id 1, written by build.rs).
fn load_icon() -> Option<nwg::Icon> {
    let embed = nwg::EmbedResource::load(None).ok()?;
    nwg::Icon::from_embed(&embed, Some(1), None).ok()
}
