# SeekDB in MineKB

MineKB uses **seekdb-rs** (Rust SDK) to talk to embedded SeekDB. There is no Python dependency; the app uses the native Rust client only.

## 1. Product overview

SeekDB is an AI-Native embedded database that supports:

- **TP (transactions)** – OLTP
- **AP (analytics)** – OLAP, columnar, materialized views
- **AI (vectors)** – vector type, HNSW index, hybrid search

MineKB uses it in **embedded mode**: the database runs inside the app process via the seekdb-rs async **Client** (embedded).

## 2. Data directory

- **Default**: `{app data dir}/mine_kb.db/` (e.g. `~/Library/Application Support/com.mine-kb.app/mine_kb.db/` on macOS).
- **Override**: set `CONFIG_DIR` to the desired app data root; the DB path is then `$CONFIG_DIR/mine_kb.db/`.

No venv or Python install is required.

## 3. seekdb-rs usage in MineKB

- **Crate**: `seekdb-rs` (path dependency, feature `embedded` only).
- **Client**: async **Client** – built with `Client::builder().path(...).database(...).build().await`; all DB operations are async (no Python, no sync bridge).
- **Vector storage**: a single collection (e.g. `vector_documents`) holds chunk embeddings; projects are isolated by metadata filter (`project_id`).
- **Capabilities used**: parameterized SQL (`execute` / `fetch_all`), collection create/get, upsert, vector KNN, hybrid search (keyword + vector), delete by filter, count.

For vector and hybrid search details, see the [seekdb-rs](https://github.com/ob-labs/seekdb-rs) project. For MineKB’s adapter API, see `src-tauri/src/services/seekdb_adapter.rs`.

## 4. Switching to distributed SeekDB (optional)

If you later move to a distributed SeekDB/OceanBase server, you would switch the client to the server mode (e.g. MySQL-compatible connection). Application logic above the adapter can remain the same.
