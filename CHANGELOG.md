# Changelog

Todos los cambios relevantes de RustTime se documentan aquí.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/)
y el proyecto usa [Versionado Semántico](https://semver.org/lang/es/).

## [0.1.0] - 2026-11-23

### Añadido
- Programación de apagado o reinicio de Windows por horas y minutos.
- Cancelación de la operación pendiente y modo «deshabilitar apagado automático».
- Cuenta atrás y estado de cada operación.
- Información de discos físicos: tipo, capacidad y horas totales de encendido (S.M.A.R.T.).
- Información de batería: ciclos consumidos, carga actual y salud.
- Botón para relanzar la aplicación como administrador cuando S.M.A.R.T. no está disponible.
- Interfaz horizontal de dos columnas, centrada, con alto dinámico y ventana no maximizable.
- Detección automática de idioma: español si Windows está en español, inglés en cualquier otro caso.
- Icono de aplicación incrustado en el ejecutable y en la ventana.
- Créditos de desarrollo: vecttrz · 2026.
- Artefactos de release: instalador (Inno Setup) y versión portable (ZIP).

[0.1.0]: https://github.com/vecttrz/RustTime/releases/tag/v0.1.0
