fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut res = winresource::WindowsResource::new();
    res.set_icon("resources/app.ico");
    res.set_manifest_file("resources/app.manifest");
    res.set("ProductName", "LAN Device Discovery");
    res.set("FileDescription", "LAN Device Discovery");
    if let Err(err) = res.compile() {
        panic!("failed to compile Windows resources: {err}");
    }
}
