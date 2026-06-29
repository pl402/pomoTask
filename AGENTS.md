# AGENTS.md

Guía para agentes de IA (Claude Code, Cursor, Copilot, etc.) que trabajen en este repositorio.
Léela antes de modificar código. El README.md está orientado a usuarios finales; este archivo está orientado a desarrolladores y agentes.

## Qué es este proyecto

**PomoTask-CLI** es una aplicación de terminal (TUI) escrita en **Rust** que combina la técnica
**Pomodoro** con la gestión de tareas de **Google Tasks**. Sincroniza tareas/subtareas de forma
bidireccional contra la API de Google, muestra un calendario con estadísticas, un temporizador
Pomodoro inmersivo, y soporta temas y multi-idioma (Español/Inglés).

- Binario / crate: `pomotask-cli` (ver `Cargo.toml`)
- Edición de Rust: **2021**
- Tipo: aplicación de escritorio en terminal (no es una librería)

## Stack tecnológico

| Área | Crate |
| :--- | :--- |
| Framework TUI | `ratatui` 0.26 + `crossterm` 0.27 |
| Runtime async | `tokio` 1.37 (multi-thread, `features = ["full"]`) |
| HTTP | `reqwest` 0.12 + `hyper` 0.14 + `hyper-rustls` |
| Google Tasks | `google-tasks1` 5.0 |
| OAuth2 | `yup-oauth2` 9.0 (InstalledFlow vía redirect HTTP local) |
| Serialización | `serde` + `serde_json` |
| Fechas | `chrono` 0.4 (con conversión a zona horaria local) |
| Notificaciones | `notify-rust` |
| Portapapeles | `arboard` |
| Errores | `color-eyre` |
| Rutas de config | `dirs` |

## Arquitectura y mapa de archivos

El flujo es un bucle de eventos asíncrono. La red corre en tareas de `tokio` y se comunica con la
UI mediante un canal `mpsc::UnboundedChannel<Event>`. La UI nunca bloquea esperando la red.

```
src/
├── main.rs          Bucle principal: render → recibe Event → muta App. Maneja flags (--version/-v).
├── events.rs        enum Event + EventHandler (poll de teclado/mouse + Tick cada 50ms en una task tokio).
├── handler.rs       handle_key_events() + sync_tasks(). Toda la lógica de teclado vive aquí (~520 líneas).
├── api.rs           ApiClient: wrapper async de Google Tasks (fetch/create/update/toggle + OAuth + paginación).
├── app/
│   ├── mod.rs       Estado global App, structs (Task, Config, Stats), persistencia de config/stats.
│   └── i18n.rs      Motor de traducción (Español/Inglés) vía App::translate(key).
└── ui/
    ├── mod.rs       render() — despacha según AppMode a la vista correspondiente.
    ├── calendar.rs  Calendario: vistas Standard/Heatmap/Progress y rangos Month/Week/Day.
    ├── list.rs      Lista jerárquica de tareas/subtareas.
    ├── timer.rs     Pantalla del temporizador y modo enfoque (reloj gigante).
    ├── modals.rs    Modales: input, edición, confirmación, ayuda, ajustes, selector de listas, logout.
    └── palette.rs   Temas y colores (Catppuccin, Nord, Gruvbox, Dracula, Monokai, Solarized, Ocean, custom).
```

### Diagrama de componentes y flujo

```
                         ┌──────────────────────────────────────┐
                         │              main.rs                  │
                         │   bucle:  render → next() → mutar App  │
                         └───────┬───────────────────────▲───────┘
                                 │ &mut App              │ Event
                  render(App)    │                       │
                                 ▼                       │
        ┌────────────────────────────────┐   ┌──────────┴───────────────┐
        │            ui/ (vista)          │   │   events.rs EventHandler  │
        │  mod.rs despacha según AppMode  │   │  task tokio:              │
        │  calendar / list / timer /      │   │   • teclado/mouse (poll)  │
        │  modals / palette               │   │   • Tick cada 50 ms       │
        └────────────────────────────────┘   └──────────▲────────────────┘
                                 │ teclas               │
                                 ▼                       │ Event (ApiUpdate,
                    ┌────────────────────────┐           │  ListsUpdate, NeedsAuth,
                    │       handler.rs        │           │  ApiTaskCompleted/Failed)
                    │ handle_key_events()     │           │
                    │ sync_tasks()            │  tokio::spawn (no bloquea UI)
                    └───────────┬─────────────┘──────────►│
                                │ llama                    │
                                ▼                          │
                    ┌────────────────────────┐    canal mpsc::UnboundedChannel<Event>
                    │        api.rs           │
                    │ ApiClient (Arc)         │◄──► Google Tasks API (OAuth2 / hyper-rustls)
                    └────────────────────────┘
```

Regla de oro: **la red nunca corre en el hilo de la UI**. Toda I/O se lanza con `tokio::spawn`
y devuelve su resultado como un `Event` por el canal; `main()` es el único que muta `App`.

### Flujo de datos (importante)

1. `main()` crea `App`, `EventHandler` y un `ApiClient` (envuelto en `Arc`).
2. `EventHandler` corre una task tokio que emite `Event::Key/Mouse/Resize` y un `Event::Tick` cada 50 ms.
3. Las operaciones de red (en `handler.rs` / `api.rs`) se lanzan con `tokio::spawn` y, al terminar,
   envían un `Event` (`ApiUpdate`, `ListsUpdate`, `ApiTaskCompleted`, `ApiTaskFailed`, `NeedsAuth`) por el canal.
4. El bucle de `main()` consume cada `Event` y muta el `App`. El render se hace en cada iteración.
5. **Optimistic UI**: las tareas se insertan/tachan de inmediato en el estado local y se confirman
   (o revierten) cuando llega el `Event` de la API. Mirar `creating_task_temp_id` y `marking_done_task_id`.

### Estado de la app (`AppMode`)

`Loading, Timer, Auth, AuthSuccess, ListSelector, Input, SubtaskInput, Edit, ConfirmComplete, Help, Settings, ConfirmLogout`.
`ui::render` despacha según este modo. Para añadir una pantalla nueva: agregar variante a `AppMode`,
manejarla en `ui/mod.rs::render`, y rutear el teclado en `handler.rs`.

## Autenticación y archivos sensibles

- La app usa OAuth2 (InstalledFlow) con captura automática del código vía un servidor HTTP local
  (`InstalledFlowReturnMethod::HTTPRedirect`). La URL se abre con `open` y se copia al portapapeles.
- **Resolución de `client_secret.json`** (orden, ver `api.rs::ApiClient::new`):
  1. `~/.config/pomotask/client_secret.json`
  2. `./client_secret.json` (raíz del proyecto)
  3. Credenciales **incrustadas en el binario** vía `include_str!("../client_secret.json")`.
- **`build.rs` ya NO falla si falta `client_secret.json`**: copia las credenciales (o un placeholder vacío)
  a `OUT_DIR/embedded_secret.json`, que `api.rs` embebe con `include_str!`. Así CI y los clones limpios
  compilan; sin credenciales válidas la app cae a modo simulado (`mock_tasks`). Ver README para generarlas.
- Scope requerido: `https://www.googleapis.com/auth/tasks`.

### Ubicación de datos en runtime (NO en el repo)

Todo se guarda en `~/.config/pomotask/` (vía `dirs::config_dir()` + `App::get_config_dir()`):
- `config.json` — preferencias (duraciones, idioma, tema, vista de calendario, última lista/tarea).
- `stats.json` — estadísticas de pomodoros por hora/tarea y timers persistidos.
- `pomotask_token.json` — token OAuth. `logout()` lo borra.

### Archivos que NUNCA debes commitear

`.gitignore` ya los excluye, pero un agente debe tenerlos presentes:
- `client_secret.json` (secreto OAuth — está en disco para compilar, pero ignorado por git).
- `pomotask_token.json` (token de sesión del usuario).
- `/target`, `Cargo.lock` (ignorado en este repo), `*.json.bak`.

Nunca imprimas, registres ni incluyas el contenido de estos archivos en código, logs o respuestas.

## Comandos de desarrollo

```bash
cargo build              # build de desarrollo (requiere client_secret.json en la raíz)
cargo build --release    # build optimizado
cargo run                # ejecutar la TUI
cargo run -- --version   # imprime la versión (timestamp del último commit git, ver build.rs)
cargo check              # chequeo rápido de tipos
cargo clippy             # linter
cargo fmt                # formateo
./install.sh             # compila en release e instala el binario en ~/.local/bin + .desktop
```

```bash
cargo test               # tests unitarios (lógica pura: parseo de fechas, jerarquía de tareas)
```

> Hay tests unitarios en `src/app/mod.rs` (módulo `#[cfg(test)] mod tests`). La verificación visual
> de la TUI sigue siendo manual. `--version` reporta el timestamp del último commit (`APP_VERSION` lo inyecta `build.rs`).

## Convenciones del proyecto

- **Idioma**: los comentarios, mensajes de UI y de error están en **español**. Mantén ese idioma al editar.
- **i18n**: todo texto visible para el usuario debe pasar por `app.translate("clave")`. Añade la clave
  en ambos idiomas en `src/app/i18n.rs`; no hardcodees strings en la UI.
- **Colores**: no hardcodees colores. Usa el sistema de `Palette` / `ThemeColors` de `ui/palette.rs`.
- **No bloquees el hilo de UI**: cualquier I/O de red va en `tokio::spawn` y devuelve resultados por `Event`.
- **Persistencia**: tras cambiar config llama a `app.save_config()`; tras cambiar stats, `app.save_stats()`.
- **Commits**: el historial usa Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`,
  con scopes opcionales como `feat(ui):`). Sigue ese estilo.
- **Estilo Rust**: el código existente usa funciones densas de una línea en varios sitios; prioriza
  coincidir con el estilo del archivo que edites por encima de reformatear todo.

## Trampas conocidas (gotchas)

- `Cargo.lock` SÍ se versiona (es un binario/aplicación). Ya no está en `.gitignore`.
- La lista `@all` es virtual (se inyecta en `main.rs` al recibir `ListsUpdate`); no existe en la API.
- El descanso largo se activa automáticamente cada 4 pomodoros (`session_pomodoros % 4 == 0`); también
  se puede cambiar de modo manualmente con la tecla `m` (timer detenido).
- No se captura el mouse: se omite a propósito `EnableMouseCapture` para preservar la selección de texto del terminal.
- Sin credenciales válidas, `ApiClient` cae a `mock_tasks()` (modo simulado) en vez de fallar.
- El `Event::Tick` (50 ms) impulsa animaciones (partículas, spinners) y el conteo del temporizador.

## Atajos de teclado

Ver la tabla completa en `README.md`. Resumen: `Espacio` (timer), `m` (cambiar modo del timer),
`Enter` (completar/guardar), `j/k` (navegar tareas), `h/l` (cambiar lista), `[`/`]` (navegar calendario),
`N/A` (nueva tarea/subtarea), `E` (editar), `C` (toggle completadas), `S` (sync), `,` (ajustes),
`?` (ayuda), `Q`/`Esc` (salir).
