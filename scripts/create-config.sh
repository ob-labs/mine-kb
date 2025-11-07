#!/bin/bash

# 创建MineKB配置文件的脚本

set -e

# 颜色输出
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}MineKB 配置文件创建工具${NC}"
echo "================================"
echo ""

# 确定配置文件路径
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    CONFIG_DIR="$HOME/.local/share/mine-kb"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    CONFIG_DIR="$HOME/Library/Application Support/mine-kb"
else
    echo -e "${RED}不支持的操作系统: $OSTYPE${NC}"
    exit 1
fi

CONFIG_FILE="$CONFIG_DIR/config.json"

echo "配置文件路径: $CONFIG_FILE"
echo ""

# 检查配置文件是否已存在
if [ -f "$CONFIG_FILE" ]; then
    echo -e "${YELLOW}警告: 配置文件已存在${NC}"
    read -p "是否覆盖? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "操作已取消"
        exit 0
    fi
    # 备份现有配置
    cp "$CONFIG_FILE" "$CONFIG_FILE.backup.$(date +%Y%m%d%H%M%S)"
    echo -e "${GREEN}已备份现有配置${NC}"
fi

# 创建目录
mkdir -p "$CONFIG_DIR"

# 询问用户API密钥
echo "请输入您的阿里云百炼 API Key:"
echo "(如果暂时没有，可以输入 'sk-test-key'，稍后再修改)"
read -p "API Key: " API_KEY

if [ -z "$API_KEY" ]; then
    API_KEY="sk-test-key"
    echo -e "${YELLOW}使用默认测试密钥，应用将无法正常工作，请稍后修改${NC}"
fi

# 创建配置文件
cat > "$CONFIG_FILE" << EOF
{
  "llm": {
    "provider": "dashscope",
    "api_key": "$API_KEY",
    "model": "qwen-max",
    "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1",
    "max_tokens": 4000,
    "temperature": 0.7,
    "stream": true
  },
  "embedding": {
    "provider": "dashscope",
    "api_key": "$API_KEY",
    "model": "text-embedding-v3",
    "base_url": "https://dashscope.aliyuncs.com/api/v1"
  }
}
EOF

echo ""
echo -e "${GREEN}✅ 配置文件创建成功！${NC}"
echo ""
echo "配置文件位置: $CONFIG_FILE"
echo ""
echo "如需修改配置，请编辑该文件："
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "  open '$CONFIG_FILE'"
else
    echo "  nano '$CONFIG_FILE'"
fi
echo ""
echo "获取 API Key: https://dashscope.console.aliyun.com/api-key"
echo ""
echo -e "${GREEN}现在可以启动 MineKB 了！${NC}"

