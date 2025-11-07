#!/bin/bash

# Tauri Linux 依赖安装脚本
# 适用于 Ubuntu/Debian 系统

echo "🔧 开始安装 Tauri 开发所需的系统依赖..."

# 更新软件包列表
echo "📦 更新软件包列表..."
sudo apt-get update

# 安装基础编译工具
echo "🛠️  安装基础编译工具..."
sudo apt-get install -y \
    build-essential \
    curl \
    wget \
    file

# 安装 pkg-config
echo "📋 安装 pkg-config..."
sudo apt-get install -y pkg-config

# 安装 GTK3 和相关开发库
echo "🎨 安装 GTK3 开发库..."
sudo apt-get install -y \
    libgtk-3-dev \
    libwebkit2gtk-4.0-dev \
    libssl-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

# 安装其他可能需要的库
echo "📚 安装其他依赖库..."
sudo apt-get install -y \
    libglib2.0-dev \
    libgdk-pixbuf2.0-dev \
    libcairo2-dev \
    libpango1.0-dev \
    libatk1.0-dev \
    libsoup2.4-dev

echo "✅ 所有依赖安装完成！"
echo ""
echo "现在你可以运行以下命令来启动开发服务器："
echo "  npm run tauri:dev"
echo ""
echo "或者构建生产版本："
echo "  npm run tauri:build"

