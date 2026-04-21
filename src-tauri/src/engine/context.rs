// engine/context.rs — 执行上下文
use crate::engine::workflow::Workflow;
use std::collections::HashMap;

pub struct ExecutionContext {
    pub run_id: String,
    pub variables: HashMap<String, serde_json::Value>,
    pub step_outputs: HashMap<String, serde_json::Value>,
}

impl ExecutionContext {
    pub fn new(run_id: &str, workflow: &Workflow) -> Self {
        let variables = workflow.variables.clone().unwrap_or_default();
        ExecutionContext {
            run_id: run_id.to_string(),
            variables,
            step_outputs: HashMap::new(),
        }
    }

    /// 用 rhai 求值表达式
    pub fn eval_expr(&self, expr: &str) -> Result<serde_json::Value, String> {
        let mut engine = rhai::Engine::new();
        let mut scope = rhai::Scope::new();

        // 注入变量
        for (k, v) in &self.variables {
            scope.push(k.clone(), v.clone());
        }

        let result = engine.eval_with_scope::<rhai::Dynamic>(&mut scope, expr)
            .map_err(|e| format!("表达式求值失败: {}", e))?;

        // 转换 rhai Dynamic 到 JSON
        rhai_to_json(&result)
    }

    pub fn set_step_output(&mut self, step_id: &str, output: serde_json::Value) {
        self.step_outputs.insert(step_id.to_string(), output);
    }

    pub fn get_step_output(&self, step_id: &str) -> Option<&serde_json::Value> {
        self.step_outputs.get(step_id)
    }
}

fn rhai_to_json(val: &rhai::Dynamic) -> Result<serde_json::Value, String> {
    if val.is::<rhai::INT>() {
        Ok(serde_json::json!(val.as_int().unwrap()))
    } else if val.is::<bool>() {
        Ok(serde_json::json!(val.as_bool().unwrap()))
    } else if val.is::<rhai::FLOAT>() {
        Ok(serde_json::json!(val.as_float().unwrap()))
    } else if val.is::<String>() {
        Ok(serde_json::json!(val.as_string().unwrap()))
    } else {
        Ok(serde_json::Value::Null)
    }
}
