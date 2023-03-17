fn main() {
    let mut res = tauri_winres::WindowsResource::new();
    res.set_icon_with_id("icon.ico", "1");
    res.compile().unwrap();
}
