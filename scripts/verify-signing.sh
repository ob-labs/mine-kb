#!/bin/bash

# 验证本地签名配置

echo "=================================="
echo "验证签名配置"
echo "=================================="

# 检查证书
echo "1. 检查可用的签名证书："
security find-identity -v -p codesigning

echo ""
echo "2. 检查钥匙串访问权限："
security find-identity -v -p codesigning | grep -i "developer id application"

if [ $? -eq 0 ]; then
    echo "✓ 找到 Developer ID Application 证书"
else
    echo "✗ 未找到 Developer ID Application 证书"
    echo "请在 Xcode -> Preferences -> Accounts 中下载证书"
fi

echo ""
echo "3. 检查环境变量配置："
if [ -f .env.local ]; then
    echo "✓ 找到 .env.local"
    set -a
    source .env.local
    set +a
    echo "  APPLE_TEAM_ID: ${APPLE_TEAM_ID:-未设置}"
    echo "  APPLE_ID: ${APPLE_ID:-未设置}"
    echo "  APPLE_SIGNING_IDENTITY: ${APPLE_SIGNING_IDENTITY:0:50}..."
else
    echo "✗ 未找到 .env.local"
    echo "请复制 .env.example 为 .env.local"
fi

echo ""
echo "=================================="

