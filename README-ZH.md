[![English](https://img.shields.io/badge/lang-English-blue)](README.md)
[![中文](https://img.shields.io/badge/lang-中文-red)](README-ZH.md)

<img src="https://mdn.alipayobjects.com/huamei_ytl0i7/afts/img/A*RgJoQp0N_VMAAAAAQGAAAAgAejCYAQ/original" width="64">

<sub>LOGO说明：电影、小说、动漫常借用“第三只眼”作为超能力或觉醒的象征（如《龙珠》中的比克、《沙丘》中的预知能力）</sub>


# MineKB - 个人知识库
<img src="https://mdn.alipayobjects.com/huamei_ytl0i7/afts/img/A*0gHyQKzD5AcAAAAAbcAAAAgAejCYAQ/original" width="800">

一个基于 SeekDB 数据库实现的本地知识库桌面应用程序。

## 项目介绍

MineKB 是一个基于 Tauri 构建的跨平台桌面应用，旨在帮助用户高效管理和查询本地文档知识库。

### 核心功能

- **项目管理**：创建多个知识库项目，每个项目可以包含多个文档
- **文档处理**：支持上传多种格式的文档（PDF、DOCX 等），自动进行向量化处理
- **批量导入**：支持从目录批量导入文档，带智能预检查和自动重试机制
- **智能对话**：通过自然语言与知识库对话，AI 会基于文档内容生成精准回答
- **向量搜索**：利用语义搜索技术，快速定位相关文档内容
- **流式输出**：实时流式展示 AI 生成的回答，提供流畅的用户体验
- **语音交互**：支持语音输入功能，让知识查询更便捷
- **本地存储**：所有数据存储在本地嵌入式数据库中，保护隐私安

## 基本原理

MineKB 采用 RAG（Retrieval-Augmented Generation）架构，结合向量检索和大语言模型技术：

### 工作流程

```
1. 文档上传 → 2. 文本提取 → 3. 向量化处理 → 4. 存储到向量数据库
                                                          ↓
6. 流式输出回答 ← 5. LLM生成回答 ← 向量检索相关内容 ← 用户查询
```

### 技术实现

1. **文档处理**
   - 支持 PDF、DOCX 等格式文档的文本提取
   - 对提取的文本进行分块处理
   - 使用阿里云百炼 API 生成文档的向量表示（Embeddings）

2. **向量存储**
   - 使用 SeekDB 0.0.1.dev4 作为嵌入式向量数据库（通过 Python 子进程访问）
   - 原生支持向量类型和 HNSW 索引，实现高效的向量检索
   - 支持项目级别的数据隔离和事务处理
   - 支持向量列输出和数据库存在性验证

3. **智能对话**
   - 用户输入查询后，系统将查询转换为向量
   - 在当前项目的向量数据库中进行相似度搜索
   - 将检索到的相关文档片段作为上下文，结合用户查询发送给 LLM
   - LLM 基于上下文生成准确的回答并流式返回

4. **语音识别**
   - 集成语音识别服务，支持语音输入查询
   - 自动将语音转换为文本进行处理

## 🛠️ 技术栈介绍

### 前端技术栈

**核心框架**
- **React 18.2.0** - 现代化的 UI 框架
- **TypeScript 5.2.2** - 类型安全的开发体验
- **Vite 5.0.8** - 快速的构建工具和开发服务器

**样式系统**
- **Tailwind CSS 3.3.6** - 实用优先的 CSS 框架
- **@tailwindcss/typography 0.5** - Markdown 文档排版
- **class-variance-authority 0.7** - 组件样式变体管理
- **clsx 2.0 / tailwind-merge 2.0** - CSS 类名合并工具

**UI 组件库**
- **Radix UI** - 无障碍的组件基础库
  - `@radix-ui/react-dialog 1.1.15` - 对话框组件
  - `@radix-ui/react-alert-dialog 1.1.15` - 警告对话框
  - `@radix-ui/react-tabs 1.1.13` - 标签页组件
  - `@radix-ui/react-dropdown-menu 2.1.16` - 下拉菜单
  - `@radix-ui/react-slot 1.2.3` - 插槽组件
- **Lucide React 0.294** - 精美的图标库

**内容渲染**
- **React Markdown 10.1** - Markdown 渲染
- **React Syntax Highlighter 15.6** - 代码语法高亮
- **remark-gfm 4.0** - GitHub Flavored Markdown 支持

**开发与测试**
- **Vitest 1.0** - 快速的单元测试框架
- **@testing-library/react 16.3** - React 组件测试
- **@testing-library/jest-dom 6.8** - DOM 断言库
- **@testing-library/user-event 14.6** - 用户事件模拟
- **ESLint 8.55** - 代码质量检查
- **TypeScript ESLint 6.14** - TypeScript 代码规范

### 后端技术栈

**核心技术**
- **Rust (Edition 2021)** - 高性能的系统编程语言
- **Tauri 1.5** - 轻量级跨平台桌面应用框架
  - `@tauri-apps/api 1.5` - 前端 API 调用库
  - `@tauri-apps/cli 1.5` - 命令行工具
  - 启用功能：`path-all`、`http-all`、`dialog-all`、`fs-all`、`shell-open`
- **Python 3.8+** - SeekDB 数据库操作（通过子进程通信）

**数据库**
- **SeekDB 0.0.1.dev4** (Python) - AI-Native 嵌入式向量数据库
  - 原生支持向量类型和 HNSW 索引
  - 支持混合检索和全文搜索
  - 高性能向量相似度计算
  - 通过 JSON-RPC 协议与 Rust 通信

### Rust 核心依赖

**文档处理**
- `pdf-extract 0.7` - PDF 文本提取
- `docx-rs 0.4` - Word 文档处理

**数据存储**
- `seekdb 0.0.1.dev4` (Python) - AI-Native 嵌入式数据库，原生支持向量索引和 HNSW 检索
- JSON 通信协议 - Rust 与 Python 子进程通信

**向量计算**
- SeekDB 原生向量索引 (HNSW) - 高效向量相似度搜索
- Rust 标准库 - 余弦相似度等数学计算（无需第三方依赖）

**网络通信**
- `reqwest 0.11` - HTTP 客户端，用于调用 AI API（支持 json、stream、blocking）

**序列化与编解码**
- `serde 1.0` / `serde_json 1.0` - JSON 序列化与反序列化
- `bincode 1.3` - 向量数据二进制序列化
- `base64 0.22.1` - Base64 编解码（语音数据传输）

**语音识别**
- `hmac 0.12` - HMAC 签名算法
- `sha1 0.10` / `sha2 0.10` - SHA 哈希算法
- `url 2.4` / `urlencoding 2.1` - URL 编码处理

**系统工具**
- `chrono 0.4` - 日期时间处理（支持 serde）
- `uuid 1.0` - 唯一标识符生成（v4 + serde）
- `anyhow 1.0` / `thiserror 1.0` - 错误处理
- `regex 1.0` - 正则表达式
- `walkdir 2.0` - 文件系统遍历

**异步编程**
- `tokio 1.x` - 异步运行时（full features）
- `futures 0.3` - Future 组合器和工具
- `async-stream 0.3` - 异步流宏（用于流式输出）

**日志系统**
- `log 0.4` - 日志门面
- `env_logger 0.10` - 环境变量日志实现


### AI 服务

- **阿里云百炼 API** - 用于文本 Embedding 和大语言模型对话

## 系统架构

### 架构概览
<img src="https://mdn.alipayobjects.com/huamei_ytl0i7/afts/img/A*Cuf4RoPSfwMAAAAAT-AAAAgAejCYAQ/original">

## 快速开始

### 环境要求

- Node.js 16+
- Rust 1.70+
- Python 3.8+

> **注意**: SeekDB 目前仅发布 Linux 版本，不久会支持 MacOS。MacOS 用户推荐使用 [UTM](https://mac.getutm.app) 虚拟机管理器运行 [Ubuntu 20.x 以上](https://mac.getutm.app/gallery/ubuntu-20-04)。

### 安装依赖

```bash
# 安装前端依赖
npm install

# Rust 和 Python 依赖会在构建时自动安装
```

### 配置

1. 复制配置文件模板：
```bash
cp src-tauri/config.example.json src-tauri/config.json
```

2. 编辑 `src-tauri/config.json`，填入阿里云百炼 API 密钥等配置信息

### 开发模式

```bash
# 启动开发服务器
tnpm run tauri:dev
```

### 构建应用

```bash
# 构建生产版本
tnpm run tauri:build
```

构建完成后，应用程序将位于 `src-tauri/target/release/bundle/` 目录下。

---

## 测试

```bash
# 运行前端测试
tnpm test

# 运行测试 UI
tnpm run test:ui

# 运行 Rust 测试
cd src-tauri && cargo test
```


### 为什么选择 SeekDB？

- ✅ **原生向量支持**：无需序列化/反序列化，性能提升 10-100x
- ✅ **HNSW 索引**：专业的向量索引算法，检索更快更准
- ✅ **AI-Native 特性**：内置全文检索、混合检索等 AI 能力
- ✅ **更好的扩展性**：支持更大规模的数据和更复杂的查询
- ✅ **最新版本特性**（0.0.1.dev4）：向量列输出、数据库验证、USE 语句稳定支持

---

**MineKB** - 让知识管理更智能 🚀🚀🚀
