#!/bin/bash
# Install Python dependencies for SeekDB bridge
# Note: The application will automatically create a virtual environment and install dependencies.
# This script is provided for manual installation if needed.

set -e

echo "🐍 Installing Python dependencies for SeekDB..."
echo

# Check if Python 3 is installed
if ! command -v python3 &> /dev/null; then
    echo "❌ Error: Python 3 is not installed"
    echo "Please install Python 3 first:"
    echo "  Ubuntu/Debian: sudo apt install python3 python3-venv python3-pip"
    echo "  macOS: brew install python3"
    echo "  Windows: Download from python.org"
    exit 1
fi

echo "✓ Python 3 found: $(python3 --version)"
echo

# Determine the application data directory
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    APP_DATA_DIR="$HOME/Library/Application Support/com.mine-kb.app"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    # Windows
    APP_DATA_DIR="$APPDATA/com.mine-kb.app"
else
    # Linux
    APP_DATA_DIR="$HOME/.local/share/com.mine-kb.app"
fi

VENV_DIR="$APP_DATA_DIR/venv"

echo "📁 Virtual environment directory: $VENV_DIR"
echo

# Create virtual environment if it doesn't exist
if [ ! -d "$VENV_DIR" ]; then
    echo "🔧 Creating Python virtual environment..."
    python3 -m venv "$VENV_DIR"
    echo "✅ Virtual environment created"
else
    echo "✅ Virtual environment already exists"
fi

# Activate virtual environment
echo "🔌 Activating virtual environment..."
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    source "$VENV_DIR/Scripts/activate"
else
    source "$VENV_DIR/bin/activate"
fi

# Upgrade pip
echo "⬆️  Upgrading pip..."
pip install --upgrade pip -i https://pypi.tuna.tsinghua.edu.cn/simple/

# Install seekdb
echo "📦 Installing seekdb package..."
pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple/

echo
echo "✅ All dependencies installed successfully!"
echo
echo "To verify installation, run:"
echo "  $VENV_DIR/bin/python3 -c 'import seekdb; print(\"SeekDB OK\")'"
echo
echo "Note: The application will use this virtual environment automatically."

