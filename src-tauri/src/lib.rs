// Library crate for mine-kb
// This allows tests to import modules from the main application

#![allow(dead_code)]

/// 应用数据目录路径（供前端 get_app_data_dir 使用，保证 temp 等与后端一致）
pub struct AppDataDirPath(pub String);

pub mod commands;
pub mod config;
pub mod models;
pub mod services;
pub mod utils;
pub mod app_state_wrapper;
