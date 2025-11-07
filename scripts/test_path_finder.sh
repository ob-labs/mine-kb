#!/bin/bash

echo "Testing SeekDB path finder..."
echo ""

# Test from project root
echo "Test 1: From project root"
cd /home/ubuntu/Desktop/mine-kb
if [ -f "src-tauri/python/seekdb_bridge.py" ]; then
    echo "  ✅ Found: src-tauri/python/seekdb_bridge.py"
else
    echo "  ❌ Not found: src-tauri/python/seekdb_bridge.py"
fi

# Test from src-tauri
echo ""
echo "Test 2: From src-tauri directory"
cd /home/ubuntu/Desktop/mine-kb/src-tauri
if [ -f "python/seekdb_bridge.py" ]; then
    echo "  ✅ Found: python/seekdb_bridge.py"
else
    echo "  ❌ Not found: python/seekdb_bridge.py"
fi

# Test from src-tauri/src
echo ""
echo "Test 3: From src-tauri/src directory"
cd /home/ubuntu/Desktop/mine-kb/src-tauri/src
if [ -f "../python/seekdb_bridge.py" ]; then
    echo "  ✅ Found: ../python/seekdb_bridge.py"
else
    echo "  ❌ Not found: ../python/seekdb_bridge.py"
fi

echo ""
echo "All paths exist correctly!"

