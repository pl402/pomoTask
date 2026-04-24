#!/bin/bash

# Script de instalación para PomoTask-CLI
# Detecta el sistema y coloca el binario en la ruta adecuada.

set -e

# Colores para la terminal
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}==>${NC} Iniciando instalación de ${GREEN}PomoTask-CLI${NC}..."

# 1. Verificar si Rust/Cargo está instalado
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Error:${NC} No se encontró 'cargo'. Por favor, instala Rust desde https://rustup.rs/"
    exit 1
fi

# 2. Compilar el proyecto en modo Release
echo -e "${BLUE}==>${NC} Compilando binario optimizado (esto puede tardar un poco)..."
cargo build --release

# 3. Determinar la ruta de instalación según el SO
OS="$(uname -s)"
case "${OS}" in
    Linux*|Darwin*)
        INSTALL_DIR="$HOME/.local/bin"
        ;;
    CYGWIN*|MINGW32*|MSYS*|MINGW*)
        INSTALL_DIR="/usr/bin" # Entorno Unix en Windows
        ;;
    *)
        echo -e "${YELLOW}SO no reconocido:${NC} ${OS}. Intentando en ~/.local/bin..."
        INSTALL_DIR="$HOME/.local/bin"
        ;;
esac

# 4. Crear el directorio si no existe
if [ ! -d "$INSTALL_DIR" ]; then
    echo -e "${BLUE}==>${NC} Creando directorio de instalación: ${INSTALL_DIR}"
    mkdir -p "$INSTALL_DIR"
fi

# 5. Copiar el binario
BINARY_NAME="pomotask-cli"
echo -e "${BLUE}==>${NC} Instalando binario en ${GREEN}${INSTALL_DIR}/${BINARY_NAME}${NC}"
cp "target/release/${BINARY_NAME}" "${INSTALL_DIR}/"

# 6. Finalización
echo -e "\n${GREEN}¡Instalación completada con éxito!${NC}"
echo -e "Asegúrate de que ${BLUE}${INSTALL_DIR}${NC} esté en tu variable ${YELLOW}PATH${NC}."
echo -e "Ahora puedes ejecutar la aplicación simplemente escribiendo: ${GREEN}${BINARY_NAME}${NC}\n"

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}Aviso:${NC} El directorio ${INSTALL_DIR} no parece estar en tu PATH."
    echo -e "Puedes añadirlo agregando esto a tu .bashrc o .zshrc:"
    echo -e "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi
