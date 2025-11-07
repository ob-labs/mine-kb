use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectStatus {
    Created,
    Processing,
    Ready,
    Error,
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectStatus::Created => write!(f, "Created"),
            ProjectStatus::Processing => write!(f, "Processing"),
            ProjectStatus::Ready => write!(f, "Ready"),
            ProjectStatus::Error => write!(f, "Error"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub document_count: u32,
    pub status: ProjectStatus,
}

impl Project {
    pub fn new(name: String, description: Option<String>) -> Result<Self, ProjectValidationError> {
        Self::validate_name(&name)?;
        Self::validate_description(&description)?;

        let now = Utc::now();
        Ok(Project {
            id: Uuid::new_v4(),
            name,
            description,
            created_at: now,
            updated_at: now,
            document_count: 0,
            status: ProjectStatus::Created,
        })
    }

    pub fn update_status(&mut self, status: ProjectStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    pub fn update_document_count(&mut self, count: u32) {
        self.document_count = count;
        self.updated_at = Utc::now();
    }

    pub fn update_name(&mut self, name: String) -> Result<(), ProjectValidationError> {
        Self::validate_name(&name)?;
        self.name = name;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn update_description(&mut self, description: Option<String>) -> Result<(), ProjectValidationError> {
        Self::validate_description(&description)?;
        self.description = description;
        self.updated_at = Utc::now();
        Ok(())
    }

    fn validate_name(name: &str) -> Result<(), ProjectValidationError> {
        if name.trim().is_empty() {
            return Err(ProjectValidationError::EmptyName);
        }
        if name.len() > 100 {
            return Err(ProjectValidationError::NameTooLong);
        }
        Ok(())
    }

    fn validate_description(description: &Option<String>) -> Result<(), ProjectValidationError> {
        if let Some(desc) = description {
            if desc.len() > 500 {
                return Err(ProjectValidationError::DescriptionTooLong);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub file_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub document_count: u32,
}

impl From<Project> for ProjectResponse {
    fn from(project: Project) -> Self {
        ProjectResponse {
            id: project.id.to_string(),
            name: project.name,
            description: project.description,
            status: project.status.to_string(),
            created_at: project.created_at.to_rfc3339(),
            updated_at: project.updated_at.to_rfc3339(),
            document_count: project.document_count,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectValidationError {
    #[error("Project name cannot be empty")]
    EmptyName,
    #[error("Project name cannot exceed 100 characters")]
    NameTooLong,
    #[error("Project description cannot exceed 500 characters")]
    DescriptionTooLong,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_creation() {
        let project = Project::new("Test Project".to_string(), Some("A test project".to_string()));
        assert!(project.is_ok());

        let project = project.unwrap();
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.description, Some("A test project".to_string()));
        assert_eq!(project.status, ProjectStatus::Created);
        assert_eq!(project.document_count, 0);
    }

    #[test]
    fn test_project_validation() {
        // Test empty name
        let result = Project::new("".to_string(), None);
        assert!(result.is_err());

        // Test name too long
        let result = Project::new("a".repeat(101), None);
        assert!(result.is_err());

        // Test description too long
        let result = Project::new("Valid Name".to_string(), Some("a".repeat(501)));
        assert!(result.is_err());
    }

    #[test]
    fn test_project_status_update() {
        let mut project = Project::new("Test".to_string(), None).unwrap();
        let original_updated_at = project.updated_at;

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));

        project.update_status(ProjectStatus::Processing);
        assert_eq!(project.status, ProjectStatus::Processing);
        assert!(project.updated_at > original_updated_at);
    }

    #[test]
    fn test_project_response_conversion() {
        let project = Project::new("Test Project".to_string(), None).unwrap();
        let response: ProjectResponse = project.into();

        assert_eq!(response.name, "Test Project");
        assert_eq!(response.status, "Created");
        assert!(response.id.len() > 0);
    }
}
