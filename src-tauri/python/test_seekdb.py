#!/usr/bin/env python3
"""
Simple test script to verify SeekDB installation and basic operations
"""

import sys

def test_import():
    """Test if seekdb module can be imported"""
    print("Testing seekdb import...", end=" ")
    try:
        import seekdb
        print("✅ OK")
        return True
    except ImportError as e:
        print(f"❌ FAILED: {e}")
        print("\nPlease install SeekDB:")
        print("  pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple/")
        return False

def test_basic_operations():
    """Test basic database operations"""
    print("\nTesting basic operations...")
    
    try:
        import seekdb
        import tempfile
        import os
        
        # Create temp database
        temp_dir = tempfile.mkdtemp()
        db_path = os.path.join(temp_dir, "test.db")
        
        print(f"  Creating database at {db_path}...", end=" ")
        seekdb.open(db_path)
        conn = seekdb.connect("test_db")
        cursor = conn.cursor()
        print("✅")
        
        # Create table
        print("  Creating table...", end=" ")
        cursor.execute("CREATE TABLE test_table (id INT PRIMARY KEY, name VARCHAR(50))")
        print("✅")
        
        # Insert data
        print("  Inserting data...", end=" ")
        cursor.execute("INSERT INTO test_table VALUES (1, 'Test')")
        conn.commit()
        print("✅")
        
        # Query data
        print("  Querying data...", end=" ")
        cursor.execute("SELECT * FROM test_table")
        rows = cursor.fetchall()
        assert len(rows) == 1
        assert rows[0][0] == 1
        assert rows[0][1] == 'Test'
        print("✅")
        
        # Close connection
        print("  Closing connection...", end=" ")
        conn.close()
        print("✅")
        
        # Cleanup
        import shutil
        shutil.rmtree(temp_dir)
        
        print("\n✅ All basic operations passed!")
        return True
        
    except Exception as e:
        print(f"❌ FAILED: {e}")
        import traceback
        traceback.print_exc()
        return False

def test_vector_operations():
    """Test vector operations"""
    print("\nTesting vector operations...")
    
    try:
        import seekdb
        import tempfile
        import os
        
        temp_dir = tempfile.mkdtemp()
        db_path = os.path.join(temp_dir, "test_vector.db")
        
        print(f"  Creating vector database...", end=" ")
        seekdb.open(db_path)
        conn = seekdb.connect("test_vector")
        cursor = conn.cursor()
        print("✅")
        
        # Create table with vector column
        print("  Creating table with vector column...", end=" ")
        cursor.execute("""
            CREATE TABLE test_vectors (
                id INT PRIMARY KEY,
                embedding vector(3)
            )
        """)
        print("✅")
        
        # Create vector index
        print("  Creating vector index...", end=" ")
        try:
            cursor.execute("""
                CREATE VECTOR INDEX idx_test ON test_vectors(embedding) 
                WITH (distance=l2, type=hnsw, lib=vsag)
            """)
            print("✅")
        except Exception as e:
            print(f"⚠️  SKIPPED: {e}")
        
        # Insert vector data
        print("  Inserting vector data...", end=" ")
        cursor.execute("INSERT INTO test_vectors VALUES (1, '[1.0, 2.0, 3.0]')")
        cursor.execute("INSERT INTO test_vectors VALUES (2, '[2.0, 3.0, 4.0]')")
        conn.commit()
        print("✅")
        
        # Vector similarity search
        print("  Testing vector search...", end=" ")
        cursor.execute("""
            SELECT id, l2_distance(embedding, '[1.0, 2.0, 3.0]') as distance
            FROM test_vectors
            ORDER BY distance
            LIMIT 1
        """)
        rows = cursor.fetchall()
        assert len(rows) == 1
        assert rows[0][0] == 1  # Should return the exact match
        print("✅")
        
        conn.close()
        
        # Cleanup
        import shutil
        shutil.rmtree(temp_dir)
        
        print("\n✅ All vector operations passed!")
        return True
        
    except Exception as e:
        print(f"❌ FAILED: {e}")
        import traceback
        traceback.print_exc()
        return False

def main():
    """Run all tests"""
    print("="*60)
    print("SeekDB Installation Test")
    print("="*60)
    
    # Test import
    if not test_import():
        sys.exit(1)
    
    # Test basic operations
    if not test_basic_operations():
        sys.exit(1)
    
    # Test vector operations
    if not test_vector_operations():
        sys.exit(1)
    
    print("\n" + "="*60)
    print("✅ All tests passed! SeekDB is ready to use.")
    print("="*60)
    sys.exit(0)

if __name__ == "__main__":
    main()

