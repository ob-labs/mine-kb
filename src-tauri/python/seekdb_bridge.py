#!/usr/bin/env python3
"""
SeekDB Bridge - Python subprocess that handles database operations via JSON protocol
Communicates with Rust via stdin/stdout using newline-delimited JSON
"""

import sys
import json
import traceback
import os
from typing import Any, Dict, List, Optional
from datetime import datetime, date
from decimal import Decimal

# 尝试导入 seekdb，如果失败则提供详细的错误信息
try:
    import seekdb
except ImportError as e:
    print(f"[SeekDB Bridge] ❌ 无法导入 seekdb 模块", file=sys.stderr)
    print(f"[SeekDB Bridge] 错误详情: {e}", file=sys.stderr)
    print(f"[SeekDB Bridge] ", file=sys.stderr)
    print(f"[SeekDB Bridge] 诊断信息:", file=sys.stderr)
    print(f"[SeekDB Bridge] - Python 版本: {sys.version}", file=sys.stderr)
    print(f"[SeekDB Bridge] - Python 路径: {sys.executable}", file=sys.stderr)
    print(f"[SeekDB Bridge] - PYTHONPATH: {os.environ.get('PYTHONPATH', '(未设置)')}", file=sys.stderr)
    print(f"[SeekDB Bridge] - sys.path: {sys.path}", file=sys.stderr)
    print(f"[SeekDB Bridge] ", file=sys.stderr)
    print(f"[SeekDB Bridge] 解决方法:", file=sys.stderr)
    print(f"[SeekDB Bridge] 1. 确保 seekdb 包已安装", file=sys.stderr)
    print(f"[SeekDB Bridge] 2. 通过 pip 安装: python -m pip install seekdb==0.0.1.dev4 -i https://pypi.tuna.tsinghua.edu.cn/simple", file=sys.stderr)
    print(f"[SeekDB Bridge] 3. 检查虚拟环境是否正确激活", file=sys.stderr)
    sys.exit(1)
except Exception as e:
    print(f"[SeekDB Bridge] ❌ 加载 seekdb 模块时发生未知错误", file=sys.stderr)
    print(f"[SeekDB Bridge] 错误详情: {e}", file=sys.stderr)
    print(f"[SeekDB Bridge] Traceback:", file=sys.stderr)
    traceback.print_exc(file=sys.stderr)
    sys.exit(1)

class SeekDBBridge:
    def __init__(self):
        self.conn = None
        self.cursor = None
        self.db_path = None
        self.db_name = None
        
    def log(self, msg: str):
        """Log to stderr (stdout is reserved for responses)"""
        print(f"[SeekDB Bridge] {msg}", file=sys.stderr, flush=True)
    
    def convert_value_for_json(self, value: Any) -> Any:
        """Convert Python objects to JSON-serializable format"""
        if value is None:
            return None
        elif isinstance(value, (datetime, date)):
            # Convert datetime/date to ISO format string
            return value.isoformat()
        elif isinstance(value, Decimal):
            # Convert Decimal to float
            return float(value)
        elif isinstance(value, bytes):
            # Convert bytes to base64 string
            import base64
            return base64.b64encode(value).decode('utf-8')
        elif isinstance(value, (list, tuple)):
            # Recursively convert list/tuple items
            return [self.convert_value_for_json(v) for v in value]
        elif isinstance(value, dict):
            # Recursively convert dict values
            return {k: self.convert_value_for_json(v) for k, v in value.items()}
        else:
            # Return as-is for basic types (str, int, float, bool)
            return value
    
    def format_sql_value(self, value: Any) -> str:
        """Format a Python value to SQL string representation for ObLite"""
        if value is None:
            return "NULL"
        elif isinstance(value, bool):
            return "1" if value else "0"
        elif isinstance(value, (int, float)):
            return str(value)
        elif isinstance(value, str):
            # Escape single quotes in strings
            escaped = value.replace("'", "''")
            return f"'{escaped}'"
        elif isinstance(value, list):
            # For vector/array values
            return str(value)
        else:
            # For other types, convert to string and quote
            escaped = str(value).replace("'", "''")
            return f"'{escaped}'"
    
    def build_sql_with_values(self, sql: str, values: List[Any]) -> str:
        """
        Replace ? placeholders in SQL with actual values
        ObLite doesn't support parameterized queries, so we embed values directly
        """
        if not values:
            return sql
        
        # Replace ? with actual values
        result = sql
        for value in values:
            formatted_value = self.format_sql_value(value)
            # Replace the first occurrence of ?
            result = result.replace("?", formatted_value, 1)
        
        return result
    
    def send_response(self, response: Dict[str, Any]):
        """Send JSON response to stdout"""
        json.dump(response, sys.stdout)
        sys.stdout.write('\n')
        sys.stdout.flush()
    
    def send_success(self, data: Any = None):
        """Send success response"""
        self.send_response({"status": "success", "data": data})
    
    def send_error(self, error: str, details: str = ""):
        """Send error response"""
        self.send_response({
            "status": "error",
            "error": error,
            "details": details
        })
    
    def handle_init(self, params: Dict[str, Any]):
        """Initialize SeekDB connection"""
        try:
            db_path = params.get("db_path", "./seekdb.db")
            db_name = params.get("db_name", "mine_kb")
            
            self.log(f"Initializing SeekDB: path={db_path}, db={db_name}")
            
            # Open database instance
            seekdb.open(db_path)
            
            # Always ensure database exists before connecting
            # Note: In seekdb 0.0.1.dev4, connect() will validate database existence
            try:
                self.log(f"Ensuring database '{db_name}' exists...")
                # Connect to default "test" database to create new database
                # SeekDB 0.0.1.dev4: connects to "test" by default when unspecified
                admin_conn = seekdb.connect("test")
                admin_cursor = admin_conn.cursor()
                admin_cursor.execute(f"CREATE DATABASE IF NOT EXISTS `{db_name}`")
                admin_conn.commit()
                admin_conn.close()
                self.log(f"✅ Database '{db_name}' created successfully")
            except Exception as create_error:
                self.log(f"❌ Error: Failed to create database: {create_error}")
                self.log(f"Traceback: {traceback.format_exc()}")
                # If database creation fails, raise exception to prevent connecting to non-existent database
                raise Exception(f"Cannot create database '{db_name}': {create_error}")
            
            # Now connect to the database
            self.conn = seekdb.connect(db_name)
            self.log(f"✅ Connected to database '{db_name}'")
            
            self.cursor = self.conn.cursor()
            self.db_path = db_path
            self.db_name = db_name
            
            # Ensure we're using the correct database
            try:
                self.cursor.execute(f"USE `{db_name}`")
                self.log(f"Switched to database '{db_name}'")
            except Exception as use_error:
                self.log(f"Warning: Failed to execute USE {db_name}: {use_error}")
                # This might not be supported, continue anyway
            
            self.log("SeekDB initialized successfully")
            self.send_success({"db_path": db_path, "db_name": db_name})
            
        except Exception as e:
            self.log(f"Init error: {e}")
            self.log(f"Traceback: {traceback.format_exc()}")
            error_details = (
                f"数据库初始化失败\n"
                f"路径: {params.get('db_path', './oblite.db')}\n"
                f"数据库名: {params.get('db_name', 'mine_kb')}\n"
                f"错误: {str(e)}"
            )
            self.send_error("InitError", error_details)
    
    def handle_execute(self, params: Dict[str, Any]):
        """Execute SQL statement (INSERT, UPDATE, DELETE, CREATE, etc.)"""
        try:
            sql = params["sql"]
            values = params.get("values", [])
            
            # ObLite doesn't support parameterized queries, embed values directly
            final_sql = self.build_sql_with_values(sql, values)
            
            self.log(f"Executing: {final_sql[:200]}...")
            
            # ObLite execute() only accepts one argument
            self.cursor.execute(final_sql)
            
            rows_affected = self.cursor.rowcount if hasattr(self.cursor, 'rowcount') else 0
            self.send_success({"rows_affected": rows_affected})
            
        except Exception as e:
            self.log(f"Execute error: {e}")
            self.send_error("ExecuteError", str(e))
    
    def handle_query(self, params: Dict[str, Any]):
        """Execute SELECT query and return results"""
        try:
            sql = params["sql"]
            values = params.get("values", [])
            
            # ObLite doesn't support parameterized queries, embed values directly
            final_sql = self.build_sql_with_values(sql, values)
            
            self.log(f"Querying: {final_sql[:200]}...")
            
            # ObLite execute() only accepts one argument
            self.cursor.execute(final_sql)
            
            rows = self.cursor.fetchall()
            
            # Convert rows to list of lists, handling datetime and other special types
            if rows:
                result = []
                for row in rows:
                    converted_row = [self.convert_value_for_json(val) for val in row]
                    result.append(converted_row)
            else:
                result = []
            
            self.log(f"Query returned {len(result)} rows")
            self.send_success({"rows": result})
            
        except Exception as e:
            self.log(f"Query error: {e}")
            self.log(f"Traceback: {traceback.format_exc()}")
            self.send_error("QueryError", str(e))
    
    def handle_query_one(self, params: Dict[str, Any]):
        """Execute SELECT query and return first row"""
        try:
            sql = params["sql"]
            values = params.get("values", [])
            
            # ObLite doesn't support parameterized queries, embed values directly
            final_sql = self.build_sql_with_values(sql, values)
            
            # ObLite execute() only accepts one argument
            self.cursor.execute(final_sql)
            
            row = self.cursor.fetchone()
            
            # Convert row values, handling datetime and other special types
            if row:
                result = [self.convert_value_for_json(val) for val in row]
            else:
                result = None
            
            self.send_success({"row": result})
            
        except Exception as e:
            self.log(f"Query one error: {e}")
            self.log(f"Traceback: {traceback.format_exc()}")
            self.send_error("QueryOneError", str(e))
    
    def handle_commit(self, params: Dict[str, Any]):
        """Commit current transaction"""
        try:
            self.log("Committing transaction")
            self.conn.commit()
            self.send_success()
            
        except Exception as e:
            self.log(f"Commit error: {e}")
            self.send_error("CommitError", str(e))
    
    def handle_rollback(self, params: Dict[str, Any]):
        """Rollback current transaction"""
        try:
            self.log("Rolling back transaction")
            self.conn.rollback()
            self.send_success()
            
        except Exception as e:
            self.log(f"Rollback error: {e}")
            self.send_error("RollbackError", str(e))
    
    def handle_ping(self, params: Dict[str, Any]):
        """Health check"""
        self.send_success({"message": "pong"})
    
    def handle_command(self, command: Dict[str, Any]):
        """Route command to appropriate handler"""
        cmd_type = command.get("command")
        params = command.get("params", {})
        
        handlers = {
            "init": self.handle_init,
            "execute": self.handle_execute,
            "query": self.handle_query,
            "query_one": self.handle_query_one,
            "commit": self.handle_commit,
            "rollback": self.handle_rollback,
            "ping": self.handle_ping,
        }
        
        handler = handlers.get(cmd_type)
        if handler:
            handler(params)
        else:
            self.send_error("UnknownCommand", f"Unknown command: {cmd_type}")
    
    def run(self):
        """Main loop - read commands from stdin and execute them"""
        self.log("SeekDB Bridge started, waiting for commands...")
        
        try:
            for line in sys.stdin:
                line = line.strip()
                if not line:
                    continue
                
                try:
                    command = json.loads(line)
                    self.handle_command(command)
                    
                except json.JSONDecodeError as e:
                    self.log(f"JSON decode error: {e}")
                    self.send_error("JSONError", str(e))
                    
                except Exception as e:
                    self.log(f"Unexpected error: {e}")
                    self.log(traceback.format_exc())
                    self.send_error("InternalError", str(e))
        
        except KeyboardInterrupt:
            self.log("Received interrupt signal, shutting down...")
        
        finally:
            if self.conn:
                try:
                    self.conn.close()
                    self.log("Database connection closed")
                except:
                    pass

if __name__ == "__main__":
    bridge = SeekDBBridge()
    bridge.run()

