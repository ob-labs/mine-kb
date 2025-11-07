#!/usr/bin/env python3
"""
调试数据库数据格式问题的脚本
检查projects和conversations表中的数据，特别是日期字段
"""

import sys
import os

# 添加 python 目录到路径
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src-tauri', 'python'))

try:
    import seekdb
    print("✅ seekdb 模块导入成功")
except ImportError as e:
    print(f"❌ 无法导入 seekdb: {e}")
    sys.exit(1)

def main():
    db_path = os.path.expanduser("~/.local/share/mine-kb/seekdb.db")
    db_name = "mine_kb"
    
    print(f"\n🔍 检查数据库: {db_path}")
    print(f"   数据库名: {db_name}\n")
    
    try:
        # 打开数据库
        seekdb.open(db_path)
        print("✅ 数据库打开成功")
        
        # 连接到数据库
        conn = seekdb.connect(db_name)
        cursor = conn.cursor()
        print(f"✅ 连接到数据库 '{db_name}' 成功\n")
        
        # 检查 projects 表
        print("=" * 60)
        print("检查 projects 表")
        print("=" * 60)
        
        cursor.execute("SELECT id, name, description, status, document_count, created_at, updated_at FROM projects")
        projects = cursor.fetchall()
        
        print(f"找到 {len(projects)} 个项目:\n")
        
        for i, proj in enumerate(projects, 1):
            print(f"项目 {i}:")
            print(f"  ID: {proj[0]}")
            print(f"  名称: {proj[1]}")
            print(f"  描述: {proj[2]}")
            print(f"  状态: {proj[3]}")
            print(f"  文档数: {proj[4]}")
            print(f"  创建时间: {proj[5]} (类型: {type(proj[5])})")
            print(f"  更新时间: {proj[6]} (类型: {type(proj[6])})")
            
            # 检查日期字段是否为空或格式不正确
            if proj[5] is None or proj[5] == "":
                print(f"  ⚠️  创建时间为空或无效")
            if proj[6] is None or proj[6] == "":
                print(f"  ⚠️  更新时间为空或无效")
            print()
        
        # 检查 conversations 表
        print("=" * 60)
        print("检查 conversations 表")
        print("=" * 60)
        
        cursor.execute("SELECT id, project_id, title, created_at, updated_at, message_count FROM conversations")
        conversations = cursor.fetchall()
        
        print(f"找到 {len(conversations)} 个对话:\n")
        
        for i, conv in enumerate(conversations, 1):
            print(f"对话 {i}:")
            print(f"  ID: {conv[0]}")
            print(f"  项目ID: {conv[1]}")
            print(f"  标题: {conv[2]}")
            print(f"  创建时间: {conv[3]} (类型: {type(conv[3])})")
            print(f"  更新时间: {conv[4]} (类型: {type(conv[4])})")
            print(f"  消息数: {conv[5]}")
            
            # 检查日期字段是否为空或格式不正确
            if conv[3] is None or conv[3] == "":
                print(f"  ⚠️  创建时间为空或无效")
            if conv[4] is None or conv[4] == "":
                print(f"  ⚠️  更新时间为空或无效")
            print()
        
        # 检查 messages 表
        print("=" * 60)
        print("检查 messages 表")
        print("=" * 60)
        
        cursor.execute("SELECT COUNT(*) FROM messages")
        msg_count = cursor.fetchone()[0]
        print(f"找到 {msg_count} 条消息\n")
        
        if msg_count > 0:
            cursor.execute("SELECT id, conversation_id, role, created_at FROM messages LIMIT 3")
            messages = cursor.fetchall()
            
            print("显示前 3 条消息:")
            for i, msg in enumerate(messages, 1):
                print(f"\n消息 {i}:")
                print(f"  ID: {msg[0]}")
                print(f"  对话ID: {msg[1]}")
                print(f"  角色: {msg[2]}")
                print(f"  创建时间: {msg[3]} (类型: {type(msg[3])})")
                
                if msg[3] is None or msg[3] == "":
                    print(f"  ⚠️  创建时间为空或无效")
        
        conn.close()
        print("\n✅ 数据库检查完成")
        
    except Exception as e:
        print(f"\n❌ 错误: {e}")
        import traceback
        traceback.print_exc()
        return 1
    
    return 0

if __name__ == "__main__":
    sys.exit(main())

