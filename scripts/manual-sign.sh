#!/bin/bash

set -e

echo "=================================="
echo "手动签名 MineKB"
echo "=================================="

# 加载环境变量
if [ -f .env.local ]; then
    echo "✓ 加载 .env.local 配置"
    set -a
    source .env.local
    set +a
else
    echo "✗ 错误：找不到 .env.local 文件"
    exit 1
fi

APP_PATH="src-tauri/target/release/bundle/macos/MineKB.app"
DMG_PATH="src-tauri/target/release/bundle/dmg/MineKB_0.1.0_aarch64.dmg"

if [ ! -d "$APP_PATH" ]; then
    echo "✗ 找不到 APP: $APP_PATH"
    echo "请先运行: npm run tauri build"
    exit 1
fi

echo ""
echo "步骤 1: 移除旧签名（如果存在）..."
codesign --remove-signature "$APP_PATH/Contents/MacOS/MineKB" 2>/dev/null || true
codesign --remove-signature "$APP_PATH" 2>/dev/null || true
echo "✓ 旧签名已清理"

echo ""
echo "步骤 2: 签名应用程序..."

# 提示：可能需要输入钥匙串密码
echo "注意：如果提示输入密码，请输入你的 Mac 登录密码"

# 签名可执行文件（使用证书指纹 + 明确指定钥匙串）
codesign --force --sign "6A07A406A9FF786D8A23F24E4E4CED29D33BACB7" \
    --keychain ~/Library/Keychains/login.keychain-db \
    --options runtime \
    --entitlements src-tauri/entitlements.plist \
    --timestamp \
    "$APP_PATH/Contents/MacOS/MineKB"

if [ $? -eq 0 ]; then
    echo "✓ 可执行文件签名成功"
else
    echo "✗ 可执行文件签名失败"
    exit 1
fi

# 签名整个 app bundle
codesign --force --sign "6A07A406A9FF786D8A23F24E4E4CED29D33BACB7" \
    --keychain ~/Library/Keychains/login.keychain-db \
    --options runtime \
    --entitlements src-tauri/entitlements.plist \
    --timestamp \
    "$APP_PATH"

if [ $? -eq 0 ]; then
    echo "✓ App bundle 签名成功"
else
    echo "✗ App bundle 签名失败"
    exit 1
fi

echo ""
echo "步骤 3: 验证签名..."
codesign --verify --deep --strict --verbose=2 "$APP_PATH"

if [ $? -eq 0 ]; then
    echo "✓ 签名验证通过"
else
    echo "✗ 签名验证失败"
    exit 1
fi

echo ""
echo "步骤 4: 显示签名信息..."
codesign -dvv "$APP_PATH" 2>&1 | grep -E "(Authority|TeamIdentifier|Signature|Timestamp)"

echo ""
echo "步骤 5: 重新创建 DMG..."
# 删除旧的 DMG
rm -f "$DMG_PATH"

# 创建临时目录
TMP_DMG="/tmp/MineKB_$(date +%s)"
mkdir -p "$TMP_DMG"

# 复制 app
cp -R "$APP_PATH" "$TMP_DMG/"

# 创建 DMG
hdiutil create -volname "MineKB" -srcfolder "$TMP_DMG" -ov -format UDZO "$DMG_PATH"

# 清理临时目录
rm -rf "$TMP_DMG"

if [ -f "$DMG_PATH" ]; then
    echo "✓ DMG 创建成功: $DMG_PATH"
else
    echo "✗ DMG 创建失败"
    exit 1
fi

echo ""
echo "步骤 6: 签名 DMG..."
codesign --force --sign "6A07A406A9FF786D8A23F24E4E4CED29D33BACB7" \
    --keychain ~/Library/Keychains/login.keychain-db \
    --timestamp \
    "$DMG_PATH"

if [ $? -eq 0 ]; then
    echo "✓ DMG 签名成功"
else
    echo "✗ DMG 签名失败"
    exit 1
fi

echo ""
echo "步骤 7: 提交公证..."
echo "正在上传到 Apple 服务器（可能需要几分钟）..."

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
    exit 1
fi

echo ""
echo "步骤 8: 装订公证票据..."
xcrun stapler staple "$DMG_PATH"

if [ $? -eq 0 ]; then
    echo "✓ 公证票据装订成功"
else
    echo "✗ 装订失败"
    exit 1
fi

echo ""
echo "=================================="
echo "✓ 签名和公证完成！"
echo "=================================="
echo "输出文件："
echo "  APP: $APP_PATH"
echo "  DMG: $DMG_PATH"
echo ""
echo "验证命令："
echo "  xcrun stapler validate \"$DMG_PATH\""
echo "  spctl -a -vvv -t install \"$APP_PATH\""

