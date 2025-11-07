# Quickstart Guide: Local Knowledge Base Management Desktop Application

**Date**: September 30, 2025
**Feature**: Local Knowledge Base Management Desktop Application
**Branch**: 001-panel-panel-panel

## Overview

This quickstart guide provides step-by-step instructions for setting up, building, and testing the knowledge base management desktop application. It covers both development setup and end-user workflows.

## Prerequisites

### Development Environment
- **Rust**: Latest stable version (1.70+)
- **Node.js**: Version 18+ with npm/yarn
- **Python**: Version 3.8+ (for Chroma vector database)
- **Git**: For version control
- **Code Editor**: VS Code recommended with Rust and TypeScript extensions

### System Requirements
- **OS**: Windows 10+, macOS 10.15+, or Linux (Ubuntu 20.04+)
- **RAM**: Minimum 4GB, recommended 8GB+
- **Storage**: 2GB free space for development, 500MB for application data
- **Network**: Internet connection for LLM API calls and initial setup

## Development Setup

### 1. Clone and Initialize Project

```bash
# Clone the repository
git clone <repository-url>
cd mine-kb

# Install Rust dependencies
cd src-tauri
cargo check

# Install frontend dependencies
cd ..
npm install

# Install Python dependencies for Chroma
pip install chromadb
```

### 2. Environment Configuration

Create `.env` file in project root:
```env
# LLM Configuration
OPENAI_API_KEY=your_openai_api_key_here
OPENAI_MODEL=gpt-4
OPENAI_BASE_URL=https://api.openai.com/v1

# Application Settings
APP_DATA_DIR=~/mine-kb-data
LOG_LEVEL=info
MAX_FILE_SIZE_MB=50
MAX_DOCUMENTS_PER_PROJECT=1000
```

Create `src-tauri/tauri.conf.json` configuration:
```json
{
  "build": {
    "beforeBuildCommand": "npm run build",
    "beforeDevCommand": "npm run dev",
    "devPath": "http://localhost:1420",
    "distDir": "../dist"
  },
  "package": {
    "productName": "Mine KB",
    "version": "0.1.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "fs": {
        "all": true,
        "scope": ["$APPDATA", "$APPDATA/**"]
      },
      "dialog": {
        "all": true
      },
      "shell": {
        "all": false,
        "open": true
      }
    }
  }
}
```

### 3. Start Development Environment

```bash
# Terminal 1: Start Chroma server
python -m chromadb.server --host localhost --port 8000

# Terminal 2: Start development server
npm run tauri dev
```

## Project Structure Validation

Verify the following directory structure exists:

```
mine-kb/
├── src-tauri/                 # Rust backend
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/
│   │   ├── models/
│   │   └── services/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                       # React frontend
│   ├── components/
│   ├── hooks/
│   ├── services/
│   ├── App.tsx
│   └── main.tsx
├── tests/
├── package.json
└── .env
```

## Core Functionality Testing

### Test 1: Application Launch
**Objective**: Verify the application starts successfully

**Steps**:
1. Run `npm run tauri dev`
2. Application window should open
3. Verify two-panel layout is visible
4. Check that left panel shows "No projects" message
5. Check that right panel shows "Select a project" message

**Expected Result**: Clean UI with proper layout and placeholder messages

### Test 2: Project Creation
**Objective**: Test basic project creation workflow

**Steps**:
1. Click "Create Project" button in left panel
2. Modal dialog should appear
3. Enter project name: "Test Project"
4. Select test files (create sample .txt and .md files)
5. Click "Create" button
6. Verify project appears in left panel
7. Check project status shows "Processing" then "Ready"

**Expected Result**: Project created successfully and visible in project list

### Test 3: Document Processing
**Objective**: Verify document upload and processing

**Test Files** (create these for testing):
- `test-doc.txt`: "This is a test document for the knowledge base."
- `test-doc.md`: "# Test Markdown\n\nThis is a markdown test file."

**Steps**:
1. Create project with test files
2. Wait for processing to complete
3. Click on project in left panel
4. Verify document count is correct
5. Check that project status is "Ready"

**Expected Result**: Documents processed and indexed successfully

### Test 4: Chat Functionality
**Objective**: Test conversation capabilities

**Steps**:
1. Select a project with processed documents
2. Right panel should show chat interface
3. Type query: "What is in the test document?"
4. Press Enter or click Send
5. Verify streaming response appears
6. Check that response references document content

**Expected Result**: AI response generated with relevant context from documents

### Test 5: Vector Search
**Objective**: Verify semantic search functionality

**Steps**:
1. Create project with multiple diverse documents
2. Ask question related to specific document content
3. Verify response includes relevant context
4. Ask follow-up question
5. Check conversation history is maintained

**Expected Result**: Accurate search results and coherent conversation flow

## User Workflow Validation

### Scenario 1: New User Onboarding
**User Story**: First-time user wants to create their first knowledge base

**Workflow**:
1. Launch application
2. See welcome screen with "Create Project" option
3. Click "Create Project"
4. Fill in project details and upload documents
5. Wait for processing
6. Start first conversation

**Success Criteria**:
- Intuitive UI flow
- Clear progress indicators
- Helpful error messages
- Successful document processing

### Scenario 2: Daily Usage
**User Story**: Regular user managing multiple projects

**Workflow**:
1. Launch application
2. See list of existing projects
3. Select recent project
4. Continue previous conversation or start new one
5. Ask questions and get relevant answers
6. Switch between projects as needed

**Success Criteria**:
- Fast application startup
- Quick project switching
- Conversation history preserved
- Responsive search and chat

### Scenario 3: Document Management
**User Story**: User wants to add documents to existing project

**Workflow**:
1. Select existing project
2. Add new documents via upload
3. Wait for processing
4. Verify new content is searchable
5. Test queries against new content

**Success Criteria**:
- Easy document addition
- Incremental processing
- Updated search results
- No data loss

## Performance Benchmarks

### Response Time Targets
- Application startup: < 3 seconds
- Project creation: < 5 seconds
- Document processing: < 2 seconds per MB
- Chat response initiation: < 500ms
- Vector search: < 200ms

### Memory Usage Targets
- Base application: < 100MB
- Per project loaded: < 50MB
- During document processing: < 200MB peak

### Storage Efficiency
- Vector embeddings: ~1KB per text chunk
- Metadata overhead: < 10% of document size
- Index size: < 20% of total content size

## Troubleshooting

### Common Issues

**Issue**: Application won't start
**Solution**:
1. Check Rust and Node.js versions
2. Verify all dependencies installed
3. Check for port conflicts (1420, 8000)

**Issue**: Document processing fails
**Solution**:
1. Verify file format is supported
2. Check file size limits
3. Ensure Chroma server is running
4. Check API key configuration

**Issue**: Chat responses are slow
**Solution**:
1. Check internet connection
2. Verify LLM API key and quotas
3. Monitor vector search performance
4. Check system resource usage

**Issue**: Vector search returns irrelevant results
**Solution**:
1. Verify document processing completed
2. Check chunk size configuration
3. Test with different query phrasing
4. Validate embedding model consistency

## Deployment Testing

### Build Verification
```bash
# Create production build
npm run tauri build

# Test built application
# On macOS: open src-tauri/target/release/bundle/macos/Mine\ KB.app
# On Windows: run src-tauri/target/release/mine-kb.exe
# On Linux: run src-tauri/target/release/mine-kb
```

### Cross-Platform Testing
- Test on Windows 10/11
- Test on macOS 10.15+
- Test on Ubuntu 20.04+
- Verify file system permissions
- Check native integrations

## Success Criteria

The quickstart is successful when:

1. **Development Environment**: All dependencies installed and working
2. **Application Launch**: App starts without errors in under 3 seconds
3. **Core Features**: Project creation, document upload, and chat all functional
4. **Performance**: Meets response time and memory usage targets
5. **User Experience**: Intuitive workflow with clear feedback
6. **Cross-Platform**: Works consistently across target operating systems

## Next Steps

After successful quickstart:

1. **Feature Development**: Implement additional features per tasks.md
2. **Testing**: Run comprehensive test suite
3. **Documentation**: Update user documentation
4. **Performance Optimization**: Profile and optimize bottlenecks
5. **Security Review**: Validate security measures
6. **User Testing**: Conduct usability testing with real users

This quickstart guide ensures a smooth development setup and validates core functionality before proceeding with full implementation.
