use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Request sent to Python subprocess
#[derive(Debug, Serialize)]
struct Request {
    command: String,
    params: Value,
}

/// Response from Python subprocess
#[derive(Debug, Deserialize)]
struct Response {
    status: String,
    #[serde(default)]
    data: Option<Value>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    details: Option<String>,
}

/// Python subprocess manager for SeekDB operations
#[derive(Debug)]
pub struct PythonSubprocess {
    child: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    stdout: Arc<Mutex<Option<BufReader<ChildStdout>>>>,
    script_path: String,
    python_executable: String,
}

impl PythonSubprocess {
    /// Create and start a new Python subprocess
    pub fn new(script_path: &str) -> Result<Self> {
        Self::new_with_python(script_path, "python3")
    }
    
    /// Create and start a new Python subprocess with custom Python executable
    pub fn new_with_python(script_path: &str, python_executable: &str) -> Result<Self> {
        log::info!("🐍 Starting Python subprocess: {}", script_path);
        log::info!("   Python 可执行文件: {}", python_executable);
        
        let mut command = Command::new(python_executable);
        command
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()); // Log stderr to our log
        
        let mut child = command
            .spawn()
            .map_err(|e| anyhow!("Failed to start Python process: {}", e))?;
        
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to open stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to open stdout"))?;
        let stdout = BufReader::new(stdout);
        
        log::info!("✅ Python subprocess started successfully");
        
        Ok(Self {
            child: Arc::new(Mutex::new(Some(child))),
            stdin: Arc::new(Mutex::new(Some(stdin))),
            stdout: Arc::new(Mutex::new(Some(stdout))),
            script_path: script_path.to_string(),
            python_executable: python_executable.to_string(),
        })
    }
    
    /// Send a command and wait for response
    pub fn send_command(&self, command: &str, params: Value) -> Result<Value> {
        let request = Request {
            command: command.to_string(),
            params: params.clone(),
        };
        
        // Serialize request to JSON
        let request_json = serde_json::to_string(&request)?;
        
        log::debug!("📤 Sending command: {} (params: {})", command, 
            serde_json::to_string(&params).unwrap_or_default());
        
        // Write to stdin
        {
            let mut stdin_guard = self.stdin.lock().unwrap();
            let stdin = stdin_guard.as_mut().ok_or_else(|| anyhow!("Stdin not available"))?;
            
            writeln!(stdin, "{}", request_json)?;
            stdin.flush()?;
        }
        
        // Read response from stdout
        let response_line = {
            let mut stdout_guard = self.stdout.lock().unwrap();
            let stdout = stdout_guard.as_mut().ok_or_else(|| anyhow!("Stdout not available"))?;
            
            let mut line = String::new();
            stdout.read_line(&mut line)?;
            line
        };
        
        log::debug!("📥 Received response: {}", response_line.trim());
        
        // Parse response
        let response: Response = serde_json::from_str(&response_line)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;
        
        // Check response status
        if response.status == "success" {
            Ok(response.data.unwrap_or(Value::Null))
        } else {
            let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
            let details = response.details.unwrap_or_default();
            Err(anyhow!("Python subprocess error: {} - {}", error_msg, details))
        }
    }
    
    /// Initialize SeekDB database
    pub fn init_db(&self, db_path: &str, db_name: &str) -> Result<()> {
        log::info!("🔧 Initializing SeekDB: path={}, name={}", db_path, db_name);
        
        let params = serde_json::json!({
            "db_path": db_path,
            "db_name": db_name
        });
        
        self.send_command("init", params)?;
        log::info!("✅ SeekDB initialized");
        Ok(())
    }
    
    /// Execute SQL statement (INSERT, UPDATE, DELETE, CREATE, etc.)
    pub fn execute(&self, sql: &str, values: Vec<Value>) -> Result<i64> {
        let params = serde_json::json!({
            "sql": sql,
            "values": values
        });
        
        let response = self.send_command("execute", params)?;
        let rows_affected = response
            .get("rows_affected")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        
        Ok(rows_affected)
    }
    
    /// Execute SELECT query and return all rows
    pub fn query(&self, sql: &str, values: Vec<Value>) -> Result<Vec<Vec<Value>>> {
        let params = serde_json::json!({
            "sql": sql,
            "values": values
        });
        
        let response = self.send_command("query", params)?;
        let rows = response
            .get("rows")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Invalid query response"))?;
        
        let result: Vec<Vec<Value>> = rows
            .iter()
            .filter_map(|row| row.as_array().map(|r| r.clone()))
            .collect();
        
        Ok(result)
    }
    
    /// Execute SELECT query and return first row
    pub fn query_one(&self, sql: &str, values: Vec<Value>) -> Result<Option<Vec<Value>>> {
        let params = serde_json::json!({
            "sql": sql,
            "values": values
        });
        
        let response = self.send_command("query_one", params)?;
        let row = response.get("row");
        
        match row {
            Some(Value::Array(arr)) => Ok(Some(arr.clone())),
            Some(Value::Null) | None => Ok(None),
            _ => Err(anyhow!("Invalid query_one response")),
        }
    }
    
    /// Commit current transaction
    pub fn commit(&self) -> Result<()> {
        self.send_command("commit", Value::Null)?;
        Ok(())
    }
    
    /// Rollback current transaction
    pub fn rollback(&self) -> Result<()> {
        self.send_command("rollback", Value::Null)?;
        Ok(())
    }
    
    /// Ping to check if subprocess is alive
    pub fn ping(&self) -> Result<()> {
        self.send_command("ping", Value::Null)?;
        Ok(())
    }
    
    /// Check if subprocess is still running
    pub fn is_alive(&self) -> bool {
        let child_guard = self.child.lock().unwrap();
        if let Some(_child) = child_guard.as_ref() {
            // Try to ping
            drop(child_guard);
            self.ping().is_ok()
        } else {
            false
        }
    }
    
    /// Restart the subprocess if it has died
    pub fn restart_if_needed(&mut self) -> Result<()> {
        if !self.is_alive() {
            log::warn!("⚠️ Python subprocess is not responding, restarting...");
            self.shutdown();
            
            // Create new subprocess with same configuration
            let python_executable = self.python_executable.clone();
            let new_subprocess = Self::new_with_python(&self.script_path, &python_executable)?;
            *self = new_subprocess;
            
            log::info!("✅ Python subprocess restarted");
        }
        Ok(())
    }
    
    /// Gracefully shutdown the subprocess
    pub fn shutdown(&mut self) {
        log::info!("🛑 Shutting down Python subprocess...");
        
        // Close stdin to signal subprocess to exit
        {
            let mut stdin_guard = self.stdin.lock().unwrap();
            *stdin_guard = None;
        }
        
        // Wait for child process to exit (with timeout)
        {
            let mut child_guard = self.child.lock().unwrap();
            if let Some(mut child) = child_guard.take() {
                thread::sleep(Duration::from_millis(500));
                
                match child.try_wait() {
                    Ok(Some(status)) => {
                        log::info!("Python subprocess exited with status: {}", status);
                    }
                    Ok(None) => {
                        log::warn!("Python subprocess still running, killing...");
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                    Err(e) => {
                        log::error!("Error waiting for subprocess: {}", e);
                    }
                }
            }
        }
        
        log::info!("✅ Python subprocess shutdown complete");
    }
}

impl Drop for PythonSubprocess {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_subprocess_creation() {
        // This test would require the actual Python script to exist
        // Skipping in unit tests, should be tested in integration tests
    }
}

