# 🍅 PomoTask-CLI

**PomoTask-CLI** es una interfaz de terminal (TUI) profesional, asíncrona y visualmente atractiva que combina la técnica **Pomodoro** con la gestión de tareas de **Google Tasks**. Diseñada con una estética moderna y altamente personalizable.

![Estado del Proyecto](https://img.shields.io/badge/Status-Functional-success?style=for-the-badge)
![Lenguaje](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![Interfaz](https://img.shields.io/badge/Ratatui-flat?style=for-the-badge&color=f38ba8)

## ✨ Características Principales

- **☁️ Sincronización Real con Google Tasks**: Gestión bidireccional de tareas y subtareas en tiempo real.
- **🎨 Temas Visuales**: Soporte nativo para esquemas de colores **Catppuccin**, **Nord**, **Gruvbox** y **Dracula**.
- **⚙️ Menú de Configuración**: Ajusta tiempos de pomodoro, idioma y temas directamente desde la app.
- **✨ Animación de Victoria**: Efecto de partículas localizado y tachado real al completar tareas.
- **📊 Gráfico de Productividad 2.0**: BarChart que registra minutos enfocados y tareas completadas por hora.
- **⏱️ Pomodoro Persistente**: Temporizador inteligente que se guarda por tarea y sobrevive a reinicios.
- **📅 Selección de Fecha Rápida**: Presets para "Hoy", "Mañana" y fechas personalizadas con formato localizado (ej: "23 Abril").
- **🔔 Notificaciones**: Avisos nativos de sistema al finalizar cada sesión.

## 🛠️ Stack Tecnológico

- **Lenguaje**: Rust 🦀 (Edición 2021)
- **TUI**: [Ratatui](https://ratatui.rs/) + `crossterm`.
- **Async**: `tokio` para operaciones de red no bloqueantes.
- **API**: Integración directa con Google Cloud Console via `google-tasks1`.
- **Estilo**: Paletas de colores personalizadas basadas en temas populares de la comunidad.

## 🚀 Instalación y Compilación

### 1. Requisitos Previos
- Tener instalado [Rust y Cargo](https://rustup.rs/).
- Un archivo `client_secret.json` de Google Cloud Console (Desktop App).

### 2. Compilar e Instalar
Para compilar y dejar la aplicación disponible globalmente en tu sistema:

```bash
# Clonar el repositorio
git clone https://github.com/pl402/pomoTask.git
cd pomoTask

# Compilar en modo Release para máximo rendimiento
cargo build --release

# Instalar en tu PATH local (generalmente ~/.cargo/bin/)
cargo install --path .
```

Una vez instalado, simplemente puedes ejecutar `pomotask-cli` desde cualquier terminal.

### 3. Configuración Inicial
La primera vez que la ejecutes:
1. Se abrirá tu navegador para autorizar la conexión con Google Tasks.
2. Selecciona ambos permisos (lectura y gestión) para que la app funcione correctamente.
3. ¡Listo! Tus preferencias se guardarán en `~/.config/pomotask/`.

## ⌨️ Atajos de Teclado (Hotkeys)

| Tecla | Acción |
| :--- | :--- |
| `Espacio` | Iniciar / Pausar Temporizador |
| `Enter` | Completar Tarea / Guardar Formulario |
| `,` (Coma) | **Abrir Menú de Configuración** |
| `?` | Ver Ayuda de Teclado |
| `N` / `A` | Nueva Tarea / Nueva Subtarea |
| `E` | Editar Tarea (incluye presets de fecha) |
| `C` | Mostrar/Ocultar tareas completadas |
| `Tab` | Cambiar entre listas de tareas |
| `L` | Cambiar idioma rápidamente |
| `S` | Sincronización forzada |
| `Q` / `Esc` | Salir / Cerrar Modales |

## 📂 Estructura de Datos

La aplicación utiliza JSON para la persistencia en `~/.config/pomotask/`:
- `config.json`: Ajustes de usuario, temas y estado de la interfaz.
- `stats.json`: Datos históricos de enfoque (minutos y tareas) por hora.
- `pomotask_token.json`: Token de autenticación seguro de Google.

---
Desarrollado con ❤️ por un humano que sobrevive a base de agua, té y papitas con mucho chile. 🍵🔥🌶️
