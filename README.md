# 🍅 PomoTask-CLI

**PomoTask-CLI** es una interfaz de terminal (TUI) profesional, asíncrona y visualmente atractiva que combina la técnica **Pomodoro** con la gestión de tareas de **Google Tasks**. Diseñada con una estética moderna, modular y altamente personalizable.

![Estado del Proyecto](https://img.shields.io/badge/Status-Functional-success?style=for-the-badge)
![Lenguaje](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![Interfaz](https://img.shields.io/badge/Ratatui-flat?style=for-the-badge&color=f38ba8)

## 📸 Capturas de Pantalla

| Vista Principal (Calendario) | Modo Enfoque (Reloj Gigante) |
| :---: | :---: |
| ![Main View](assets/main_view.png) | ![Focus Mode](assets/focus_mode.png) |

## 🧠 ¿Qué es la Técnica Pomodoro?

La **Técnica Pomodoro** es un método de gestión del tiempo desarrollado por Francesco Cirillo a fines de la década de 1980. Se basa en el uso de un temporizador para dividir el trabajo en intervalos (llamados "pomodoros"), tradicionalmente de **25 minutos**, separados por breves descansos.

Este método se fundamenta en la idea de que las pausas frecuentes pueden mejorar la agilidad mental y la concentración. PomoTask-CLI facilita este flujo integrando tus tareas reales de Google directamente en el temporizador.

> [Más información en Wikipedia](https://es.wikipedia.org/wiki/T%C3%A9cnica_Pomodoro)

## ✨ Características Principales

- **☁️ Sincronización Real con Google Tasks**: Gestión bidireccional de tareas y subtareas en tiempo real.
- **🔗 Conexión Automática**: Servidor local temporal para capturar credenciales OAuth sin copiar/pegar códigos.
- **📅 Vistas de Calendario Personalizables**:
    - **Semáforo**: Indicadores clásicos de Pendiente (🔴), Hecho (🟢) y Actividad (🔵).
    - **Mapa de Calor**: Intensidad de color basada en tareas completadas por día (estilo GitHub).
    - **Progreso Diario**: Visualización de porcentaje de cumplimiento mediante iconos dinámicos.
- **⏱️ Análisis Horario**: Vistas por **Mes**, **Semana** (cuadrícula horaria) y **Día** (detalle por hora) con soporte de zona horaria local.
- **🎨 Personalización Estética Avanzada**:
    - **Temas Expandidos**: Soporte nativo para Catppuccin, Nord, Gruvbox, Dracula, Monokai, Solarized Dark y Ocean.
    - **Temas Custom**: Posibilidad de definir paletas RGB propias en `config.json`.
- **🚀 Interfaz Ultrarrápida (Optimistic UI)**:
    - **Feedback Inmediato**: Inserción instantánea de tareas y spinners animados mientras se sincroniza con la nube.
    - **Animación de Victoria**: Efecto de partículas y tachado visual tras confirmación de la API.
- **🌐 Multilingüe**: Soporte completo para Español e Inglés con mensajes motivacionales dinámicos en listas vacías.
- **🍅 Modo Concentración Inmersivo**: Pantalla completa con reloj digital gigante y gestión de subtareas.
- **🔔 Notificaciones**: Avisos nativos del sistema al finalizar sesiones.

## 🛠️ Stack Tecnológico y Arquitectura

- **Rust 🦀 (Edición 2021)**: Código optimizado para alto rendimiento y seguridad de memoria.
- **Arquitectura Modular**:
    - `src/ui/palette.rs`: Lógica de temas y colores totalmente desacoplada.
    - `src/app/i18n.rs`: Sistema de internacionalización centralizado.
    - `src/api.rs`: Comunicación asíncrona robusta con Google Cloud.
- **Ratatui + Tokio**: Interfaz de terminal reactiva con procesamiento de red no bloqueante.

## 🚀 Instalación y Compilación

### 1. Configuración de la API de Google (client_secret.json)
Para que PomoTask-CLI pueda sincronizarse con tus tareas, necesitas tus propias credenciales de Google Cloud:

1. Ve a [Google Cloud Console](https://console.cloud.google.com/).
2. Crea un nuevo proyecto (ej. "PomoTask").
3. En el buscador superior, busca **"Google Tasks API"** y haz clic en **Habilitar**.
4. Ve a **"Pantalla de consentimiento de OAuth"**:
    - Selecciona tipo de usuario **Externo**.
    - Rellena los datos obligatorios (nombre de app, email).
    - En **Permisos (Scopes)**, añade: `https://www.googleapis.com/auth/tasks`.
    - En **Usuarios de prueba**, añade tu propio correo electrónico de Google.
5. Ve a **"Credenciales"**:
    - Haz clic en **Crear credenciales** -> **ID de cliente de OAuth**.
    - Tipo de aplicación: **App de escritorio**.
    - Nombre: PomoTask-CLI.
6. Una vez creado, descarga el archivo JSON.
7. Renombra el archivo descargado a `client_secret.json` y colócalo en la raíz del proyecto `pomoTask/`.

### 2. Requisitos Previos
- Tener instalado [Rust y Cargo](https://rustup.rs/).
- El archivo `client_secret.json` configurado en el paso anterior.

### 3. Instalación Rápida (Linux/macOS)
```bash
git clone https://github.com/pl402/pomoTask.git
cd pomoTask
./install.sh
```

El script compilará el proyecto en modo `release` e instalará el binario en `~/.local/bin`.

## ⌨️ Atajos de Teclado (Hotkeys)

| Tecla | Acción |
| :--- | :--- |
| `Espacio` | Iniciar / Pausar Temporizador |
| `Enter` | Completar Tarea / Guardar |
| `j` / `k` | Navegar tareas (el calendario sigue tu selección) |
| `h` / `l` | Cambiar entre listas de tareas (← / →) |
| `[` / `]` | **Navegar Calendario** (mes anterior / siguiente) |
| `Tab` | Abrir selector de listas o saltar campos |
| `,` (Coma) | Configuración (Tiempos, Idioma, Temas) |
| `N` / `A` | Nueva Tarea / Nueva Subtarea |
| `E` | Editar Tarea (disponible en todas las vistas) |
| `C` | Mostrar/Ocultar completadas en la lista |
| `S` | Sincronización manual |
| `?` | Ver Ayuda |
| `Q` / `Esc` | Salir |
| `--version` / `-v` | Ver versión instalada (CLI) |

## 📂 Estructura del Código

El proyecto ha sido refactorizado para ser altamente mantenible:
- `src/ui/`: Componentes modulares de la interfaz (Calendario, Listas, Modales, Temporizador).
- `src/handler.rs`: Lógica centralizada de eventos de teclado.
- `src/app.rs`: Estado de la aplicación y lógica de negocio.
- `src/api.rs`: Comunicación con la API de Google.

---
Desarrollado con ❤️ desde **México** 🇲🇽 por un humano que sobrevive a base de agua, té y papitas con mucho chile. 🍵🔥🌶️
