# Data Model: Local Knowledge Base Management Desktop Application

**Date**: September 30, 2025
**Feature**: Local Knowledge Base Management Desktop Application
**Branch**: 001-panel-panel-panel

## Entity Definitions

### Project
Represents a collection of documents with associated conversation history and vector embeddings.

**Attributes**:
- `id`: UUID - Unique identifier for the project
- `name`: String - User-defined project name (required, 1-100 characters)
- `description`: String - Optional project description (0-500 characters)
- `created_at`: DateTime - Project creation timestamp
- `updated_at`: DateTime - Last modification timestamp
- `document_count`: Integer - Number of documents in the project
- `status`: Enum - Project processing status (Created, Processing, Ready, Error)

**Validation Rules**:
- Name must be unique across all projects
- Name cannot be empty or contain only whitespace
- Description is optional but limited to 500 characters
- Status transitions: Created → Processing → Ready/Error

**Relationships**:
- One-to-many with Document entities
- One-to-many with Conversation entities

### Document
Represents an uploaded file that has been processed into searchable chunks.

**Attributes**:
- `id`: UUID - Unique identifier for the document
- `project_id`: UUID - Foreign key to parent project
- `filename`: String - Original filename with extension
- `file_path`: String - Local file system path
- `file_size`: Integer - File size in bytes
- `mime_type`: String - MIME type of the file
- `content_hash`: String - SHA-256 hash of file content for deduplication
- `chunk_count`: Integer - Number of text chunks created
- `processing_status`: Enum - Processing state (Uploaded, Processing, Indexed, Failed)
- `error_message`: String - Error details if processing failed
- `created_at`: DateTime - Upload timestamp
- `processed_at`: DateTime - Processing completion timestamp

**Validation Rules**:
- Filename must be valid for the file system
- File size must be within limits (max 50MB per file)
- Supported MIME types: text/plain, text/markdown, application/pdf
- Content hash must be unique within project (prevent duplicates)

**Relationships**:
- Many-to-one with Project entity
- One-to-many with DocumentChunk entities

### DocumentChunk
Represents a processed text segment from a document, stored as vector embeddings.

**Attributes**:
- `id`: UUID - Unique identifier for the chunk
- `document_id`: UUID - Foreign key to parent document
- `chunk_index`: Integer - Sequential position within document
- `content`: String - Original text content of the chunk
- `token_count`: Integer - Number of tokens in the chunk
- `start_offset`: Integer - Character offset in original document
- `end_offset`: Integer - End character offset in original document
- `embedding_id`: String - Reference to vector embedding in Chroma
- `created_at`: DateTime - Chunk creation timestamp

**Validation Rules**:
- Content cannot be empty
- Token count should be between 100-1000 tokens
- Chunk index must be sequential within document
- Offsets must be valid for the source document

**Relationships**:
- Many-to-one with Document entity
- Referenced by vector embeddings in Chroma database

### Conversation
Represents a chat session within a project context.

**Attributes**:
- `id`: UUID - Unique identifier for the conversation
- `project_id`: UUID - Foreign key to parent project
- `title`: String - Auto-generated or user-defined conversation title
- `created_at`: DateTime - Conversation start timestamp
- `updated_at`: DateTime - Last message timestamp
- `message_count`: Integer - Number of messages in conversation

**Validation Rules**:
- Title limited to 200 characters
- Must belong to a valid project
- Message count must match actual message records

**Relationships**:
- Many-to-one with Project entity
- One-to-many with Message entities

### Message
Represents individual messages within a conversation.

**Attributes**:
- `id`: UUID - Unique identifier for the message
- `conversation_id`: UUID - Foreign key to parent conversation
- `role`: Enum - Message sender (User, Assistant, System)
- `content`: String - Message text content
- `timestamp`: DateTime - Message creation time
- `token_count`: Integer - Number of tokens in message
- `context_chunks`: Array<UUID> - Referenced document chunks for context
- `processing_time`: Float - Time taken to generate response (for Assistant messages)

**Validation Rules**:
- Content cannot be empty for User and Assistant messages
- Role must be valid enum value
- Context chunks must reference valid DocumentChunk entities
- Processing time only applicable for Assistant messages

**Relationships**:
- Many-to-one with Conversation entity
- References DocumentChunk entities for context

## State Transitions

### Project Status Flow
```
Created → Processing → Ready
Created → Processing → Error
Error → Processing → Ready (retry)
```

### Document Processing Flow
```
Uploaded → Processing → Indexed
Uploaded → Processing → Failed
Failed → Processing → Indexed (retry)
```

## Data Storage Strategy

### Local File System
- Project metadata stored in SQLite database
- Document files stored in organized directory structure:
  ```
  ~/mine-kb-data/
  ├── projects/
  │   ├── {project-id}/
  │   │   ├── documents/
  │   │   │   └── {document-id}-{filename}
  │   │   └── metadata.json
  │   └── database.sqlite
  ```

### Vector Database (Chroma)
- Document chunks stored as embeddings with metadata
- Collection per project for isolation
- Metadata includes: document_id, chunk_index, token_count, content_preview

### Caching Strategy
- Recent conversations cached in memory
- Frequently accessed projects cached
- LLM responses cached by query hash
- Vector search results cached for identical queries

## Performance Considerations

### Indexing Strategy
- SQLite indexes on: project.name, document.project_id, message.conversation_id
- Chroma collections partitioned by project
- Lazy loading of conversation history

### Memory Management
- Streaming document processing to prevent memory spikes
- Pagination for large conversation histories
- Automatic cleanup of old cache entries

### Scalability Limits
- Maximum 1000 projects per installation
- Maximum 1000 documents per project
- Maximum 10MB per document chunk collection
- Maximum 100 concurrent vector searches

This data model provides a solid foundation for the knowledge base application while maintaining performance and scalability within the defined constraints.
