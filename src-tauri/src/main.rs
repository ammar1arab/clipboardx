#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    std::env::set_var("GTK_OVERLAY_SCROLLING", "0");
    clipboardx_lib::run()
}
