# SQLite to SeekDB Migration Guide

This guide explains how to migrate from SQLite to SeekDB and provides information about the new database architecture.

> **版本说明**: 本文档适用于 SeekDB 0.0.1.dev4 版本。从 0.0.1.dev2 升级的用户，请参考 [UPGRADE_SEEKDB_0.0.1.dev4.md](UPGRADE_SEEKDB_0.0.1.dev4.md)

## What Changed?

The application has been migrated from SQLite (via rusqlite) to **SeekDB 0.0.1.dev4**, an embedded database with native AI capabilities including:

- **Native Vector Search**: Built-in HNSW vector indexing for efficient similarity search
- **Full-text Search**: Integrated fulltext search capabilities
- **Hybrid Search**: Combined vector and keyword search (coming soon)
- **OLAP Support**: Column storage and analytical query optimization

## Architecture

The new architecture uses:

1. **Python Subprocess**: A persistent Python process manages SeekDB operations
2. **JSON Protocol**: Communication between Rust and Python via stdin/stdout using JSON
3. **SeekDB**: OceanBase's lightweight embedded database with AI-native features

```
┌─────────────────┐
│   Rust App      │
│   (Tauri)       │
└────────┬────────┘
         │ JSON over stdin/stdout
         ▼
┌─────────────────┐
│ Python Bridge   │
│ (subprocess)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    SeekDB       │
│  (oblite.db)    │
└─────────────────┘
```

## Prerequisites

### Python Setup

1. **Install Python 3.x** (if not already installed):
   ```bash
   # Ubuntu/Debian
   sudo apt update
   sudo apt install python3 python3-pip
   
   # macOS
   brew install python3
   
   # Windows
   # Download from python.org
   ```

2. **Install SeekDB package** (version 0.0.1.dev4):
   ```bash
   pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple/
   ```

3. **Verify installation**:
   ```bash
   python3 -c "import seekdb; print('SeekDB 0.0.1.dev4 installed successfully')"
   ```

## Migration Process

### Option 1: Automatic Migration (Recommended)

When you first run the application after upgrading, it will automatically detect the old SQLite database and offer to migrate it.

### Option 2: Manual Migration

If you want to manually migrate your existing SQLite database:

```bash
cd src-tauri/python
python3 migrate_sqlite_to_seekdb.py <old_sqlite_path> <new_seekdb_path>
```

**Example**:
```bash
# Migrate from old SQLite database to new SeekDB
python3 migrate_sqlite_to_seekdb.py ~/Library/Application\ Support/mine-kb/mine_kb.db ./oblite.db
```

The migration script will:
- ✅ Copy all projects
- ✅ Copy all conversations and messages
- ✅ Convert and migrate all vector embeddings (1536 dimensions)
- ✅ Create proper indexes (including HNSW vector index)
- ✅ Verify data integrity

### Migration Notes

- **Embedding Dimension**: The migration pads/truncates embeddings to 1536 dimensions (DashScope text-embedding-v1 standard)
- **Vector Index**: A native HNSW index is created for efficient vector search
- **Backup Recommended**: Always backup your data before migration
- **Time Estimate**: Migration takes approximately 1-5 minutes per 10,000 documents

## Configuration

### Database Location

SeekDB stores data in the database directory (previously `oblite.db`, can be named as needed). You can configure the location:

```json
// config.json
{
  "database": {
    "path": "./mine_kb.db",
    "name": "mine_kb"
  }
}
```

> **注意**: 从 0.0.1.dev4 版本开始，推荐使用更清晰的数据库路径名称，而非固定的 `oblite.db`。

### Python Script Location

The Python bridge script (`seekdb_bridge.py`) should be located at:
- Production: `<app_dir>/python/seekdb_bridge.py`
- Development: `src-tauri/python/seekdb_bridge.py`

## Performance Improvements

SeekDB provides several performance benefits over SQLite:

1. **Faster Vector Search**: Native HNSW indexing (10-100x faster for large datasets)
2. **Better Scalability**: Optimized for AI/ML workloads
3. **Native Vector Types**: No need to serialize/deserialize embeddings
4. **Approximate Search**: Trade-off between speed and accuracy

## Troubleshooting

### Python subprocess not starting

**Error**: `Failed to start Python process`

**Solutions**:
- Verify Python 3 is installed: `python3 --version`
- Check SeekDB installation: `pip list | grep seekdb`
- Ensure script path is correct
- Check Python PATH environment variable

### Import error: seekdb module not found

**Error**: `ModuleNotFoundError: No module named 'seekdb'` or `No module named 'oblite'`

**Solution**:
```bash
# 安装最新版本
pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple/

# 验证安装
python3 -c "import seekdb; print('OK')"
```

> **注意**: 从 0.0.1.dev4 版本开始，应使用 `import seekdb` 而非 `import oblite`。详见 [升级指南](UPGRADE_SEEKDB_0.0.1.dev4.md)。

### Vector dimension mismatch

**Error**: `Vector dimension mismatch`

**Solution**: 
- Ensure you're using DashScope text-embedding-v1 (1536 dimensions)
- Re-run migration with `--force-dimension 1536` flag

### Subprocess communication timeout

**Error**: `Timeout waiting for subprocess response`

**Solutions**:
- Check if Python process is still running
- Restart the application
- Check system resources (CPU/memory)

## Rollback to SQLite

If you need to rollback to the old SQLite implementation:

1. Checkout the previous version:
   ```bash
   git checkout <previous_commit>
   ```

2. Rebuild the application:
   ```bash
   cd src-tauri
   cargo build --release
   ```

## Development

### Testing SeekDB Operations

```bash
# Start Python bridge in standalone mode
cd src-tauri/python
python3 seekdb_bridge.py
```

Then send JSON commands via stdin:
```json
{"command": "init", "params": {"db_path": "./test.db", "db_name": "test"}}
{"command": "query", "params": {"sql": "SELECT * FROM projects", "values": []}}
```

### Adding New Database Operations

1. Add command handler in `seekdb_bridge.py`
2. Add Rust wrapper in `python_subprocess.rs`
3. Add high-level method in `seekdb_adapter.rs`

## Support

For issues or questions:
- Check [docs/seekdb.md](seekdb.md) for SeekDB basic documentation
- Check [docs/SEEKDB_USAGE_GUIDE.md](SEEKDB_USAGE_GUIDE.md) for comprehensive usage guide
- Check [docs/UPGRADE_SEEKDB_0.0.1.dev4.md](UPGRADE_SEEKDB_0.0.1.dev4.md) for version upgrade guide
- Create an issue on GitHub
- Consult SeekDB documentation: [SeekDB Docs](https://www.oceanbase.com/)

## Future Enhancements

Planned features leveraging SeekDB capabilities:
- [ ] Hybrid search (vector + fulltext)
- [ ] Materialized views for faster aggregations
- [ ] External table support for document imports
- [ ] Advanced analytics with OLAP features

