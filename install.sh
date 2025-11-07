#!/bin/bash

# MineKB 安装脚本
# 适用于 macOS

set -e

APP_NAME="MineKB"
APP_DIR="/Applications"
CONFIG_DIR="$HOME/Library/Application Support/com.mine-kb.app"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  MineKB 安装脚本"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 检查是否存在 .app 文件
if [ ! -d "${APP_NAME}.app" ]; then
    echo "❌ 错误: 找不到 ${APP_NAME}.app"
    echo "请确保安装脚本与 ${APP_NAME}.app 在同一目录下"
    exit 1
fi

# 1. 复制应用到 Applications
echo "📦 步骤 1/4: 安装应用到 Applications 文件夹..."
if [ -d "${APP_DIR}/${APP_NAME}.app" ]; then
    echo "   发现已存在的应用，正在移除旧版本..."
    rm -rf "${APP_DIR}/${APP_NAME}.app"
fi
cp -R "${APP_NAME}.app" "${APP_DIR}/"
echo "   ✅ 应用已安装到: ${APP_DIR}/${APP_NAME}.app"

# 2. 移除隔离属性
echo ""
echo "🔓 步骤 2/4: 移除 Gatekeeper 隔离属性..."
xattr -cr "${APP_DIR}/${APP_NAME}.app"
echo "   ✅ 已移除隔离属性"

# 3. 创建配置目录
echo ""
echo "📁 步骤 3/4: 创建配置目录..."
mkdir -p "${CONFIG_DIR}"
echo "   ✅ 配置目录已创建: ${CONFIG_DIR}"

# 4. 处理配置文件
echo ""
echo "⚙️  步骤 4/4: 配置文件设置..."
CONFIG_FILE="${CONFIG_DIR}/config.json"
EXAMPLE_CONFIG_FILE="${CONFIG_DIR}/config.example.json"

if [ -f "${CONFIG_FILE}" ]; then
    echo "   ✅ 配置文件已存在，跳过配置步骤"
else
    # 从应用包中复制示例配置
    APP_RESOURCES="${APP_DIR}/${APP_NAME}.app/Contents/Resources"
    if [ -f "${APP_RESOURCES}/config.example.json" ]; then
        cp "${APP_RESOURCES}/config.example.json" "${EXAMPLE_CONFIG_FILE}"
        echo "   ✅ 已创建示例配置文件"
    fi

    echo ""
    echo "   ⚠️  配置文件不存在，需要手动配置"
    echo ""
    echo "   请按照以下步骤操作："
    echo "   1. 编辑配置文件:"
    echo "      ${EXAMPLE_CONFIG_FILE}"
    echo "   2. 填写您的 API 密钥和其他配置信息"
    echo "   3. 将文件重命名为: config.json"
    echo "   4. 启动应用"
    echo ""

    # 询问是否现在打开配置目录
    read -p "   是否现在打开配置目录? (y/n) " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        open "${CONFIG_DIR}"
    fi
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ✅ 安装完成！"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🚀 启动方式:"
echo "   - 从 Launchpad 启动"
echo "   - 从 Applications 文件夹启动"
echo "   - 或使用 Spotlight 搜索 '${APP_NAME}'"
echo ""

if [ ! -f "${CONFIG_FILE}" ]; then
    echo "⚠️  首次运行前请确保已配置 config.json"
    echo "   配置目录: ${CONFIG_DIR}"
fi

echo ""
