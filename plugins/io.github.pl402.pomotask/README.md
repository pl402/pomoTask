# PomoTask Omarchy Shell Plugin (`io.github.pl402.pomotask`)

Plugin nativo de barra y escritorio para **Omarchy Quattro** que integra la técnica **Pomodoro**, gestión de tareas de **Google Tasks**, bloqueo anti-distracciones en **Hyprland** y descanso estricto con bloqueo de pantalla.

## Características

- 🍅 **Indicador en la Barra de Omarchy (`BarWidget.qml`):**
  - Muestra el tiempo restante (`MM:SS`) y el estado actual (`🍅` trabajo, `☕` descanso corto, `🌴` descanso largo).
  - Clic interactivo para desplegar el panel de control o alternar el temporizador.
- 📋 **Panel Desplegable Interactivo (`Panel.qml`):**
  - Controles del temporizador: Play / Pausa / Saltar / Reiniciar.
  - Sincronización bidireccional y navegación de Google Tasks con selector de listas.
  - Tareas en foco (🎯) para asociar el objetivo actual al temporizador.
  - Creación rápida de nuevas tareas con `Enter`.
  - Toggles rápidos para activar/desactivar modo anti-distracción y descanso estricto.
- 🛡️ **Monitor Anti-distracciones (`DistractionMonitor.qml`):**
  - Inspección en tiempo real de títulos y clases de ventana activas en Hyprland.
  - Advertencias instantáneas cuando se detectan sitios o aplicaciones distractoras en modo trabajo.
- 🔒 **Bloqueo Estricto en Descansos (`BreakOverlay.qml`):**
  - Bloqueo de pantalla automático (`omarchy system lock`) o recordatorio inmersivo al entrar en pausas de descanso.

## Arquitectura

El plugin se comunica con el motor Rust `pomotask-cli` mediante subcomandos IPC (`pomotask-cli ipc ...`) y observación reactiva de archivos de estado compartidos (`runtime_state.json`, `tasks_cache.json`, `blocklist.json`) en `~/.config/pomotask/`.

### Estado de la conexión con Google Tasks

El CLI escribe en `runtime_state.json` tres campos que el plugin observa:

- `google_connected` (`true`/`false`/ausente): último resultado conocido.
- `last_sync_at`: timestamp Unix de la última sincronización completa exitosa.
- `last_sync_error`: motivo del fallo. Si empieza por `auth_required` o `no_token`, hay que volver a iniciar sesión desde la TUI (`pomotask-cli`).

Cuando `google_connected` es `false` el widget de la barra se tinta con el color *urgent* del tema y muestra un triángulo de aviso; el panel muestra un banner con el motivo y un botón para abrir la TUI (re-autenticar) o reintentar. El servicio comprueba la sesión con `pomotask-cli ipc auth-status` a los pocos segundos de arrancar y sincroniza automáticamente cada 10 minutos.

```
plugins/io.github.pl402.pomotask/
├── manifest.json         # Manifiesto del plugin (schemaVersion: 1)
├── PomotaskService.qml   # Servicio singleton / enlace IPC y FileWatcher
├── BarWidget.qml         # Componente de la barra de estado
├── Panel.qml             # Panel emergente con temporizador y tareas
├── DistractionMonitor.qml# Monitor de ventanas de Hyprland
├── DistractionOverlay.qml# Overlay inferior visual de distracción activa
├── BreakOverlay.qml      # Overlay de descanso y bloqueo estricto
└── README.md             # Este archivo
```

## Validación

Para validar el plugin con Omarchy:

```bash
omarchy plugin validate plugins/io.github.pl402.pomotask
```
