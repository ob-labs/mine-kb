#!/bin/bash

set -e  # 遇到错误立即退出

# 脚本说明
echo "=================================="
echo "MineKB 签名和公证自动化脚本"
echo "=================================="

# 加载环境变量
if [ -f .env.local ]; then
    echo "✓ 加载 .env.local 配置"
    set -a
    source .env.local
    set +a
else
    echo "✗ 错误：找不到 .env.local 文件"
    echo "请复制 .env.example 为 .env.local 并填写配置"
    exit 1
fi

# 验证必需的环境变量
required_vars=("APPLE_SIGNING_IDENTITY" "APPLE_TEAM_ID" "APPLE_ID" "APPLE_APP_PASSWORD" "APP_NAME" "APP_VERSION")
for var in "${required_vars[@]}"; do
    if [ -z "${!var}" ]; then
        echo "✗ 错误：环境变量 $var 未设置"
        exit 1
    fi
done

echo "✓ 环境变量验证通过"
echo ""

# 定义路径
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/src-tauri/target/release/bundle"
APP_PATH="$BUILD_DIR/macos/${APP_NAME}.app"
DMG_PATH="$BUILD_DIR/dmg/${APP_NAME}_${APP_VERSION}_aarch64.dmg"

# 步骤 1: 清理旧构建
echo "步骤 1/6: 清理旧构建..."
if [ -d "$BUILD_DIR" ]; then
    # 清理扩展属性（防止 "failed to remove extra attributes" 错误）
    echo "  清理扩展属性..."
    find "$BUILD_DIR" -exec xattr -c {} \; 2>/dev/null || true
    # 删除旧构建
    rm -rf "$BUILD_DIR"
    echo "✓ 已清理旧构建"
fi
echo ""

# 步骤 2: 构建应用
echo "步骤 2/6: 构建应用..."
cd "$PROJECT_ROOT"
npm run tauri build
if [ ! -d "$APP_PATH" ]; then
    echo "✗ 构建失败：找不到 $APP_PATH"
    exit 1
fi
echo "✓ 构建成功"
echo ""

# 步骤 3: 验证签名
echo "步骤 3/6: 验证应用签名..."
codesign -dvv "$APP_PATH" 2>&1 | grep -q "$APPLE_TEAM_ID"
if [ $? -eq 0 ]; then
    echo "✓ 应用已正确签名"
    echo "签名信息："
    codesign -dvv "$APP_PATH" 2>&1 | grep -E "(Authority|TeamIdentifier|Signature)"
else
    echo "✗ 签名验证失败"
    exit 1
fi
echo ""

# 步骤 4: 验证 DMG 存在
echo "步骤 4/6: 检查 DMG 文件..."
if [ ! -f "$DMG_PATH" ]; then
    echo "✗ 找不到 DMG 文件：$DMG_PATH"
    exit 1
fi
echo "✓ DMG 文件已生成：$DMG_PATH"
echo ""

# 步骤 5: 提交公证
echo "步骤 5/6: 提交公证（这可能需要几分钟）..."
echo "正在上传到 Apple 服务器..."

NOTARIZE_OUTPUT=$(xcrun notarytool submit "$DMG_PATH" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_APP_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait 2>&1)

echo "$NOTARIZE_OUTPUT"

if echo "$NOTARIZE_OUTPUT" | grep -q "status: Accepted"; then
    echo "✓ 公证成功"
else
    echo "✗ 公证失败"
    echo "查看详细日志："
    # 提取 submission ID
    SUBMISSION_ID=$(echo "$NOTARIZE_OUTPUT" | grep "id:" | head -1 | awk '{print $2}')
    if [ ! -z "$SUBMISSION_ID" ]; then
        xcrun notarytool log "$SUBMISSION_ID" \
            --apple-id "$APPLE_ID" \
            --password "$APPLE_APP_PASSWORD" \
            --team-id "$APPLE_TEAM_ID"
    fi
    exit 1
fi
echo ""

# 步骤 6: 装订公证票据
echo "步骤 6/6: 装订公证票据..."
xcrun stapler staple "$DMG_PATH"
if [ $? -eq 0 ]; then
    echo "✓ 公证票据装订成功"
else
    echo "✗ 装订失败"
    exit 1
fi
echo ""

# 验证最终结果
echo "=================================="
echo "最终验证"
echo "=================================="

echo "DMG 公证验证："
xcrun stapler validate "$DMG_PATH"

echo ""
echo "应用 Gatekeeper 验证："
spctl -a -vvv -t install "$APP_PATH"

echo ""
echo "=================================="
echo "✓ 签名和公证完成！"
echo "=================================="
echo "输出文件："
echo "  APP: $APP_PATH"
echo "  DMG: $DMG_PATH"
echo ""
echo "现在可以将 DMG 文件分发给用户了！"

