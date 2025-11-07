# Tauri Commands API Contract

**Date**: September 30, 2025
**Feature**: Local Knowledge Base Management Desktop Application
**Branch**: 001-panel-panel-panel

## Project Management Commands

### create_project
Creates a new project with the specified name and processes uploaded files.

**Command**: `create_project`
**Input**:
```typescript
interface CreateProjectRequest {
  name: string;           // Project name (1-100 chars, unique)
  description?: string;   // Optional description (0-500 chars)
  file_paths: string[];   // Array of file paths to upload
}
```

**Output**:
```typescript
interface CreateProjectResponse {
  project: {
    id: string;           // UUID
    name: string;
    description?: string;
    status: "Created" | "Processing" | "Ready" | "Error";
    created_at: string;   // ISO 8601 timestamp
    document_count: number;
  };
}
```

**Errors**:
- `ProjectNameExists`: Project name already exists
- `InvalidFileName`: One or more files have invalid names
- `FileTooLarge`: File exceeds size limit (50MB)
- `UnsupportedFileType`: File type not supported
- `FileNotFound`: One or more files not found at specified paths

### get_projects
Retrieves list of all projects.

**Command**: `get_projects`
**Input**: None

**Output**:
```typescript
interface GetProjectsResponse {
  projects: Array<{
    id: string;
    name: string;
    description?: string;
    status: "Created" | "Processing" | "Ready" | "Error";
    created_at: string;
    updated_at: string;
    document_count: number;
  }>;
}
```

### get_project_details
Retrieves detailed information about a specific project.

**Command**: `get_project_details`
**Input**:
```typescript
interface GetProjectDetailsRequest {
  project_id: string;     // UUID
}
```

**Output**:
```typescript
interface GetProjectDetailsResponse {
  project: {
    id: string;
    name: string;
    description?: string;
    status: "Created" | "Processing" | "Ready" | "Error";
    created_at: string;
    updated_at: string;
    document_count: number;
    documents: Array<{
      id: string;
      filename: string;
      file_size: number;
      processing_status: "Uploaded" | "Processing" | "Indexed" | "Failed";
      created_at: string;
      error_message?: string;
    }>;
  };
}
```

**Errors**:
- `ProjectNotFound`: Project with specified ID does not exist

### delete_project
Deletes a project and all associated data.

**Command**: `delete_project`
**Input**:
```typescript
interface DeleteProjectRequest {
  project_id: string;     // UUID
}
```

**Output**:
```typescript
interface DeleteProjectResponse {
  success: boolean;
}
```

**Errors**:
- `ProjectNotFound`: Project with specified ID does not exist
- `ProjectInUse`: Project is currently being processed

## Document Management Commands

### upload_documents
Adds new documents to an existing project.

**Command**: `upload_documents`
**Input**:
```typescript
interface UploadDocumentsRequest {
  project_id: string;     // UUID
  file_paths: string[];   // Array of file paths to upload
}
```

**Output**:
```typescript
interface UploadDocumentsResponse {
  documents: Array<{
    id: string;
    filename: string;
    file_size: number;
    processing_status: "Uploaded" | "Processing";
    created_at: string;
  }>;
}
```

**Errors**:
- `ProjectNotFound`: Project with specified ID does not exist
- `FileTooLarge`: File exceeds size limit
- `UnsupportedFileType`: File type not supported
- `DuplicateFile`: File with same content hash already exists in project

### get_document_content
Retrieves the processed content of a document.

**Command**: `get_document_content`
**Input**:
```typescript
interface GetDocumentContentRequest {
  document_id: string;    // UUID
}
```

**Output**:
```typescript
interface GetDocumentContentResponse {
  document: {
    id: string;
    filename: string;
    content: string;      // Full text content
    chunks: Array<{
      id: string;
      content: string;
      chunk_index: number;
      token_count: number;
    }>;
  };
}
```

**Errors**:
- `DocumentNotFound`: Document with specified ID does not exist
- `DocumentNotProcessed`: Document processing not yet complete

## Chat/Conversation Commands

### create_conversation
Creates a new conversation within a project.

**Command**: `create_conversation`
**Input**:
```typescript
interface CreateConversationRequest {
  project_id: string;     // UUID
  title?: string;         // Optional conversation title
}
```

**Output**:
```typescript
interface CreateConversationResponse {
  conversation: {
    id: string;           // UUID
    project_id: string;
    title: string;        // Auto-generated if not provided
    created_at: string;
    message_count: number;
  };
}
```

**Errors**:
- `ProjectNotFound`: Project with specified ID does not exist
- `ProjectNotReady`: Project is not ready for conversations

### get_conversations
Retrieves all conversations for a project.

**Command**: `get_conversations`
**Input**:
```typescript
interface GetConversationsRequest {
  project_id: string;     // UUID
}
```

**Output**:
```typescript
interface GetConversationsResponse {
  conversations: Array<{
    id: string;
    title: string;
    created_at: string;
    updated_at: string;
    message_count: number;
  }>;
}
```

### send_message
Sends a message in a conversation and gets AI response.

**Command**: `send_message`
**Input**:
```typescript
interface SendMessageRequest {
  conversation_id: string;  // UUID
  content: string;          // User message content
}
```

**Output** (Streaming):
```typescript
interface SendMessageResponse {
  message_id: string;       // UUID of user message
  response_stream: AsyncIterator<{
    type: "token" | "context" | "complete";
    content?: string;       // For token type
    context_chunks?: Array<{
      document_id: string;
      filename: string;
      content: string;
      relevance_score: number;
    }>;                     // For context type
    response_id?: string;   // For complete type
  }>;
}
```

**Errors**:
- `ConversationNotFound`: Conversation with specified ID does not exist
- `EmptyMessage`: Message content cannot be empty
- `LLMServiceError`: Error communicating with LLM service
- `NoRelevantContext`: No relevant documents found for query

### get_conversation_history
Retrieves message history for a conversation.

**Command**: `get_conversation_history`
**Input**:
```typescript
interface GetConversationHistoryRequest {
  conversation_id: string;  // UUID
  limit?: number;           // Optional limit (default: 50)
  offset?: number;          // Optional offset for pagination
}
```

**Output**:
```typescript
interface GetConversationHistoryResponse {
  messages: Array<{
    id: string;
    role: "User" | "Assistant" | "System";
    content: string;
    timestamp: string;
    context_chunks?: Array<{
      document_id: string;
      filename: string;
      content: string;
    }>;
  }>;
  total_count: number;
}
```

## System Commands

### get_app_status
Retrieves application status and health information.

**Command**: `get_app_status`
**Input**: None

**Output**:
```typescript
interface GetAppStatusResponse {
  status: "Ready" | "Initializing" | "Error";
  version: string;
  database_status: "Connected" | "Disconnected" | "Error";
  vector_db_status: "Connected" | "Disconnected" | "Error";
  llm_service_status: "Connected" | "Disconnected" | "Error";
  storage_info: {
    total_projects: number;
    total_documents: number;
    storage_used_mb: number;
  };
}
```

### configure_llm_service
Configures the LLM service settings.

**Command**: `configure_llm_service`
**Input**:
```typescript
interface ConfigureLLMServiceRequest {
  provider: "OpenAI" | "Anthropic" | "Local";
  api_key?: string;         // Required for cloud providers
  model: string;            // e.g., "gpt-4", "claude-3-sonnet"
  base_url?: string;        // For local or custom endpoints
}
```

**Output**:
```typescript
interface ConfigureLLMServiceResponse {
  success: boolean;
  test_connection: boolean; // Whether test connection succeeded
}
```

**Errors**:
- `InvalidAPIKey`: API key is invalid or expired
- `UnsupportedModel`: Model not supported by provider
- `ConnectionFailed`: Unable to connect to service

## Error Handling

All commands follow consistent error handling patterns:

```typescript
interface TauriError {
  code: string;             // Error code for programmatic handling
  message: string;          // Human-readable error message
  details?: any;            // Additional error context
}
```

Common error codes:
- `ValidationError`: Input validation failed
- `NotFound`: Requested resource not found
- `PermissionDenied`: Operation not permitted
- `ServiceUnavailable`: External service unavailable
- `InternalError`: Unexpected internal error

## Event System

The application emits events for long-running operations:

### Processing Events
```typescript
// Document processing progress
interface DocumentProcessingEvent {
  type: "document_processing";
  project_id: string;
  document_id: string;
  status: "started" | "progress" | "completed" | "failed";
  progress?: number;        // 0-100 for progress events
  error?: string;           // Error message for failed events
}

// Project status changes
interface ProjectStatusEvent {
  type: "project_status";
  project_id: string;
  status: "Created" | "Processing" | "Ready" | "Error";
  message?: string;
}
```

These contracts define the complete API surface between the React frontend and Rust backend, ensuring type safety and clear communication patterns.
