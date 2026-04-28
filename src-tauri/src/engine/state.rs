// engine/state.rs — 运行状态
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunState {
    pub run_id: String,
    pub status: String,
    pub steps: std::collections::HashMap<String, StepStatus>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub variables: std::collections::HashMap<String, serde_json::Value>,
}

impl RunState {
    pub fn new(run_id: &str, variables: std::collections::HashMap<String, serde_json::Value>) -> Self {
        RunState {
            run_id: run_id.to_string(),
            status: "running".to_string(),
            steps: std::collections::HashMap::new(),
            started_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            variables,
        }
    }

    pub fn mark_step_running(&mut self, step_id: &str) {
        self.steps.insert(step_id.to_string(), StepStatus::Running);
    }

    pub fn mark_step_completed(&mut self, step_id: &str) {
        self.steps.insert(step_id.to_string(), StepStatus::Completed);
    }

    pub fn mark_step_failed(&mut self, step_id: &str) {
        self.steps.insert(step_id.to_string(), StepStatus::Failed);
    }

    pub fn mark_step_skipped(&mut self, step_id: &str) {
        self.steps.insert(step_id.to_string(), StepStatus::Skipped);
    }

    pub fn mark_completed(&mut self) {
        self.status = "completed".to_string();
        self.finished_at = Some(chrono::Utc::now().to_rfc3339());
    }

    pub fn mark_failed(&mut self) {
        self.status = "failed".to_string();
        self.finished_at = Some(chrono::Utc::now().to_rfc3339());
    }
}
