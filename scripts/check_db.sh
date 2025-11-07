#!/bin/bash
# 检查数据库内容

DB_PATH="$HOME/Library/Application Support/com.mine-kb.app/mine_kb.db"

echo "=== 数据库路径 ==="
echo "$DB_PATH"
echo ""

echo "=== 对话列表 ==="
sqlite3 "$DB_PATH" "SELECT id, title, message_count FROM conversations LIMIT 5;"
echo ""

echo "=== 消息总数 ==="
sqlite3 "$DB_PATH" "SELECT COUNT(*) as total FROM messages;"
echo ""

echo "=== 消息列表（如果有）==="
sqlite3 "$DB_PATH" "SELECT id, conversation_id, role, substr(content, 1, 30) as content_preview FROM messages LIMIT 10;"
echo ""

echo "=== 检查特定对话的消息 ==="
sqlite3 "$DB_PATH" "SELECT id, role, substr(content, 1, 50) FROM messages WHERE conversation_id = '9309bbca-4fe8-4030-be06-99e77f80e518';"
echo ""

echo "=== 检查所有表 ==="
sqlite3 "$DB_PATH" ".tables"
echo ""

