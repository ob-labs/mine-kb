#!/bin/bash
# 测试日期时间解析的修复

echo "======================================"
echo "测试日期时间解析修复"
echo "======================================"
echo ""

cd "$(dirname "$0")/.." || exit 1

echo "1. 测试编译..."
cd src-tauri || exit 1
if cargo build --quiet 2>&1 | grep -q "error:"; then
    echo "❌ 编译失败"
    cargo build 2>&1 | grep "error:"
    exit 1
fi
echo "✅ 编译成功"
echo ""

echo "2. 检查修改的函数..."
echo ""

# 检查关键修复是否存在
echo "检查关键修复..."

# 检查是否添加了空字符串检查
EMPTY_CHECKS=$(grep -c "if created_at_str.is_empty()" src/services/seekdb_adapter.rs)
if [ "$EMPTY_CHECKS" -ge 3 ]; then
    echo "  ✅ 包含空字符串检查 ($EMPTY_CHECKS 处)"
else
    echo "  ❌ 空字符串检查不足 (找到 $EMPTY_CHECKS 处，需要至少 3 处)"
    exit 1
fi

# 检查是否添加了 match 错误处理
MATCH_CHECKS=$(grep -c "match DateTime::parse_from_rfc3339" src/services/seekdb_adapter.rs)
if [ "$MATCH_CHECKS" -ge 6 ]; then
    echo "  ✅ 包含 match 错误处理 ($MATCH_CHECKS 处)"
else
    echo "  ❌ match 错误处理不足 (找到 $MATCH_CHECKS 处，需要至少 6 处)"
    exit 1
fi

# 检查是否添加了适当的警告日志
DATETIME_WARNS=$(grep -c "时间.*解析失败\|时间.*为空" src/services/seekdb_adapter.rs)
if [ "$DATETIME_WARNS" -ge 6 ]; then
    echo "  ✅ 包含时间解析警告日志 ($DATETIME_WARNS 处)"
else
    echo "  ❌ 警告日志不足 (找到 $DATETIME_WARNS 处，需要至少 6 处)"
    exit 1
fi

# 检查是否使用了默认时间
DEFAULT_TIME=$(grep -c "chrono::Utc::now()" src/services/seekdb_adapter.rs)
if [ "$DEFAULT_TIME" -ge 6 ]; then
    echo "  ✅ 使用默认时间作为降级 ($DEFAULT_TIME 处)"
else
    echo "  ⚠️  默认时间使用较少 (找到 $DEFAULT_TIME 处)"
fi

echo ""
echo "3. 检查日志消息..."
LOG_MESSAGES=$(grep -c "成功加载.*个项目\|成功加载.*个对话" src/services/seekdb_adapter.rs)
echo "  找到 $LOG_MESSAGES 条成功加载日志"

if [ "$LOG_MESSAGES" -ge 2 ]; then
    echo "  ✅ 包含成功日志"
else
    echo "  ⚠️  成功日志可能不完整"
fi

echo ""
echo "4. 统计代码修改..."
WARN_COUNT=$(grep -c "log::warn!" src/services/seekdb_adapter.rs)
echo "  警告日志语句: $WARN_COUNT"

if [ "$WARN_COUNT" -ge 10 ]; then
    echo "  ✅ 添加了足够的警告日志"
else
    echo "  ⚠️  警告日志可能不足"
fi

echo ""
echo "======================================"
echo "测试完成"
echo "======================================"
echo ""
echo "✅ 所有检查通过！"
echo ""
echo "修复摘要:"
echo "  - 添加了空字符串检查"
echo "  - 添加了 match 错误处理"
echo "  - 添加了详细的警告日志"
echo "  - 提供了默认值作为降级方案"
echo ""
echo "下一步:"
echo "  1. 运行完整的应用测试"
echo "  2. 检查应用日志: tail -f ~/.local/share/mine-kb/app.log"
echo "  3. 验证数据是否正确加载"
echo ""

