#!/usr/bin/env bash

# Script de instalación para el Plugin de PomoTask en Omarchy Quattro
# Compila el binario CLI, instala el plugin QML y lo registra en Omarchy.

set -e

# Colores para la terminal
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

PLUGIN_ID="io.github.pl402.pomotask"
PLUGIN_SRC="$SCRIPT_DIR/plugins/$PLUGIN_ID"
BIN_NAME="pomotask-cli"
INSTALL_BIN_DIR="$HOME/.local/bin"
PLUGINS_DEST_DIR="$HOME/.config/omarchy/plugins"
PLUGIN_TARGET_DIR="$PLUGINS_DEST_DIR/$PLUGIN_ID"

echo -e "${BLUE}==>${NC} Iniciando instalación de ${GREEN}PomoTask Plugin para Omarchy Quattro${NC}..."

# 1. Verificar si Rust/Cargo está instalado
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error:${NC} No se encontró 'cargo'. Por favor, instala Rust desde https://rustup.rs/"
    exit 1
fi

# 2. Verificar origen del plugin
if [ ! -d "$PLUGIN_SRC" ]; then
    echo -e "${RED}Error:${NC} No se encontró el directorio de plugin en ${PLUGIN_SRC}"
    exit 1
fi

# 3. Compilar el proyecto en modo Release
echo -e "${BLUE}==>${NC} Compilando ${GREEN}${BIN_NAME}${NC} en modo release..."
cargo build --release

# 4. Instalar binario en ~/.local/bin
mkdir -p "$INSTALL_BIN_DIR"
echo -e "${BLUE}==>${NC} Instalando binario en ${GREEN}${INSTALL_BIN_DIR}/${BIN_NAME}${NC}"
cp "target/release/${BIN_NAME}" "${INSTALL_BIN_DIR}/"
chmod +x "${INSTALL_BIN_DIR}/${BIN_NAME}"

# 5. Instalar plugin en ~/.config/omarchy/plugins
mkdir -p "$PLUGINS_DEST_DIR"
echo -e "${BLUE}==>${NC} Instalando plugin en ${GREEN}${PLUGIN_TARGET_DIR}${NC}"
rm -rf "$PLUGIN_TARGET_DIR"
cp -r "$PLUGIN_SRC" "$PLUGIN_TARGET_DIR"

# 6. Validar plugin con omarchy si está disponible
if command -v omarchy &> /dev/null; then
    echo -e "${BLUE}==>${NC} Validando plugin con Omarchy..."
    if omarchy plugin validate "$PLUGIN_TARGET_DIR"; then
        echo -e "${GREEN}✓ Manifiesto y estructura de plugin válidos.${NC}"
    else
        echo -e "${YELLOW}Advertencia:${NC} La validación del plugin reportó advertencias."
    fi
else
    echo -e "${YELLOW}Nota:${NC} Comando 'omarchy' no detectado en PATH. Se omitió la validación automática."
fi

# 7. Notificar a omarchy-shell para recargar plugins y habilitar el widget
if command -v omarchy-shell &> /dev/null; then
    echo -e "${BLUE}==>${NC} Notificando a Omarchy Shell para reescanear plugins..."
    omarchy-shell -q shell rescanPlugins || true
fi

if command -v omarchy &> /dev/null; then
    echo -e "${BLUE}==>${NC} Habilitando widget en la barra de Omarchy..."
    omarchy plugin enable "$PLUGIN_ID" --section center || true
fi

# 8. Mensaje final e instrucciones de uso
echo -e "\n${GREEN}====================================================${NC}"
echo -e "${GREEN}¡Plugin de PomoTask instalado exitosamente!${NC}"
echo -e "${GREEN}====================================================${NC}\n"

if [[ ":$PATH:" != *":$INSTALL_BIN_DIR:"* ]]; then
    echo -e "${YELLOW}Aviso:${NC} ${INSTALL_BIN_DIR} no parece estar en tu variable PATH."
    echo -e "Añádelo agregando esto a tu ~/.bashrc o ~/.zshrc:"
    echo -e "  export PATH=\"\$HOME/.local/bin:\$PATH\"\n"
fi

echo -e "Para activar el widget en tu barra de Omarchy, ejecuta:"
echo -e "  ${BLUE}omarchy plugin enable ${PLUGIN_ID} --section center${NC}"
echo -e "\nO para moverlo si ya está activo:"
echo -e "  ${BLUE}omarchy bar move ${PLUGIN_ID} --section center${NC}"
echo -e "\nPuedes probar la interfaz CLI / IPC ejecutando:"
echo -e "  ${GREEN}pomotask-cli ipc status${NC}\n"
