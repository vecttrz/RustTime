fn main() {
    // Embeds Icon.ico as the executable icon (Explorer, taskbar, etc.).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("design/Icon.ico");
        resource
            .compile()
            .expect("Failed to compile the icon resource");
    }
}
