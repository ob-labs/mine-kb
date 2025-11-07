#!/bin/bash

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'
BOLD='\033[1m'

echo ""
echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BOLD}    安装 G2 中间证书${NC}"
echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# 检查当前状态
echo -e "${BLUE}[检查]${NC} 检查现有证书..."
EXISTING_G2=$(security find-certificate -a -c "Developer ID Certification Authority" 2>/dev/null | grep "OU=G2" | wc -l | tr -d ' ')

if [ "${EXISTING_G2:-0}" -gt 0 ]; then
    echo -e "      ${GREEN}✓${NC} 已经安装了 G2 证书"
    echo ""
    echo "你可以运行 'bash scripts/verify-g2.sh' 来验证"
    exit 0
fi

echo -e "      ${YELLOW}⚠${NC}  未找到 G2 证书，准备安装..."
echo ""

# 下载证书
echo -e "${BLUE}[步骤 1/3]${NC} 下载 G2 中间证书..."
cd /tmp

if [ -f "DeveloperIDG2CA.cer" ]; then
    echo -e "      ${BLUE}→${NC} 证书文件已存在，跳过下载"
else
    echo -e "      ${BLUE}→${NC} 从 Apple 官网下载..."
    if curl -f -O https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer 2>/dev/null; then
        echo -e "      ${GREEN}✓${NC} 下载成功"
    else
        echo -e "      ${RED}✗${NC} 下载失败"
        echo ""
        echo "请手动下载："
        echo "  https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer"
        exit 1
    fi
fi

# 验证文件
echo ""
echo -e "${BLUE}[步骤 2/3]${NC} 验证证书文件..."
if [ -f "DeveloperIDG2CA.cer" ]; then
    FILE_SIZE=$(stat -f%z DeveloperIDG2CA.cer 2>/dev/null || echo "0")
    if [ "$FILE_SIZE" -gt 100 ]; then
        echo -e "      ${GREEN}✓${NC} 证书文件有效 (${FILE_SIZE} 字节)"
    else
        echo -e "      ${RED}✗${NC} 证书文件损坏"
        rm -f DeveloperIDG2CA.cer
        exit 1
    fi
else
    echo -e "      ${RED}✗${NC} 证书文件不存在"
    exit 1
fi

# 安装证书
echo ""
echo -e "${BLUE}[步骤 3/3]${NC} 安装证书..."
echo -e "      ${YELLOW}ℹ${NC}  需要输入系统密码"
echo ""

if sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain DeveloperIDG2CA.cer; then
    echo ""
    echo -e "      ${GREEN}✓${NC} 证书安装成功"

    # 验证安装
    echo ""
    echo -e "${BLUE}[验证]${NC} 检查安装结果..."
    sleep 1

    INSTALLED_G2=$(security find-certificate -a -c "Developer ID Certification Authority" 2>/dev/null | grep "OU=G2" | wc -l | tr -d ' ')

    if [ "${INSTALLED_G2:-0}" -gt 0 ]; then
        echo -e "      ${GREEN}✓${NC} G2 证书已正确安装到系统钥匙串"

        echo ""
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${GREEN}${BOLD}    ✓✓✓ 安装完成！${NC}"
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo ""
        echo -e "${BOLD}下一步：${NC}"
        echo -e "  ${GREEN}→${NC} 验证证书配置:"
        echo -e "    ${BLUE}bash scripts/verify-g2.sh${NC}"
        echo ""
    else
        echo -e "      ${YELLOW}⚠${NC}  安装成功但验证失败，可能需要重启安全服务"
        echo ""
        echo "尝试运行："
        echo "  sudo killall -9 securityd"
        echo "  bash scripts/verify-g2.sh"
    fi
else
    echo ""
    echo -e "      ${RED}✗${NC} 证书安装失败"
    echo ""
    echo -e "${YELLOW}备选方案：${NC}"
    echo "  1. 双击打开 /tmp/DeveloperIDG2CA.cer"
    echo "  2. 在钥匙串访问中手动导入到「系统」钥匙串"
    echo "  3. 设置信任级别为「始终信任」"
    exit 1
fi

# 清理
rm -f /tmp/DeveloperIDG2CA.cer

echo ""

