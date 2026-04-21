# 🍅 PomoTask-CLI

**PomoTask-CLI** es una interfaz de terminal (TUI) profesional, asíncrona y visualmente atractiva que combina la técnica **Pomodoro** con la gestión de tareas de **Google Tasks**. Diseñada con una estética moderna basada en la paleta de colores **Catppuccin Mocha**.

![Estado del Proyecto](https://img.shields.io/badge/Status-Functional-success?style=for-the-badge)
![Lenguaje](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![Interfaz](https://img.shields.io/badge/Ratatui-flat?style=for-the-badge&color=f38ba8)

## ✨ Características Principales

- **☁️ Sincronización Real con Google Tasks**: Visualiza, crea, edita y completa tareas directamente desde tu terminal.
- **🌳 Jerarquía de Subtareas**: Organización real de tareas y subtareas con indentación visual (`↳`).
- **⏱️ Pomodoro Persistente por Tarea**: Cada tarea recuerda su propio cronómetro. Si cambias de tarea o cierras la app, el tiempo se guarda.
- **📊 Historial de Productividad**: Gráfica Sparkline que muestra tus pomodoros completados en las últimas 24 horas.
- **🔔 Notificaciones de Escritorio**: Avisos nativos al terminar sesiones de enfoque o descansos.
- **🌑 Estética Catppuccin**: Interfaz elegante con bordes redondeados y colores TrueColor.
- **🌍 Multilenguaje**: Soporte completo para Español e Inglés (cambio en tiempo real con `L`).
- **🔐 Seguridad OAuth2**: Autenticación segura integrada en la TUI con apertura automática de navegador.

## 🛠️ Stack Tecnológico

- **Lenguaje**: Rust 🦀
- **UI Framework**: [Ratatui](https://ratatui.rs/) con backend `crossterm`.
- **Runtime Asíncrono**: `tokio`.
- **API**: `google-tasks1` para la integración con Google.
- **Persistencia**: JSON local en `~/.config/pomotask/`.

## 🚀 Instalación y Configuración

### 1. Requisitos Previos
- Tener instalado [Rust y Cargo](https://rustup.rs/).
- Un proyecto en [Google Cloud Console](https://console.cloud.google.com/):
    - Habilita la **Google Tasks API**.
    - Configura la **OAuth Consent Screen** (añade tu email como tester).
    - Crea credenciales de tipo **Desktop App**.
    - Descarga el JSON de credenciales y renómbralo como `client_secret.json`.

### 2. Preparación
Clona el repositorio y coloca tu archivo de credenciales en la raíz:
```bash
git clone https://github.com/tu-usuario/pomoTask.git
cd pomoTask
cp ~/descargas/tu_secreto_google.json ./client_secret.json
```

### 3. Ejecución
```bash
cargo run
```

## ⌨️ Atajos de Teclado (Hotkeys)

| Tecla | Acción |
| :--- | :--- |
| `Espacio` | Iniciar / Pausar Pomodoro (Modo Enfoque) |
| `Enter` | Confirmar (Hecho/Pendiente) o Guardar en formularios |
| `N` | Crear nueva tarea principal |
| `A` | Añadir subtarea a la tarea seleccionada |
| `E` | Editar detalles de la tarea (Título, Notas, Vencimiento) |
| `C` | Alternar mostrar/ocultar tareas ya completadas |
| `Tab` | Abrir selector de listas de Google Tasks |
| `L` | Cambiar idioma (Español / Inglés) |
| `S` | Sincronización manual con la nube |
| `Q` | Salir de la aplicación de forma segura |

## 📂 Estructura de Archivos (Configuración)

La aplicación almacena sus datos de forma persistente en Linux en `~/.config/pomotask/`:
- `config.json`: Preferencias de idioma, última lista y tarea seleccionada.
- `stats.json`: Contadores de pomodoros por hora y por tarea.
- `pomotask_token.json`: Token de sesión cifrado de Google.

---
Desarrollado con ❤️ por un desarrollador Senior en Rust. 🍅🚀
