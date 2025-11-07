#!/bin/bash

GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'
BOLD='\033[1m'

CERT_HASH="6A07A406A9FF786D8A23F24E4E4CED29D33BACB7"
CERT_NAME="Developer ID Application: Beijing OceanBase Technology Co., Ltd. (QWQ3HBA8MF)"

echo ""
echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BOLD}    测试代码签名${NC}"
echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# 创建测试文件
TEST_FILE="/tmp/test-sign-$(date +%s).txt"
echo "test content" > "$TEST_FILE"

echo -e "${BLUE}[测试 1]${NC} 简单文件签名..."
#if codesign --sign "$CERT_HASH" --force "$TEST_FILE" 2>&1 | tee /tmp/sign-output.txt; then
if codesign --sign "$CERT_NAME" --keychain ~/Library/Keychains/login.keychain-db --force "$TEST_FILE" 2>&1 | tee /tmp/sign-output.txt; then
    echo -e "      ${GREEN}✓${NC} 签名成功"

    echo ""
    echo -e "${BLUE}[测试 2]${NC} 验证签名..."
    if codesign -vvv "$TEST_FILE" 2>&1; then
        echo -e "      ${GREEN}✓${NC} 验证通过"

        echo ""
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${GREEN}${BOLD}    ✓✓✓ 代码签名正常工作！${NC}"
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo ""
        echo -e "${BOLD}可以开始签名应用：${NC}"
        echo -e "  ${BLUE}bash scripts/manual-sign.sh${NC}"
        echo ""
        rm -f "$TEST_FILE"
        exit 0
    else
        echo -e "      ${RED}✗${NC} 验证失败"
    fi
else
    echo -e "      ${RED}✗${NC} 签名失败"
    echo ""
    echo -e "${YELLOW}错误详情：${NC}"
    cat /tmp/sign-output.txt | sed 's/^/  /'
    echo ""
fi

rm -f "$TEST_FILE"

echo ""
echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${RED}${BOLD}    签名仍然失败${NC}"
echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}可能的解决方案：${NC}"
echo ""
echo -e "${BLUE}方案 1:${NC} 检查证书访问权限"
echo "  1. 打开「钥匙串访问」应用"
echo "  2. 找到证书：Developer ID Application: Beijing OceanBase"
echo "  3. 右键 → 显示简介 → 访问控制"
echo "  4. 选择「允许所有应用程序访问此项目」"
echo ""
echo -e "${BLUE}方案 2:${NC} 重新导入开发者证书"
echo "  可能需要从 Apple Developer 重新下载证书"
echo ""
echo -e "${BLUE}方案 3:${NC} 使用应用专用密码"
echo "  在代码签名时可能需要解锁钥匙串："
echo "  security unlock-keychain ~/Library/Keychains/login.keychain-db"
echo ""
exit 1

