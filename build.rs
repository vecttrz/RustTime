fn main() {
    // Incorpora Icon.ico como icono del ejecutable (Explorador, barra de tareas, etc.).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("design/Icon.ico");
        resource
            .compile()
            .expect("No se pudo compilar el recurso del icono");
    }
}
