# 🍅 PomoTask-CLI

**PomoTask-CLI** es una interfaz de terminal (TUI) profesional, asíncrona y visualmente atractiva que combina la técnica **Pomodoro** con la gestión de tareas de **Google Tasks**. Diseñada con una estética moderna y altamente personalizable.

![Estado del Proyecto](https://img.shields.io/badge/Status-Functional-success?style=for-the-badge)
![Lenguaje](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![Interfaz](https://img.shields.io/badge/Ratatui-flat?style=for-the-badge&color=f38ba8)

## ✨ Características Principales

- **☁️ Sincronización Real con Google Tasks**: Gestión bidireccional de tareas y subtareas en tiempo real.
- **🌐 Vista Consolidada "Todas"**: Lista virtual que reúne las tareas de todas tus listas reales para una visión global.
- **🎨 Temas visuales**: Soporte nativo para esquemas de colores **Catppuccin**, **Nord**, **Gruvbox** y **Dracula**.
- **🍅 Modo concentración inmersivo**: Pantalla completa con un **reloj digital gigante** y fecha localizada para un seguimiento claro del tiempo.
- **📑 Gestión de subtareas**: Visualiza y marca subtareas como completadas directamente desde el modo enfoque.
- **✨ Animación de Victoria**: Efecto de partículas localizado y tachado real al completar tareas.
- **📊 Gráfico de Productividad 2.0**: BarChart que registra minutos enfocados y tareas completadas por hora.
- **⏱️ Pomodoro Inteligente**: Temporizador que se guarda por tarea y se detiene automáticamente al completar la tarea principal.
- **📅 Fechas Flexibles**: Selección rápida de fecha con presets ("Hoy", "Mañana", "Sin fecha") y formato localizado.
- **🔔 Notificaciones**: Avisos nativos de sistema al finalizar cada sesión de trabajo o descanso.

## 🛠️ Stack Tecnológico

- **Lenguaje**: Rust 🦀 (Edición 2021)
- **TUI**: [Ratatui](https://ratatui.rs/) + `crossterm`.
- **Async**: `tokio` para operaciones de red no bloqueantes y concurrentes.
- **API**: Integración directa con Google Cloud Console via `google-tasks1`.
- **Estilo**: Paletas de colores dinámicas y soporte para ajuste de línea (`wrap`) en textos largos.

## 🚀 Instalación y Compilación

### 1. Requisitos Previos
- Tener instalado [Rust y Cargo](https://rustup.rs/).
- Un archivo `client_secret.json` de Google Cloud Console (Desktop App).

### 2. Compilar e Instalar
```bash
# Clonar el repositorio
git clone https://github.com/pl402/pomoTask.git
cd pomoTask

# Compilar e Instalar en tu PATH local
cargo install --path .
```

### 3. Configuración Inicial
La primera vez que la ejecutes:
1. Se abrirá tu navegador para autorizar la conexión con Google Tasks.
2. ¡Listo! Tus preferencias se guardarán en `~/.config/pomotask/`.

## ⌨️ Atajos de Teclado (Hotkeys)

| Tecla | Acción |
| :--- | :--- |
| `Espacio` | Iniciar / Pausar Temporizador |
| `Enter` | Completar Tarea / Guardar (con confirmación de seguridad) |
| `j` / `k` | Navegar entre tareas y subtareas (incluso en modo enfoque) |
| `h` / `l` | Cambiar rápidamente entre listas de tareas (← / →) |
| `Tab` | Abrir selector de listas o saltar campos en formularios |
| `,` (Coma) | **Abrir Menú de Configuración** |
| `N` / `A` | Nueva Tarea / Nueva Subtarea (con selector de lista destino) |
| `E` | Editar Tarea seleccionada |
| `C` | Mostrar/Ocultar tareas completadas |
| `S` | Sincronización manual forzada |
| `?` | Ver Ayuda de Teclado |
| `Q` / `Esc` | Salir / Cerrar Modales |

## 📂 Estructura de Datos

La aplicación utiliza JSON para la persistencia en `~/.config/pomotask/`:
- `config.json`: Ajustes de usuario, temas e idioma.
- `stats.json`: Datos históricos de enfoque por hora.
- `pomotask_token.json`: Token de autenticación seguro de Google.

---
Desarrollado con ❤️ desde **México** 🇲🇽 por un humano que sobrevive a base de agua, té y papitas con mucho chile. 🍵🔥🌶️
