use crate::models::project::Project;
use crate::services::seekdb_adapter::SeekDbAdapter;
use anyhow::{anyhow, Result};
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ProjectService {
    projects: HashMap<Uuid, Project>,
    db: Arc<Mutex<SeekDbAdapter>>,
}

impl ProjectService {
    pub fn new(db: Arc<Mutex<SeekDbAdapter>>) -> Self {
        let mut service = Self {
            projects: HashMap::new(),
            db,
        };

        // 从数据库加载已有项目
        if let Err(e) = service.load_projects_from_db() {
            log::error!("加载项目失败: {}", e);
        }

        service
    }

    /// 从数据库加载项目到内存
    fn load_projects_from_db(&mut self) -> Result<()> {
        let db = self.db.clone();
        let db_guard = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                db.lock().await
            })
        });

        let projects = db_guard.load_all_projects()?;
        log::info!("从数据库加载了 {} 个项目", projects.len());

        for project in projects {
            self.projects.insert(project.id, project);
        }

        Ok(())
    }

    /// 保存项目到数据库
    pub fn save_project_to_db(&self, project: &Project) -> Result<()> {
        let db = self.db.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut db_guard = db.lock().await;
                db_guard.save_project(project)
            })
        })
    }

    /// 从数据库删除项目
    fn delete_project_from_db(&self, project_id: Uuid) -> Result<()> {
        let db = self.db.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut db_guard = db.lock().await;
                db_guard.delete_project_by_id(&project_id.to_string())?;
                db_guard.delete_project_documents(&project_id.to_string())
                    .map(|_| ())
            })
        })
    }

    pub fn create_project(&mut self, name: String, description: Option<String>) -> Result<Uuid> {
        let project = Project::new(name, description)?;
        let project_id = project.id;

        // 保存到数据库
        self.save_project_to_db(&project)?;

        // 保存到内存
        self.projects.insert(project_id, project);
        Ok(project_id)
    }

    pub fn get_project(&self, project_id: Uuid) -> Option<&Project> {
        self.projects.get(&project_id)
    }

    pub fn get_project_mut(&mut self, project_id: Uuid) -> Option<&mut Project> {
        self.projects.get_mut(&project_id)
    }

    pub fn list_projects(&self) -> Vec<&Project> {
        self.projects.values().collect()
    }

    pub fn update_project(
        &mut self,
        project_id: Uuid,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<()> {
        {
            let project = self.projects
                .get_mut(&project_id)
                .ok_or_else(|| anyhow!("Project not found: {}", project_id))?;

            if let Some(new_name) = name {
                project.update_name(new_name)?;
            }

            if let Some(new_description) = description {
                project.update_description(Some(new_description))?;
            }
        }

        // 保存到数据库
        if let Some(project) = self.projects.get(&project_id) {
            self.save_project_to_db(project)?;
        }

        Ok(())
    }

    pub fn delete_project(&mut self, project_id: Uuid) -> Result<()> {
        self.projects
            .remove(&project_id)
            .ok_or_else(|| anyhow!("Project not found: {}", project_id))?;

        // 从数据库删除
        self.delete_project_from_db(project_id)?;

        Ok(())
    }

    pub fn project_exists(&self, project_id: Uuid) -> bool {
        self.projects.contains_key(&project_id)
    }

    pub fn find_projects_by_name(&self, name_pattern: &str) -> Vec<&Project> {
        let pattern = name_pattern.to_lowercase();
        self.projects
            .values()
            .filter(|project| project.name.to_lowercase().contains(&pattern))
            .collect()
    }

    pub fn count_projects(&self) -> usize {
        self.projects.len()
    }

    pub fn get_project_stats(&self, project_id: Uuid) -> Result<ProjectStats> {
        let project = self.projects
            .get(&project_id)
            .ok_or_else(|| anyhow!("Project not found: {}", project_id))?;

        // In a real implementation, these would be calculated from actual data
        Ok(ProjectStats {
            project_id,
            document_count: 0,
            conversation_count: 0,
            total_chunks: 0,
            storage_size: 0,
            created_at: project.created_at,
            updated_at: project.updated_at,
        })
    }

    pub fn update_project_status(&mut self, project_id: Uuid, status: crate::models::project::ProjectStatus) -> Result<()> {
        {
            let project = self.projects
                .get_mut(&project_id)
                .ok_or_else(|| anyhow!("Project not found: {}", project_id))?;

            project.update_status(status);
        }

        // 保存到数据库
        if let Some(project) = self.projects.get(&project_id) {
            self.save_project_to_db(project)?;
        }

        Ok(())
    }

    pub fn list_projects_by_status(&self, status: crate::models::project::ProjectStatus) -> Vec<&Project> {
        self.projects
            .values()
            .filter(|project| project.status == status)
            .collect()
    }
}


#[derive(Debug, Clone)]
pub struct ProjectStats {
    pub project_id: Uuid,
    pub document_count: usize,
    pub conversation_count: usize,
    pub total_chunks: usize,
    pub storage_size: u64,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_service_creation() {
        let service = ProjectService::new();
        assert_eq!(service.projects.len(), 0);
    }

    #[test]
    fn test_create_and_get_project() {
        let mut service = ProjectService::new();

        let project_id = service.create_project(
            "Test Project".to_string(),
            Some("A test project".to_string()),
        ).unwrap();

        let project = service.get_project(project_id).unwrap();
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.description, Some("A test project".to_string()));
        assert!(!project.is_archived);
    }

    #[test]
    fn test_update_project() {
        let mut service = ProjectService::new();

        let project_id = service.create_project("Original".to_string(), None).unwrap();

        service.update_project(
            project_id,
            Some("Updated".to_string()),
            Some("Updated description".to_string()),
        ).unwrap();

        let project = service.get_project(project_id).unwrap();
        assert_eq!(project.name, "Updated");
        assert_eq!(project.description, Some("Updated description".to_string()));
    }

    #[test]
    fn test_delete_project() {
        let mut service = ProjectService::new();

        let project_id = service.create_project("Test".to_string(), None).unwrap();
        assert!(service.get_project(project_id).is_some());

        service.delete_project(project_id).unwrap();
        assert!(service.get_project(project_id).is_none());
    }

    #[test]
    fn test_find_projects_by_name() {
        let mut service = ProjectService::new();

        service.create_project("My Project".to_string(), None).unwrap();
        service.create_project("Another Project".to_string(), None).unwrap();
        service.create_project("Something Else".to_string(), None).unwrap();

        let results = service.find_projects_by_name("project");
        assert_eq!(results.len(), 2);

        let results = service.find_projects_by_name("My");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_project_status_update() {
        let mut service = ProjectService::new();

        let project_id = service.create_project("Test".to_string(), None).unwrap();

        // Initially Created status
        assert_eq!(service.get_project(project_id).unwrap().status, crate::models::project::ProjectStatus::Created);

        // Update to Processing
        service.update_project_status(project_id, crate::models::project::ProjectStatus::Processing).unwrap();
        assert_eq!(service.get_project(project_id).unwrap().status, crate::models::project::ProjectStatus::Processing);

        // Test filtering by status
        let processing_projects = service.list_projects_by_status(crate::models::project::ProjectStatus::Processing);
        assert_eq!(processing_projects.len(), 1);

        let ready_projects = service.list_projects_by_status(crate::models::project::ProjectStatus::Ready);
        assert_eq!(ready_projects.len(), 0);
    }

    #[test]
    fn test_project_stats() {
        let mut service = ProjectService::new();

        let project_id = service.create_project("Test".to_string(), None).unwrap();
        let stats = service.get_project_stats(project_id).unwrap();

        assert_eq!(stats.project_id, project_id);
        assert_eq!(stats.document_count, 0);
        assert_eq!(stats.conversation_count, 0);
    }

    #[test]
    fn test_project_exists() {
        let mut service = ProjectService::new();

        let project_id = service.create_project("Test".to_string(), None).unwrap();
        assert!(service.project_exists(project_id));

        let non_existent_id = Uuid::new_v4();
        assert!(!service.project_exists(non_existent_id));
    }
}
