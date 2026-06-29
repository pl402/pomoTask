# CLAUDE.md

Este proyecto sigue el estándar **AGENTS.md**. La guía completa para agentes de IA está en
[`AGENTS.md`](./AGENTS.md) — léela antes de trabajar en el código.

## Resumen rápido

- **PomoTask-CLI**: TUI en Rust (ratatui + tokio) que une la técnica Pomodoro con Google Tasks.
- Punto de entrada: `src/main.rs` (bucle de eventos async). Lógica de teclado: `src/handler.rs`.
- Estado global: `src/app/mod.rs`. Red/Google Tasks: `src/api.rs`. UI: `src/ui/`.

## Lo esencial antes de editar

- **No compila sin `client_secret.json`** en la raíz (`build.rs` lo exige vía `include_str!`).
- **Nunca** commitees ni muestres `client_secret.json` ni `pomotask_token.json`.
- Texto de UI **siempre** vía `app.translate(...)` (i18n ES/EN en `src/app/i18n.rs`).
- Colores vía `ui/palette.rs`, nunca hardcodeados.
- I/O de red en `tokio::spawn`, comunicado a la UI por `Event` (`src/events.rs`).
- Datos de runtime en `~/.config/pomotask/` (config.json, stats.json, token).

## Comandos

```bash
cargo run            # ejecutar la TUI
cargo build --release
cargo test           # tests unitarios (lógica pura)
cargo clippy         # linter (debe quedar en 0 warnings)
cargo fmt            # formateo
```

Hay tests unitarios en `src/app/mod.rs`; la verificación visual de la TUI es manual.
Comentarios y mensajes en **español**; commits en formato Conventional Commits.
