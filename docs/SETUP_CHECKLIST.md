# SeekDB Setup Checklist

MineKB uses **seekdb-rs** (Rust) for embedded SeekDB. No Python or venv is required.

## Prerequisites

**Build / development** (for local dev or packaging only):

- [ ] Rust 1.70+ installed
  ```bash
  rustc --version
  ```

- [ ] Node.js 16+ installed (frontend build)
  ```bash
  node --version
  ```

**End-user runtime**: No extra runtime (Python, Node, etc.). The built app is self-contained.

## Installation Steps

### 1. Install application dependencies

```bash
# From project root
npm install   # or tnpm install

# Rust (including seekdb-rs) is pulled automatically on build
```

### 2. Configure application

```bash
cp src-tauri/config.example.json src-tauri/config.json
# Edit config.json and add your API keys
```

### 3. Build and run

```bash
# Development (default data dir: CONFIG_DIR=com.mine-kb)
npm run tauri:dev

# Custom data directory
CONFIG_DIR=/path/to/your/data npm run tauri:dev

# Production build
npm run tauri:build
```

## Data directory

- Default: `{app data dir}/mine_kb.db/` (e.g. `~/Library/Application Support/com.mine-kb.app/mine_kb.db/` on macOS).
- Override: set `CONFIG_DIR` to the desired app data root.

## References

- [SeekDB in MineKB](./seekdb.md)
- [Development tutorial](./MINEKB_DEV_TUTORIAL.md)
