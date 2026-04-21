// engine/state.rs — 运行状态
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
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
    pub steps: HashMap<String, StepStatus>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

impl RunState {
    pub fn new(run_id: &str) -> Self {
        RunState {
            run_id: run_id.to_string(),
            status: "running".to_string(),
            steps: HashMap::new(),
            started_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
        }
    }

    pub fn mark_step_done(&mut self, step_id: &str) {
        self.steps.insert(step_id.to_string(), StepStatus::Completed);
    }

    pub fn mark_step_failed(&mut self, step_id: &str) {
        self.steps.insert(step_id.to_string(), StepStatus::Failed);
        self.status = "failed".to_string();
    }

    pub fn finish(&mut self) {
        self.status = "completed".to_string();
        self.finished_at = Some(chrono::Utc::now().to_rfc3339());
    }
}
