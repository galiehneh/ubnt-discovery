use std::sync::atomic::AtomicBool;

pub mod about_window;
pub mod details_window;
pub mod main_window;
pub mod model;

/// Set once the main window has asked the event loop to quit, so a nested
/// dialog loop that swallowed the WM_QUIT can re-post it.
pub(crate) static QUIT_REQUESTED: AtomicBool = AtomicBool::new(false);
