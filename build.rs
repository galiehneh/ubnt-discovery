#[cfg(windows)]
fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("resources/app.ico");
    res.set_manifest_file("resources/app.manifest");
    res.set("ProductName", "LAN Device Discovery");
    res.set("FileDescription", "LAN Device Discovery");
    if let Err(err) = res.compile() {
        panic!("failed to compile Windows resources: {err}");
    }
}

#[cfg(not(windows))]
fn main() {}
