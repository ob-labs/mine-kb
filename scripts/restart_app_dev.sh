#!/bin/bash
# 重启应用（开发模式）以应用消息排序修复

echo "🔄 重启应用以应用消息排序修复"
echo "================================"
echo ""

# 查找并终止现有的 Tauri 进程
echo "🔍 查找运行中的应用进程..."
PIDS=$(pgrep -f "mine-kb" | grep -v $$ | grep -v grep)

if [ -z "$PIDS" ]; then
    echo "✅ 没有找到运行中的应用进程"
else
    echo "⚠️  找到运行中的进程: $PIDS"
    echo "   正在终止..."
    echo "$PIDS" | xargs kill -9 2>/dev/null
    sleep 2
    echo "✅ 进程已终止"
fi

echo ""
echo "🚀 启动应用（开发模式）..."
echo "   - 修复已应用：消息按时间从旧到新排序"
echo "   - 日志中会显示：'已按时间排序'"
echo ""
echo "📝 验证步骤："
echo "   1. 等待应用启动完成"
echo "   2. 打开一个历史对话"
echo "   3. 检查消息顺序：最早的在上面，最新的在下面"
echo "   4. 发送一条新消息，应该出现在底部"
echo ""

cd /home/ubuntu/Desktop/mine-kb

# 启动应用
npm run tauri dev

