#!/usr/bin/env python3
"""
测试 datetime 序列化修复
"""

import sys
import json
from datetime import datetime, date
from decimal import Decimal

# 导入 SeekDBBridge 类
sys.path.insert(0, '.')
from seekdb_bridge import SeekDBBridge

def test_datetime_conversion():
    """测试 datetime 转换功能"""
    bridge = SeekDBBridge()
    
    # 测试用例
    test_cases = [
        ("datetime", datetime(2025, 1, 29, 10, 30, 45)),
        ("date", date(2025, 1, 29)),
        ("decimal", Decimal("123.456")),
        ("string", "test string"),
        ("int", 42),
        ("float", 3.14),
        ("bool", True),
        ("none", None),
        ("list", [1, datetime(2025, 1, 29), "text"]),
        ("dict", {"date": datetime(2025, 1, 29), "value": 100}),
    ]
    
    print("测试 datetime 转换功能")
    print("=" * 60)
    
    all_passed = True
    for name, value in test_cases:
        try:
            converted = bridge.convert_value_for_json(value)
            json_str = json.dumps(converted)  # 尝试序列化为 JSON
            print(f"✅ {name:12} | {str(value)[:30]:30} -> {json_str[:40]}")
        except Exception as e:
            print(f"❌ {name:12} | {str(value)[:30]:30} -> ERROR: {e}")
            all_passed = False
    
    print("=" * 60)
    if all_passed:
        print("✅ 所有测试通过！")
        return 0
    else:
        print("❌ 部分测试失败")
        return 1

if __name__ == "__main__":
    exit(test_datetime_conversion())

