// engine/executor.rs — 步骤执行器
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::NodeExecutor;
use anyhow::Result;

pub struct StepExecutor {
    executors: std::collections::HashMap<String, Box<dyn NodeExecutor>>,
}

impl StepExecutor {
    pub fn new() -> Self {
        let mut executors: std::collections::HashMap<String, Box<dyn NodeExecutor>> =
            std::collections::HashMap::new();

        // 注册节点执行器
        executors.insert("http".to_string(), Box::new(crate::nodes::http::HttpNode));
        executors.insert("data".to_string(), Box::new(crate::nodes::data::DataNode));
        executors.insert("script".to_string(), Box::new(crate::nodes::script::ScriptNode));
        executors.insert("condition".to_string(), Box::new(crate::nodes::condition::ConditionNode));
        executors.insert("loop".to_string(), Box::new(crate::nodes::loop_node::LoopNode));

        StepExecutor { executors }
    }

    pub async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
    ) -> Result<serde_json::Value> {
        let executor = self.executors.get(&step.step_type)
            .ok_or_else(|| anyhow::anyhow!("未知节点类型: {}", step.step_type))?;

        executor.execute(step, ctx).await
    }
}
