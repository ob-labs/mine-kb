#!/bin/bash

# 测试SeekDB向量检索修复
# 用途：验证向量检索是否正常工作，不再出现fetchall 1235错误

echo "======================================================"
echo "SeekDB向量检索修复测试"
echo "======================================================"
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查应用是否正在运行
echo "📋 步骤 1/3: 检查应用状态"
if pgrep -f "mine-kb" > /dev/null; then
    echo -e "${GREEN}✅ 应用正在运行${NC}"
else
    echo -e "${RED}❌ 应用未运行${NC}"
    echo ""
    echo "请先启动应用："
    echo "  npm run tauri dev"
    echo "或"
    echo "  npm run tauri build"
    exit 1
fi
echo ""

# 检查日志文件
echo "📋 步骤 2/3: 检查最新日志"
LOG_FILE=$(find ~/Library/Logs/com.example.mine-kb 2>/dev/null || find ~/.local/share/mine-kb/logs 2>/dev/null || echo "")

if [ -z "$LOG_FILE" ]; then
    echo -e "${YELLOW}⚠️  未找到日志文件，请手动查看应用日志${NC}"
else
    echo "日志位置: $LOG_FILE"
    
    # 检查是否有错误
    if grep -q "fetchall failed 1235" "$LOG_FILE" 2>/dev/null; then
        echo -e "${RED}❌ 发现 'fetchall failed 1235' 错误${NC}"
        echo "最新错误："
        grep -A 5 "fetchall failed 1235" "$LOG_FILE" | tail -6
        echo ""
        echo "修复可能未生效，请确认："
        echo "  1. 已重新编译应用"
        echo "  2. 已重启应用"
        exit 1
    else
        echo -e "${GREEN}✅ 未发现 'fetchall failed 1235' 错误${NC}"
    fi
    
    # 检查是否有成功的向量检索
    if grep -q "使用SeekDB向量检索" "$LOG_FILE" 2>/dev/null; then
        echo -e "${GREEN}✅ 发现向量检索日志${NC}"
        echo "最新检索："
        grep "使用SeekDB向量检索" "$LOG_FILE" | tail -3
    fi
fi
echo ""

# 提供测试指南
echo "📋 步骤 3/3: 手动验证指南"
echo ""
echo "请在应用中进行以下测试："
echo "  1. 创建一个新项目"
echo "  2. 上传一些文档"
echo "  3. 在聊天中提问（触发向量检索）"
echo ""
echo "✅ 预期结果："
echo "  - 聊天功能正常工作"
echo "  - 能找到相关文档上下文"
echo "  - 日志显示: '✅ SeekDB向量检索成功，找到 X 个相关文档块'"
echo ""
echo "❌ 如果出现问题："
echo "  - 检查日志是否有 'fetchall failed 1235' 错误"
echo "  - 确认已重新编译: cd src-tauri && cargo build --release"
echo "  - 确认已重启应用"
echo ""

# 检查编译时间
echo "📋 编译信息检查"
BINARY_PATH="src-tauri/target/release/mine-kb"
if [ -f "$BINARY_PATH" ]; then
    COMPILE_TIME=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M:%S" "$BINARY_PATH" 2>/dev/null || stat -c "%y" "$BINARY_PATH" 2>/dev/null | cut -d'.' -f1)
    echo "二进制文件编译时间: $COMPILE_TIME"
    
    # 检查是否是最近编译的（1小时内）
    if [ -n "$COMPILE_TIME" ]; then
        CURRENT_TIME=$(date +%s)
        FILE_TIME=$(stat -f "%m" "$BINARY_PATH" 2>/dev/null || stat -c "%Y" "$BINARY_PATH" 2>/dev/null)
        TIME_DIFF=$((CURRENT_TIME - FILE_TIME))
        
        if [ $TIME_DIFF -lt 3600 ]; then
            echo -e "${GREEN}✅ 二进制文件是最近编译的（${TIME_DIFF}秒前）${NC}"
        else
            HOURS=$((TIME_DIFF / 3600))
            echo -e "${YELLOW}⚠️  二进制文件编译于 ${HOURS} 小时前，可能需要重新编译${NC}"
        fi
    fi
else
    echo -e "${RED}❌ 未找到编译后的二进制文件${NC}"
    echo "请先编译: cd src-tauri && cargo build --release"
fi
echo ""

echo "======================================================"
echo "测试完成"
echo "======================================================"

