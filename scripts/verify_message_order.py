#!/usr/bin/env python3
"""
验证消息排序顺序的测试脚本
"""
import sys
import os

# 添加 python 目录到路径
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../src-tauri/python'))

from seekdb_bridge import SeekDbBridge, Value
from pathlib import Path

def verify_message_order():
    """验证数据库中消息的排序顺序"""
    # 获取数据库路径
    home = Path.home()
    db_path = home / '.mine-kb' / 'mine_kb.db'
    
    print(f"📂 数据库路径: {db_path}")
    
    if not db_path.exists():
        print("❌ 数据库文件不存在")
        return
    
    # 初始化 SeekDB
    bridge = SeekDbBridge()
    bridge.init_db(str(db_path.parent / 'oblite.db'), 'mine_kb')
    
    # 获取所有对话
    print("\n🔍 查询所有对话...")
    conversations = bridge.query(
        "SELECT id, title FROM conversations LIMIT 5",
        []
    )
    
    if not conversations:
        print("⚠️  没有找到对话")
        return
    
    print(f"✅ 找到 {len(conversations)} 个对话\n")
    
    # 检查每个对话的消息顺序
    for conv in conversations:
        conv_id = conv[0]
        conv_title = conv[1]
        
        print(f"📝 对话: {conv_title} (ID: {conv_id})")
        print("-" * 60)
        
        # 查询消息（不带 ORDER BY，看数据库原始顺序）
        messages = bridge.query(
            "SELECT id, role, created_at, SUBSTR(content, 1, 50) as content_preview FROM messages WHERE conversation_id = ?",
            [Value.String(conv_id)]
        )
        
        if not messages:
            print("  (无消息)\n")
            continue
        
        print(f"  找到 {len(messages)} 条消息:")
        for idx, msg in enumerate(messages, 1):
            msg_id = msg[0]
            role = msg[1]
            created_at = msg[2]
            content_preview = msg[3]
            
            print(f"  {idx}. [{role}] {created_at}")
            print(f"     内容: {content_preview}...")
            print()
        
        # 检查时间顺序
        if len(messages) > 1:
            timestamps = [msg[2] for msg in messages]
            is_ascending = all(timestamps[i] <= timestamps[i+1] for i in range(len(timestamps)-1))
            is_descending = all(timestamps[i] >= timestamps[i+1] for i in range(len(timestamps)-1))
            
            if is_ascending:
                print("  ✅ 消息按时间升序排列 (从旧到新)")
            elif is_descending:
                print("  ⚠️  消息按时间降序排列 (从新到旧)")
            else:
                print("  ❌ 消息时间顺序混乱")
        
        print()

if __name__ == '__main__':
    try:
        verify_message_order()
    except Exception as e:
        print(f"❌ 错误: {e}")
        import traceback
        traceback.print_exc()

