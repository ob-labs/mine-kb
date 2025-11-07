// Test library for Rust contract and integration tests
// This file allows tests to access the main application modules

// Re-export modules from the main application for testing
pub use mine_kb::commands;
pub use mine_kb::models;
pub use mine_kb::services;

// Test modules
pub mod contract;
pub mod integration;
pub mod unit;
pub mod performance;
