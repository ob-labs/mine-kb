#!/bin/bash
# 验证启动错误修复脚本

echo "======================================"
echo "验证启动错误修复"
echo "======================================"
echo ""

# 进入项目目录
cd "$(dirname "$0")/.." || exit 1

echo "1. 检查编译状态..."
cd src-tauri || exit 1
if cargo check --quiet 2>&1 | grep -q "error"; then
    echo "❌ 编译失败"
    exit 1
fi
echo "✅ 编译成功"
echo ""

echo "2. 清理旧的日志文件..."
LOG_FILE="$HOME/.local/share/mine-kb/app.log"
if [ -f "$LOG_FILE" ]; then
    mv "$LOG_FILE" "$LOG_FILE.backup.$(date +%s)"
    echo "✅ 旧日志已备份"
else
    echo "ℹ️  没有旧日志文件"
fi
echo ""

echo "3. 准备测试启动..."
echo "   日志文件: $LOG_FILE"
echo "   数据库: ~/.local/share/mine-kb/oblite.db"
echo ""

echo "4. 启动应用程序 (将在后台运行 10 秒)..."
cd ..
timeout 10s npm run tauri dev > /tmp/mine-kb-startup.log 2>&1 &
APP_PID=$!
echo "   进程 ID: $APP_PID"
echo ""

echo "5. 等待启动..."
sleep 8
echo ""

echo "6. 检查日志中的错误..."
if [ -f "$LOG_FILE" ]; then
    echo ""
    echo "查找 'premature end of input' 错误..."
    if grep -q "premature end of input" "$LOG_FILE"; then
        echo "❌ 仍然存在 'premature end of input' 错误"
        echo ""
        echo "错误详情:"
        grep -A 3 "premature end of input" "$LOG_FILE"
        exit 1
    else
        echo "✅ 未发现 'premature end of input' 错误"
    fi
    
    echo ""
    echo "查找其他 ERROR 级别日志..."
    if grep -q "ERROR" "$LOG_FILE"; then
        echo "⚠️  发现其他 ERROR 日志:"
        grep "ERROR" "$LOG_FILE" | tail -5
    else
        echo "✅ 没有 ERROR 级别日志"
    fi
    
    echo ""
    echo "检查项目和对话加载..."
    if grep -q "成功加载.*个项目" "$LOG_FILE"; then
        PROJECT_COUNT=$(grep "成功加载.*个项目" "$LOG_FILE" | tail -1 | grep -o '[0-9]\+' | head -1)
        echo "✅ 成功加载 $PROJECT_COUNT 个项目"
    else
        echo "⚠️  未找到项目加载日志"
    fi
    
    if grep -q "成功加载.*个对话" "$LOG_FILE"; then
        CONV_COUNT=$(grep "成功加载.*个对话" "$LOG_FILE" | tail -1 | grep -o '[0-9]\+' | head -1)
        echo "✅ 成功加载 $CONV_COUNT 个对话"
    else
        echo "⚠️  未找到对话加载日志"
    fi
    
    echo ""
    echo "检查警告信息..."
    if grep -q "WARN" "$LOG_FILE"; then
        echo "ℹ️  发现警告信息 (这是正常的，表示在处理不完整数据):"
        grep "WARN.*时间" "$LOG_FILE" | head -3
    else
        echo "✅ 没有警告信息"
    fi
else
    echo "⚠️  日志文件不存在: $LOG_FILE"
    echo "   应用可能未正确启动"
fi

echo ""
echo "7. 清理..."
if ps -p $APP_PID > /dev/null 2>&1; then
    kill $APP_PID 2>/dev/null
    echo "✅ 应用进程已终止"
fi

echo ""
echo "======================================"
echo "验证完成"
echo "======================================"
echo ""
echo "如果需要查看完整日志:"
echo "  tail -f $LOG_FILE"
echo ""

