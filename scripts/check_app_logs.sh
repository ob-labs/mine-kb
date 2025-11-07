#!/bin/bash

echo "=== 检查应用日志 ==="
echo ""
echo "请在另一个终端运行应用并发送消息，然后查看此处的实时日志"
echo "----------------------------------------"
echo ""

# 查找应用进程
APP_NAME="Mine KB"

# 如果是开发模式，日志会输出到控制台
# 如果是生产模式，检查系统日志

echo "方法1: 如果你是用 cargo tauri dev 运行的，日志会直接显示在那个终端"
echo ""
echo "方法2: 查看系统日志（最近5分钟）"
echo "运行: log show --predicate 'process contains \"Mine KB\" OR process contains \"mine-kb\"' --last 5m --style compact | grep -E '(save_message|add_message|INSERT|保存消息)'"
echo ""
echo "方法3: 如果应用正在运行，检查其输出"
echo ""

# 尝试找到应用进程的 PID
PID=$(pgrep -f "Mine KB" | head -1)

if [ -n "$PID" ]; then
    echo "✅ 找到应用进程 PID: $PID"
    echo ""
    echo "检查进程信息:"
    ps -p $PID -o pid,command
else
    echo "❌ 应用未运行或无法找到进程"
fi

echo ""
echo "========================================="
echo "重要：请在运行应用的终端中查找以下关键日志："
echo "----------------------------------------"
echo "✓ 'add_message 开始: conversation_id=...'"
echo "✓ 'save_message: id=...'"
echo "✓ 'INSERT 成功: rows=...'"
echo "✓ 'save_message 成功: rows_affected=...'"
echo "✓ 'add_message 完成: message_id=...'"
echo ""
echo "如果看到这些日志，说明代码执行了"
echo "如果没有看到，说明代码没有执行或被提前返回"
echo "========================================="

