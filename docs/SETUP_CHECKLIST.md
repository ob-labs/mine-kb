# SeekDB Setup Checklist

Follow these steps to set up the application with SeekDB support.

## Prerequisites

- [ ] Python 3.8 or higher installed
  ```bash
  python3 --version
  ```

- [ ] pip3 installed
  ```bash
  pip3 --version
  ```

- [ ] Rust 1.70+ installed (for building)
  ```bash
  rustc --version
  ```

- [ ] Node.js 16+ installed (for frontend)
  ```bash
  node --version
  ```

## Installation Steps

### 1. Install SeekDB Python Package

```bash
# Using pip with Tsinghua mirror (recommended in China)
pip3 install seekdb==0.0.1.dev2 -i https://pypi.tuna.tsinghua.edu.cn/simple/

# Or using the installation script
cd src-tauri/python
bash install_deps.sh
```

### 2. Verify Installation

```bash
cd src-tauri/python
python3 test_seekdb.py
```

Expected output:
```
============================================================
SeekDB Installation Test
============================================================
Testing oblite import... ✅ OK

Testing basic operations...
  Creating database at /tmp/.../test.db... ✅
  Creating table... ✅
  Inserting data... ✅
  Querying data... ✅
  Closing connection... ✅

✅ All basic operations passed!
...
✅ All tests passed! SeekDB is ready to use.
============================================================
```

### 3. Install Application Dependencies

```bash
# From project root
cd /home/ubuntu/Desktop/mine-kb

# Install frontend dependencies
npm install  # or tnpm install

# Rust dependencies will be installed automatically during build
```

### 4. Configure Application

```bash
# Copy config template
cp src-tauri/config.example.json src-tauri/config.json

# Edit config file and add your API keys
nano src-tauri/config.json
```

### 5. Build and Run

```bash
# Development mode
npm run tauri:dev

# Production build
npm run tauri:build
```

## Migration from SQLite (If Upgrading)

If you have an existing SQLite database:

```bash
cd src-tauri/python
python3 migrate_sqlite_to_seekdb.py <old_sqlite_path> ./oblite.db
```

Example:
```bash
# macOS
python3 migrate_sqlite_to_seekdb.py ~/Library/Application\ Support/mine-kb/mine_kb.db ./oblite.db

# Linux
python3 migrate_sqlite_to_seekdb.py ~/.local/share/mine-kb/mine_kb.db ./oblite.db

# Windows
python3 migrate_sqlite_to_seekdb.py %APPDATA%\mine-kb\mine_kb.db .\oblite.db
```

## Troubleshooting

### Issue: "ModuleNotFoundError: No module named 'oblite'"

**Solution:**
```bash
pip3 install seekdb==0.0.1.dev2 -i https://pypi.tuna.tsinghua.edu.cn/simple/
```

### Issue: "Failed to start Python process"

**Solutions:**
1. Verify Python 3 is in PATH:
   ```bash
   which python3
   ```

2. Check if SeekDB is installed:
   ```bash
   python3 -c "import oblite; print('OK')"
   ```

3. Check script permissions:
   ```bash
   chmod +x src-tauri/python/seekdb_bridge.py
   ```

### Issue: "Vector index creation failed"

This is usually not critical. The application will work without the index, just slower for large datasets.

To manually create index later:
```sql
CREATE VECTOR INDEX idx_embedding ON vector_documents(embedding) 
WITH (distance=l2, type=hnsw, lib=vsag)
```

### Issue: Subprocess communication timeout

**Solutions:**
1. Restart the application
2. Check system resources (CPU/memory)
3. Check Python process logs in stderr

## Verification

After setup, verify everything works:

1. [ ] Application starts without errors
2. [ ] Can create a new project
3. [ ] Can upload a document
4. [ ] Document processing completes
5. [ ] Can query the document via chat
6. [ ] Chat responses are generated correctly
7. [ ] Data persists after application restart

## Getting Help

- Check [MIGRATION_SEEKDB.md](MIGRATION_SEEKDB.md) for detailed migration guide
- Check [MIGRATION_SUMMARY.md](MIGRATION_SUMMARY.md) for technical details
- Check [docs/seekdb.md](docs/seekdb.md) for SeekDB documentation
- Create an issue on GitHub

## Checklist Summary

- [ ] Python 3.8+ installed
- [ ] SeekDB package installed
- [ ] Installation test passed
- [ ] Application dependencies installed
- [ ] Configuration file created
- [ ] Application builds successfully
- [ ] Application runs without errors
- [ ] (If upgrading) Data migrated from SQLite

Once all items are checked, you're ready to use MineKB with SeekDB! 🎉

