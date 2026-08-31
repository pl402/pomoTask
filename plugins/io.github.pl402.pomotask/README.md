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

```
plugins/io.github.pl402.pomotask/
├── manifest.json         # Manifiesto del plugin (schemaVersion: 1)
├── PomotaskService.qml   # Servicio singleton / enlace IPC y FileWatcher
├── BarWidget.qml         # Componente de la barra de estado
├── Panel.qml             # Panel emergente con temporizador y tareas
├── DistractionMonitor.qml# Monitor de ventanas de Hyprland
├── BreakOverlay.qml      # Overlay de descanso y bloqueo estricto
└── README.md             # Este archivo
```

## Validación

Para validar el plugin con Omarchy:

```bash
omarchy plugin validate plugins/io.github.pl402.pomotask
```
