# Spec de Diseño: Plugin de PomoTask para Omarchy Quattro

**Fecha:** 31 de Agosto de 2026  
**Estado:** Aprobado  
**Identificador del Plugin:** `io.github.pomotask.bar`  

---

## 1. Visión General y Objetivos

El objetivo de este proyecto es integrar y extender las capacidades de **pomoTask** (aplicación Pomodoro + Google Tasks escrita en Rust) creando un plugin nativo de primera clase para el entorno de escritorio **Omarchy Quattro** (basado en Quickshell y Hyprland).

El plugin no es un simple clon de la TUI, sino una experiencia de escritorio integrada que añade:
1. **Control de Pomodoro y Tareas en la Barra de Omarchy:** Indicador en vivo, reloj de cuenta regresiva, estado del ciclo y panel desplegable con lista interactiva de Google Tasks.
2. **Monitor Anti-distracciones en Tiempo Real (Hyprland):** Detección de títulos de ventanas y clases de aplicaciones bloqueadas (ej. Facebook, Twitter, Reddit, YouTube, Discord, Steam) durante las sesiones de enfoque, emitiendo alertas inmediatas o disuadiendo el uso.
3. **Bloqueo Estricto en Descansos:** Capacidad de forzar el descanso mediante el bloqueo de pantalla (`omarchy system lock`) o un overlay inmersivo de pausa activa para evitar el agotamiento y asegurar el descanso efectivo.
4. **Arquitectura Híbrida Compartida:** Reutilización completa del motor Rust (`pomotask-cli`), su sistema de OAuth2, caché de listas/tareas y persistencia de estadísticas sin duplicar lógica de red.

---

## 2. Arquitectura del Sistema

```
 ┌─────────────────────────────────────────────────────────────┐
 │                      Omarchy Quattro                        │
 │  ┌───────────────────────────────────────────────────────┐  │
 │  │        Plugin: io.github.pomotask.bar (QML)           │  │
 │  │  • BarWidget.qml (Barra de estado / Reloj)            │  │
 │  │  • Panel.qml (Panel de control, Google Tasks, Foco)   │  │
 │  │  • DistractionMonitor.qml (Hyprland socket / IPC)     │  │
 │  │  • BreakOverlay.qml (Pantalla de descanso / Bloqueo)  │  │
 │  └───────────────┬──────────────────────▲────────────────┘  │
 └──────────────────┼──────────────────────┼───────────────────┘
                    │                      │
       CLI / JSON   │                      │ Observa state.json
       Subcommands  │                      │ (FileWatcher / Polling)
                    ▼                      │
 ┌─────────────────────────────────────────┴───────────────────┐
 │                   Motor Rust (pomotask-cli)                 │
 │  • Subcomandos CLI (--ipc status / timer / tasks / complete)│
 │  • Google Tasks API Client + OAuth2 Token Manager           │
 │  • Runtime State: ~/.config/pomotask/runtime_state.json     │
 │  • Caché de Tareas: ~/.config/pomotask/tasks_cache.json     │
 │  • Bloqueo/Reglas: ~/.config/pomotask/blocklist.json        │
 └─────────────────────────────────────────────────────────────┘
```

---

## 3. Componente 1: Motor Rust (`pomotask-cli`)

El binario existente `pomotask-cli` se extiende con comandos CLI orientados a IPC/JSON para ser invocado de forma transparente por el plugin de Omarchy o scripts de automatización:

### 3.1. Subcomandos CLI
* `pomotask-cli ipc status`  
  Devuelve el estado actual en JSON:
  ```json
  {
    "state": "running", // "running", "paused", "stopped"
    "mode": "work",     // "work", "short_break", "long_break"
    "remaining_seconds": 1492,
    "total_seconds": 1500,
    "session_pomodoros": 3,
    "active_task_id": "MTEwMjkz...",
    "active_task_title": "Fix login authentication flow",
    "strict_break": true,
    "anti_distraction": true
  }
  ```

* `pomotask-cli ipc timer <start|pause|toggle|skip|reset>`  
  Controla el reloj de Pomodoro y actualiza de inmediato el archivo de estado en tiempo de ejecución.

* `pomotask-cli ipc tasks list [--list-id <id>]`  
  Devuelve la jerarquía de tareas y subtareas en formato JSON estructurado desde la caché local (y sincroniza si es necesario).

* `pomotask-cli ipc task complete <task-id>`  
  Marca una tarea como completada (optimistic UI) y lanza la sincronización asíncrona contra Google Tasks.

* `pomotask-cli ipc task create --title "<texto>" [--list-id <id>] [--parent <id>]`  
  Crea una nueva tarea o subtarea en Google Tasks.

* `pomotask-cli ipc task focus <task-id|clear>`  
  Establece la tarea actual asociada al temporizador de enfoque.

* `pomotask-cli ipc sync`  
  Fuerza la sincronización en segundo plano de todas las listas con Google Tasks.

### 3.2. Archivos de Estado Compartidos
* `~/.config/pomotask/runtime_state.json`: Estado en tiempo real del temporizador y tarea activa.
* `~/.config/pomotask/blocklist.json`: Reglas de títulos y aplicaciones bloqueadas.
* `~/.config/pomotask/tasks_cache.json`: Caché completa de Google Tasks por lista.

---

## 4. Componente 2: Plugin Nativo de Omarchy (`io.github.pomotask.bar`)

El plugin sigue el estándar de plugins de Omarchy Quattro y se instala en `~/.config/omarchy/plugins/io.github.pomotask.bar/`.

### 4.1. Manifiesto (`manifest.json`)
```json
{
  "schemaVersion": 1,
  "id": "io.github.pomotask.bar",
  "name": "PomoTask",
  "version": "1.0.0",
  "author": "PomoTask Team",
  "license": "MIT",
  "description": "Pomodoro timer, Google Tasks integration, and distraction blocker for Omarchy Quattro.",
  "kinds": ["bar-widget"],
  "entryPoints": {
    "barWidget": "BarWidget.qml"
  },
  "barWidget": {
    "displayName": "PomoTask",
    "category": "Productivity",
    "allowMultiple": false,
    "defaultSection": "center"
  }
}
```

### 4.2. `BarWidget.qml`
* Representa el item en la barra de Omarchy usando `qs.Ui.BarWidget` y `qs.Ui.WidgetButton`.
* Muestra el icono según el modo (`🍅` trabajo, `☕` descanso corto, `🌴` descanso largo, `⏸` pausado), el tiempo restante (`MM:SS`) y opcionalmente el título de la tarea en foco si hay espacio disponible.
* Clic izquierdo: Abre/cierra el panel desplegable (`Panel.qml`).
* Rueda del ratón o clic central: Toggle rápido de inicio/pausa.

### 4.3. `Panel.qml`
* Implementado con `qs.Ui.KeyboardPanel` y `qs.Ui.PanelKeyCatcher` con soporte para tecla `Escape` y temas dinámicos (`qs.Commons.Style`, `qs.Commons.Color`).
* **Sección Hero (Temporizador):**
  * Display grande del tiempo restante con barra de progreso circular o lineal.
  * Controles: Iniciar/Pausar, Saltar a siguiente fase, Resetear ciclo.
  * Indicador de ciclo: Pomodoros completados (ej. `● ● ● ○` - 3 de 4).
* **Sección de Tareas (Google Tasks):**
  * Selector desplegable de lista de tareas (`Dropdown`).
  * Lista con desplazamiento (`ListView`) de tareas y subtareas:
    * Checkbox interactivo para marcar como completada.
    * Indicador de foco (🎯) para vincular la tarea al reloj.
    * Notas o fecha de vencimiento visible si existe.
  * Campo de entrada rápido inferior (`TextField`) para añadir tareas con Enter.
* **Sección de Ajustes Rápidos:**
  * Toggle: `Modo Anti-distracciones` (on/off).
  * Toggle: `Bloqueo Estricto en Descanso` (on/off).
  * Botón para abrir la TUI completa en terminal si se desea edición avanzada.

---

## 5. Componente 3: Monitor Anti-distracciones y Bloqueo en Descanso

### 5.1. Detección de Ventanas con Hyprland
* `DistractionMonitor.qml` se ejecuta como componente en segundo plano dentro del plugin.
* Monitorea la ventana activa mediante `hyprctl activewindow -j` o el socket IPC de eventos de Hyprland (`activewindow>>...`).
* Cuando el temporizador está en modo `work`:
  * Compara `title` y `class` de la ventana activa contra las reglas de `blocklist.json`.
  * Reglas por defecto en `blocklist.json`:
    ```json
    {
      "title_keywords": ["facebook", "twitter", "x.com", "instagram", "reddit", "youtube.com", "tiktok", "netflix"],
      "blocked_classes": ["steam", "discord", "spotify"],
      "action": "warn_and_unfocus" // "warn", "warn_and_unfocus", "minimize"
    }
    ```
  * Si hay coincidencia:
    1. Lanza una notificación OSD (`omarchy.osd` o `notify-send`) advirtiendo: *"⚠️ Concentración activa: Estás trabajando en '[Tarea]'. Evita distracciones."*
    2. Si `action` incluye desenfoque: cambia el foco a la ventana anterior de trabajo o minimiza la ventana distractora con `hyprctl dispatch workspace previous`.

### 5.2. Bloqueo en Modo Descanso
* Cuando el temporizador pasa a `short_break` o `long_break`:
  * Si `strict_break` está habilitado:
    * Lanza `omarchy system lock` (usando el lockscreen nativo de Omarchy).
    * Alternativamente, muestra un `Overlay` en pantalla completa con un temporizador de cuenta regresiva, ejercicios de estiramiento y respiración guiada.
  * Notificación sonora y visual al iniciar y terminar el descanso.

---

## 6. Manejo de Errores y Casos Borde

1. **Falta de conexión o API de Google inalcanzable:**
   * El plugin opera fluidamente usando la caché local de tareas (`tasks_cache.json`). Si una acción offline ocurre, se aplica localmente y se encola la sincronización.
2. **Motor Rust no iniciado o primera ejecución:**
   * El plugin maneja llamadas CLI de forma segura. Si el estado aún no existe, inicializa con valores por defecto (25m trabajo / 5m descanso) y muestra estado listo.
3. **Múltiples instancias:**
   * La fuente de verdad del estado de tiempo de ejecución es el motor central (`runtime_state.json`), garantizando consistencia absoluta entre la TUI, el widget de la barra y el monitor de Hyprland.

---

## 7. Plan de Verificación

* **Pruebas de CLI / IPC en Rust:**
  * Tests unitarios para serialización/deserialización de comandos IPC (`cargo test`).
  * Validación de ejecución de subcomandos `ipc status`, `ipc timer toggle`, `ipc tasks list`.
* **Pruebas de QML / Omarchy Plugin:**
  * Validación del manifiesto: `omarchy plugin validate ~/.config/omarchy/plugins/io.github.pomotask.bar`.
  * Verificación con `qmllint` contra los módulos de `/usr/share/omarchy/shell`.
  * Prueba de apertura, renderizado de colores según tema de Omarchy, clics de tareas y actualización en vivo del reloj.
* **Pruebas de Anti-distracciones:**
  * Abrir navegador con pestaña de Facebook/YouTube durante modo trabajo y verificar la alerta inmediata.
  * Comprobar que en modo descanso no se emiten alertas de distracción.
