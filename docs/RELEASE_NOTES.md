# RustTime v0.1.0

Primera versión pública de RustTime.

## Novedades

- Programación de **apagado** o **reinicio** de Windows por horas y minutos.
- Cancelación de la operación pendiente y modo «deshabilitar apagado automático».
- Información de discos: tipo, capacidad y **horas totales de encendido** (S.M.A.R.T., requiere administrador).
- Información de batería: **ciclos**, **carga actual** y **salud**.
- Interfaz horizontal, centrada, con alto dinámico y ventana no maximizable.
- **Idioma automático:** español si Windows está en español; inglés en cualquier otro caso.
- Icono de aplicación, créditos de desarrollo y verificación de datos cada 5 minutos.

## Artefactos de esta release

| Archivo | Descripción |
| --- | --- |
| `RustTime-v0.1.0-setup.exe` | Instalador con Inno Setup (accesos directos y desinstalador). |
| `RustTime-v0.1.0-portable.zip` | Versión portable: contiene `RustTime.exe`, README, LICENSE y CHANGELOG. |
| `RustTime-v0.1.0-checksums.txt` | Hashes SHA-256 de los dos archivos anteriores. |

## Verificación

```powershell
Get-FileHash .\RustTime-v0.1.0-setup.exe -Algorithm SHA256
Get-FileHash .\RustTime-v0.1.0-portable.zip -Algorithm SHA256
```

## Requisitos

Windows 10 u 11. Para leer las horas S.M.A.R.T. de los discos, ejecuta la aplicación como administrador.

---

Desarrollo: **vecttrz** · 2026 · Licencia MIT.
