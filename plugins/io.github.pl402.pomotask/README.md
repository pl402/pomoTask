# PomoTask Omarchy Shell Plugin (`io.github.pl402.pomotask`)

Plugin nativo de barra y escritorio para **Omarchy Quattro** que integra la técnica **Pomodoro**, gestión de tareas de **Google Tasks**, bloqueo anti-distracciones en **Hyprland** y descanso estricto con bloqueo de pantalla.

## Características

- 🍅 **Indicador en la Barra de Omarchy (`BarWidget.qml`):**
  - Muestra el glifo de estado (trabajo, descanso corto, descanso largo o pausa), el tiempo restante (`MM:SS`) y un anillo de progreso que se llena conforme avanza la fase.
  - Ancho siempre constante: el título de la tarea activa no va en la barra (salía cortado), se muestra en el tooltip junto con el modo, el porcentaje y el estado.
  - Clic interactivo para desplegar el panel de control o alternar el temporizador.
- 📋 **Panel Desplegable Interactivo (`Panel.qml`):**
  - Héroe del temporizador: anillo de progreso con el tiempo dentro y cuatro puntos de ciclo. El estado (en curso, pausado, listo) se lee en el propio anillo; los colores de fase derivan del tema (acento para enfoque, acento atenuado en descansos).
  - Controles con jerarquía: Iniciar/Pausar grande como acción primaria; Reiniciar (pide una segunda pulsación en 3 s) y Saltar pequeños a los lados.
  - Tarjeta de la **tarea en foco** con título completo, descripción, subtareas marcables y botón Completar (completar la tarea en foco detiene el pomodoro en curso y deja el reloj listo para la siguiente). Sin tarea en foco se muestra un estado vacío que explica qué hacer; en descansos, una tarjeta que recuerda a qué tarea vuelves.
  - Lista de Google Tasks con selector de listas, contador de pendientes, fecha límite y pomodoros por tarea, y acciones copiar 󰆏 (título + descripción vía `wl-copy`) y enfocar por fila. Mientras corre el temporizador la lista se pliega a una fila "Tareas · N pendientes" que se despliega al clic.
  - Navegación por teclado en la lista: `J`/`K` mueven el cursor, `Enter` enfoca, `C` completa, `Y` copia, `Esc` quita el cursor. Además `Espacio`/`P` alterna el temporizador, `S` salta la fase, `R` sincroniza y `Q` cierra. El botón `?` del pie muestra la ayuda.
  - Pie con el resumen de **hoy** (pomodoros, tareas completadas y tiempo de foco) leído de `stats.json`.
  - Creación rápida de nuevas tareas con `Enter`.
  - Ajustes: modo como grupo de chips, **duraciones editables** en minutos (persisten en `config.json` vía `pomotask-cli ipc config set`), toggles de anti-distracción, bloqueo de pantalla en descansos y **ciclo automático** (al terminar el trabajo arranca solo el descanso, y al terminar el descanso vuelve solo al trabajo **si hay tarea en foco**; sin tarea el trabajo queda preparado pero detenido y la instancia líder avisa para que se elija una; `auto_cycle` en `runtime_state.json`, se alterna con `pomotask-cli ipc timer toggle-auto`), y listas de bloqueo en pestañas (títulos web, apps, excepciones).
- 🛡️ **Monitor Anti-distracciones (`DistractionMonitor.qml`):**
  - Inspección en tiempo real de títulos y clases de ventana activas en Hyprland.
  - Tres acciones al detectar una distracción en modo trabajo, elegibles en Ajustes y visualmente distintas:
    - **Aviso discreto abajo** (`warn`): tarjeta pequeña abajo al centro, estilo OSD, con la distracción, la tarea y el tiempo restante. No tapa nada.
    - **Pantalla de enfoque** (`hud`): oscurece toda la pantalla (oscurecimiento configurable) y pone encima la tarea, el reloj y el progreso.
    - **Ocultar ventana hasta el descanso** (`minimize`): la ventana se va a `special:minimized` y aparece el aviso discreto explicándolo; al terminar o pausar el trabajo las ventanas vuelven solas a su workspace original. Si se abre el workspace especial para mirarla, se vuelve a cerrar. Ojo: si era la única ventana del escritorio, Hyprland le deja el foco de teclado aunque no se vea; el monitor comprueba con `hyprctl -j monitors` si el especial está realmente abierto antes de cerrarlo (alternarlo a ciegas lo abría y producía un ciclo mostrar/ocultar).
  - Las acciones sobre ventanas usan la API Lua de `hyprctl dispatch` (`hl.dsp.window.move`, `hl.dsp.workspace.toggle_special`), obligatoria desde Hyprland 0.56; la instancia líder es la única que las ejecuta. Los valores antiguos `warn_and_unfocus`/`unfocus` se leen como `hud`.
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

### Varios monitores: una instancia líder

La barra instancia `BarWidget.qml` una vez por monitor, cada una con su `PomotaskService` y sus overlays. Para que los efectos globales ocurran una sola vez, el widget elige una instancia **líder** (la primera que devuelve `bar.moduleWidgets(moduleName)`, reelegida cada 5 s) y solo ella lanza la notificación y el bloqueo del descanso estricto, las acciones de Hyprland del monitor anti-distracciones, la sincronización automática y la comprobación inicial de sesión. Las demás instancias siguen dibujando sus overlays y leen el estado de los archivos compartidos.

### Buzón de salida (cambios sin conexión)

Si creas o completas una tarea desde el panel y Google no responde, el cambio se guarda en `~/.config/pomotask/outbox.json` además de la caché local. Cada sincronización (desde el plugin o desde la TUI) intenta subir primero lo pendiente; al crear la tarea en Google se sustituye su id temporal (`task_…`) por el real, conservando los pomodoros contabilizados. Lo que no se pueda subir se re-aplica sobre las tareas descargadas, así que ya no se pierde al pisar la caché. El panel muestra "N cambios pendientes de subir" mientras el buzón no esté vacío. `pomotask-cli ipc outbox` lo lista.

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
