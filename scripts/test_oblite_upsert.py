#!/usr/bin/env python3
"""
测试 ObLite 数据库的 UPSERT 语法支持
"""
import sys
import os

# 添加父目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

try:
    import seekdb
    print("✅ seekdb 模块加载成功")
except ImportError as e:
    print(f"❌ 无法导入 seekdb: {e}")
    print(f"PYTHONPATH: {os.environ.get('PYTHONPATH', '(未设置)')}")
    sys.exit(1)

def test_upsert_syntax():
    """测试不同的 UPSERT 语法"""
    
    # 创建临时测试数据库
    test_db_path = "/tmp/test_seekdb_upsert.db"
    test_db_name = "test_upsert"
    
    print(f"\n📋 测试 SeekDB UPSERT 语法")
    print(f"数据库路径: {test_db_path}")
    print(f"数据库名: {test_db_name}")
    
    try:
        # 初始化数据库
        seekdb.open(test_db_path)
        print("✅ 数据库打开成功")
        
        # 创建数据库
        admin_conn = seekdb.connect("")
        admin_cursor = admin_conn.cursor()
        admin_cursor.execute(f"CREATE DATABASE IF NOT EXISTS `{test_db_name}`")
        admin_conn.commit()
        admin_conn.close()
        print(f"✅ 数据库 '{test_db_name}' 已创建")
        
        # 连接到测试数据库
        conn = seekdb.connect(test_db_name)
        cursor = conn.cursor()
        print(f"✅ 已连接到数据库 '{test_db_name}'")
        
        # 创建测试表
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS test_projects (
                id VARCHAR(36) PRIMARY KEY,
                name TEXT NOT NULL,
                value INTEGER DEFAULT 0
            )
        """)
        conn.commit()
        print("✅ 测试表创建成功")
        
        # 测试 1: 基本 INSERT
        print("\n📝 测试 1: 基本 INSERT")
        cursor.execute("INSERT INTO test_projects VALUES ('test-1', 'Project 1', 10)")
        conn.commit()
        cursor.execute("SELECT * FROM test_projects WHERE id = 'test-1'")
        result = cursor.fetchone()
        print(f"   结果: {result}")
        
        # 测试 2: REPLACE INTO (MySQL 风格)
        print("\n📝 测试 2: REPLACE INTO")
        try:
            cursor.execute("REPLACE INTO test_projects VALUES ('test-1', 'Project 1 Updated', 20)")
            conn.commit()
            cursor.execute("SELECT * FROM test_projects WHERE id = 'test-1'")
            result = cursor.fetchone()
            print(f"   ✅ REPLACE INTO 语法支持！")
            print(f"   结果: {result}")
        except Exception as e:
            print(f"   ❌ REPLACE INTO 不支持: {e}")
        
        # 测试 3: ON DUPLICATE KEY UPDATE (MySQL 风格)
        print("\n📝 测试 3: ON DUPLICATE KEY UPDATE")
        try:
            cursor.execute("""
                INSERT INTO test_projects VALUES ('test-2', 'Project 2', 30)
                ON DUPLICATE KEY UPDATE name = 'Project 2 Updated', value = 40
            """)
            conn.commit()
            cursor.execute("SELECT * FROM test_projects WHERE id = 'test-2'")
            result = cursor.fetchone()
            print(f"   ✅ ON DUPLICATE KEY UPDATE 语法支持！")
            print(f"   结果: {result}")
            
            # 再次执行以测试更新
            cursor.execute("""
                INSERT INTO test_projects VALUES ('test-2', 'Project 2 Updated Again', 50)
                ON DUPLICATE KEY UPDATE name = 'Project 2 Updated Again', value = 50
            """)
            conn.commit()
            cursor.execute("SELECT * FROM test_projects WHERE id = 'test-2'")
            result = cursor.fetchone()
            print(f"   结果（更新后）: {result}")
        except Exception as e:
            print(f"   ❌ ON DUPLICATE KEY UPDATE 不支持: {e}")
        
        # 测试 4: ON CONFLICT DO UPDATE (SQLite 风格)
        print("\n📝 测试 4: ON CONFLICT DO UPDATE")
        try:
            cursor.execute("""
                INSERT INTO test_projects VALUES ('test-3', 'Project 3', 60)
                ON CONFLICT(id) DO UPDATE SET name = 'Project 3 Updated', value = 70
            """)
            conn.commit()
            cursor.execute("SELECT * FROM test_projects WHERE id = 'test-3'")
            result = cursor.fetchone()
            print(f"   ✅ ON CONFLICT DO UPDATE 语法支持！")
            print(f"   结果: {result}")
        except Exception as e:
            print(f"   ❌ ON CONFLICT DO UPDATE 不支持: {e}")
        
        # 测试 5: INSERT ... ON CONFLICT DO UPDATE with excluded (SQLite 风格)
        print("\n📝 测试 5: INSERT ... ON CONFLICT DO UPDATE with excluded")
        try:
            cursor.execute("""
                INSERT INTO test_projects (id, name, value)
                VALUES ('test-4', 'Project 4', 80)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    value = excluded.value
            """)
            conn.commit()
            cursor.execute("SELECT * FROM test_projects WHERE id = 'test-4'")
            result = cursor.fetchone()
            print(f"   ✅ ON CONFLICT DO UPDATE with excluded 语法支持！")
            print(f"   结果: {result}")
        except Exception as e:
            print(f"   ❌ ON CONFLICT DO UPDATE with excluded 不支持: {e}")
        
        # 显示所有数据
        print("\n📊 最终数据:")
        cursor.execute("SELECT * FROM test_projects ORDER BY id")
        for row in cursor.fetchall():
            print(f"   {row}")
        
        # 清理
        conn.close()
        print("\n✅ 测试完成")
        
    except Exception as e:
        print(f"\n❌ 测试失败: {e}")
        import traceback
        traceback.print_exc()
        return False
    
    return True

if __name__ == "__main__":
    success = test_upsert_syntax()
    sys.exit(0 if success else 1)

