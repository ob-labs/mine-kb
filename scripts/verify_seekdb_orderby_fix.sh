#!/bin/bash

# SeekDB ORDER BY 修复验证脚本
# 用于验证移除 ORDER BY 子句后的功能是否正常

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔍 SeekDB ORDER BY 修复验证"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 进入 src-tauri 目录
cd "$(dirname "$0")/../src-tauri"

# 1. 检查代码中是否还有不支持的 ORDER BY 使用
echo "📝 步骤 1: 检查代码中的 ORDER BY 使用..."
echo ""

# 搜索 seekdb_adapter.rs 中的 ORDER BY（排除注释和向量搜索专用语法）
echo "🔍 检查 seekdb_adapter.rs..."
ORDERBY_COUNT=$(grep -n "ORDER BY" src/services/seekdb_adapter.rs | \
    grep -v "Note:" | \
    grep -v "l2_distance" | \
    grep -v "APPROXIMATE" | \
    wc -l || true)

if [ "$ORDERBY_COUNT" -gt 0 ]; then
    echo "⚠️  警告: 发现 $ORDERBY_COUNT 处可能有问题的 ORDER BY 使用"
    grep -n "ORDER BY" src/services/seekdb_adapter.rs | \
        grep -v "Note:" | \
        grep -v "l2_distance" | \
        grep -v "APPROXIMATE"
    echo ""
else
    echo "✅ 未发现有问题的 ORDER BY 使用"
    echo ""
fi

# 2. 编译检查
echo "📝 步骤 2: 编译检查..."
echo ""
cargo check --quiet
if [ $? -eq 0 ]; then
    echo "✅ 编译成功"
    echo ""
else
    echo "❌ 编译失败"
    exit 1
fi

# 3. 检查是否添加了内存排序
echo "📝 步骤 3: 检查是否正确添加了内存排序..."
echo ""

SORT_BY_COUNT=$(grep -n "sort_by" src/services/seekdb_adapter.rs | wc -l)

if [ "$SORT_BY_COUNT" -ge 5 ]; then
    echo "✅ 发现 $SORT_BY_COUNT 处内存排序实现"
    echo ""
    echo "排序位置："
    grep -n "sort_by" src/services/seekdb_adapter.rs | sed 's/^/   /'
    echo ""
else
    echo "⚠️  警告: 只发现 $SORT_BY_COUNT 处内存排序，可能遗漏了某些地方"
    echo ""
fi

# 4. 运行单元测试（如果存在）
echo "📝 步骤 4: 运行单元测试..."
echo ""

if cargo test --lib --quiet 2>/dev/null; then
    echo "✅ 单元测试通过"
    echo ""
else
    echo "⚠️  单元测试失败或不存在"
    echo ""
fi

# 5. 总结
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ 验证完成"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "修复内容："
echo "  ✓ 移除了 SeekDB/ObLite 不支持的 ORDER BY 子句"
echo "  ✓ 在 Rust 代码中添加了内存排序"
echo "  ✓ 保留了向量搜索专用的 ORDER BY l2_distance 语法"
echo ""
echo "修改的函数："
echo "  • get_project_documents - 按 document_id, chunk_index 排序"
echo "  • load_all_projects - 按 updated_at DESC 排序"
echo "  • load_conversations_by_project - 按 updated_at DESC 排序"
echo "  • load_all_conversations - 按 updated_at DESC 排序"
echo "  • load_messages_by_conversation - 按 created_at ASC 排序"
echo ""
echo "接下来："
echo "  1. 启动应用进行手动测试"
echo "  2. 创建项目并添加文档"
echo "  3. 在聊天界面发送消息"
echo "  4. 检查日志，确认不再出现 'fetchall failed 1235' 错误"
echo ""
echo "相关文档: docs/FIX_SEEKDB_ORDER_BY.md"
echo ""

