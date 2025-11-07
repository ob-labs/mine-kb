#!/bin/bash
# 测试消息顺序修复

echo "🧪 测试消息顺序修复"
echo "===================="
echo ""

# 检查数据库是否存在
DB_PATH="$HOME/.mine-kb/mine_kb.db"
if [ ! -f "$DB_PATH" ]; then
    echo "❌ 数据库文件不存在: $DB_PATH"
    echo "   请先运行应用并创建一些对话"
    exit 1
fi

echo "✅ 找到数据库: $DB_PATH"
echo ""

# 运行 Python 验证脚本
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PYTHON_SCRIPT="$SCRIPT_DIR/verify_message_order.py"

if [ ! -f "$PYTHON_SCRIPT" ]; then
    echo "❌ 验证脚本不存在: $PYTHON_SCRIPT"
    exit 1
fi

echo "🔍 检查数据库中的消息顺序..."
echo ""

python3 "$PYTHON_SCRIPT"

echo ""
echo "✅ 测试完成"
echo ""
echo "📝 说明："
echo "   - 消息应该按时间升序排列（从旧到新）"
echo "   - 最早的消息显示在上面"
echo "   - 最新的消息显示在下面"
echo ""
echo "🚀 重启应用后，进入任意历史对话验证修复效果"

