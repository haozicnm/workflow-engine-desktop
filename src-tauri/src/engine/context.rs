// engine/context.rs — 执行上下文
use crate::engine::workflow::Workflow;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

/// 执行上下文
#[derive(Clone)]
pub struct ExecutionContext {
    pub run_id: String,
    pub variables: HashMap<String, serde_json::Value>,
    pub step_outputs: HashMap<String, serde_json::Value>,
    /// 容器节点输入端口数据（由 DAG 调度器从上游连线注入）
    pub input_ports: HashMap<String, serde_json::Value>,
    /// v4.1: 容器 session 管理 — node_id → session 信息
    pub sessions: HashMap<String, ContainerSession>,
    /// 全局默认超时配置（节点可从这里获取回退值）
    pub default_timeouts: crate::data::config::TimeoutConfig,
    /// Shell 命令白名单（glob 模式），空=允许所有
    pub shell_allowed_commands: Vec<String>,
    /// 子流程嵌套深度（用于防止无限递归）
    pub sub_workflow_depth: u32,
    /// 调试：单步模式标志（true = 每步暂停等待 debug_step）
    pub step_mode_flag: Option<Arc<std::sync::atomic::AtomicBool>>,
    /// 调试：断点暂停标志（true = 在断点处暂停等待恢复）
    pub breakpoint_flag: Option<Arc<std::sync::atomic::AtomicBool>>,
    /// 调试：暂停标志（true = 暂停执行）
    pub pause_flag: Option<Arc<std::sync::atomic::AtomicBool>>,
}

/// 容器 session 状态
#[derive(Clone, Debug)]
pub struct ContainerSession {
    pub session_id: String,
    pub node_id: String,
    pub node_type: String,
    pub status: SessionStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionStatus {
    Created,
    Running,
    Closed,
}

impl ExecutionContext {
    pub fn new(run_id: &str, workflow: &Workflow) -> Self {
        let variables = workflow.variables.clone().unwrap_or_default();
        ExecutionContext {
            run_id: run_id.to_string(),
            variables,
            step_outputs: HashMap::new(),
            input_ports: HashMap::new(),
            sessions: HashMap::new(),
            default_timeouts: crate::data::config::TimeoutConfig::default(),
            shell_allowed_commands: Vec::new(),
            sub_workflow_depth: 0,
            step_mode_flag: None,
            breakpoint_flag: None,
            pause_flag: None,
        }
    }

    /// v4.1: 打开容器 session（幂等 — 已存在则返回已有 session）
    pub fn open_session(&mut self, node_id: &str, node_type: &str) -> &ContainerSession {
        let session =
            self.sessions
                .entry(node_id.to_string())
                .or_insert_with(|| ContainerSession {
                    session_id: format!("{}-{}", node_id, Uuid::new_v4()),
                    node_id: node_id.to_string(),
                    node_type: node_type.to_string(),
                    status: SessionStatus::Created,
                });
        if session.status == SessionStatus::Created {
            session.status = SessionStatus::Running;
        }
        &*session
    }

    /// v4.1: 关闭容器 session
    pub fn close_session(&mut self, node_id: &str) {
        if let Some(s) = self.sessions.get_mut(node_id) {
            s.status = SessionStatus::Closed;
        }
    }

    /// 用 rhai 求值表达式（沙箱化：纯计算，无 I/O，有限资源）
    pub fn eval_expr(&self, expr: &str) -> Result<serde_json::Value, String> {
        thread_local! {
            static RHAI_ENGINE: rhai::Engine = {
                let mut engine = rhai::Engine::new();
                engine.set_max_operations(100_000);
                engine.set_max_string_size(1024 * 1024);
                engine.set_max_array_size(10_000);
                engine.set_max_map_size(10_000);
                engine
            };
        }

        RHAI_ENGINE.with(|engine| {
            let mut scope = rhai::Scope::new();

            // v7.1: 迭代变量（__item/__index/__index1/loop）注入顶层作用域
            // 这些是循环体内临时变量，生命周期仅一次迭代，不会与步骤输出冲突
            let iter_keys: &[&str] = &["__item", "__index", "__index1", "loop"];
            for k in iter_keys {
                if let Some(v) = self.variables.get(*k) {
                    scope.push(k.to_string(), json_to_rhai(v));
                }
            }
            let mut vars_map = rhai::Map::new();
            for (k, v) in &self.variables {
                vars_map.insert(k.clone().into(), json_to_rhai(v));
            }
            scope.push("__vars__".to_string(), rhai::Dynamic::from(vars_map));

            // 步骤输出保持 step_ 前缀
            for (k, v) in &self.step_outputs {
                let dynamic = json_to_rhai(v);
                let stem = k.strip_prefix("step_").unwrap_or(k);
                scope.push(format!("step_{}", stem), dynamic);
            }

            let result = engine
                .eval_with_scope::<rhai::Dynamic>(&mut scope, expr)
                .map_err(|e| {
                    eprintln!("[eval_expr DEBUG] expr='{}'", expr);
                    for (k, _, _) in scope.iter() {
                        eprintln!("  scope.{}", k);
                    }
                    format!("表达式求值失败: {}", e)
                })?;

            Ok(rhai_to_json(&result))
        })
    }

    /// 设置变量
    pub fn set_var(&mut self, key: String, value: serde_json::Value) {
        self.variables.insert(key, value);
    }

    /// 设置步骤输出
    pub fn set_output(&mut self, step_id: &str, output: serde_json::Value) {
        self.step_outputs.insert(step_id.to_string(), output);
    }

    /// 获取步骤输出
    pub fn get_output(&self, step_id: &str) -> Option<&serde_json::Value> {
        self.step_outputs.get(step_id)
    }

    // ─── 配置变量替换 ───

    pub fn resolve_config(&self, config: &serde_json::Value) -> serde_json::Value {
        match config {
            serde_json::Value::String(s) => self.resolve_string(s),
            serde_json::Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    new_map.insert(k.clone(), self.resolve_config(v));
                }
                serde_json::Value::Object(new_map)
            }
            serde_json::Value::Array(arr) => {
                let new_arr: Vec<serde_json::Value> =
                    arr.iter().map(|v| self.resolve_config(v)).collect();
                serde_json::Value::Array(new_arr)
            }
            other => other.clone(),
        }
    }

    fn resolve_string(&self, s: &str) -> serde_json::Value {
        let trimmed = s.trim();
        if trimmed.starts_with("{{") && trimmed.ends_with("}}") && trimmed.len() > 4 {
            let inner = trimmed[2..trimmed.len() - 2].trim();
            if inner.is_empty() {
                warn!("空的模板变量引用 '{{{{}}}}' → 返回 null");
                return serde_json::Value::Null;
            }
            if !inner.contains("{{") {
                if let Some(val) = self.resolve_var(inner) {
                    return val.clone();
                }
                if let Ok(result) = self.eval_expr(inner) {
                    return result;
                }
                // 纯变量引用解析失败 → 返回 Null 并警告
                warn!(
                    "变量引用 '{{{{{}}}}}' 解析失败 → 返回 null（步骤输出或变量不存在）",
                    inner
                );
                return serde_json::Value::Null;
            }
        }
        serde_json::Value::String(self.interpolate(s))
    }
    /// 访问 JSON 值的字段，支持对象字段、数组索引和嵌套括号
    /// "name" → obj.name, "0" → arr[0], "items[0]" → obj.items[0], "items[0][1]" → obj.items[0][1]
    fn get_field<'a>(value: &'a serde_json::Value, field: &str) -> Option<&'a serde_json::Value> {
        let mut current = value;
        let mut remaining = field;
        while !remaining.is_empty() {
            if let Some(bracket_pos) = remaining.find('[') {
                let key = &remaining[..bracket_pos];
                if !key.is_empty() {
                    current = current.as_object()?.get(key)?;
                }
                remaining = &remaining[bracket_pos + 1..];
                let close = remaining.find(']')?;
                let idx: usize = remaining[..close].parse().ok()?;
                current = current.as_array()?.get(idx)?;
                remaining = &remaining[close + 1..];
            } else if let Some(obj) = current.as_object() {
                return obj.get(remaining);
            } else if let Some(arr) = current.as_array() {
                return remaining.parse::<usize>().ok().and_then(|i| arr.get(i));
            } else {
                return None;
            }
        }
        Some(current)
    }

    pub fn resolve_var(&self, key: &str) -> Option<&serde_json::Value> {
        let parts: Vec<&str> = key.split('.').collect();
        let root_key = parts[0];

        // 特殊处理：{{params.X.Y}} — workflow params 以 flat map 存储
        if root_key == "params" && parts.len() >= 2 {
            if let Some(root) = self.variables.get(parts[1]) {
                let mut current = root;
                for part in &parts[2..] {
                    current = Self::get_field(current, part)?;
                }
                return Some(current);
            }
        }

        // v7.1: 新命名空间 {{vars.xxx}} — 显式引用用户变量
        if root_key == "vars" && parts.len() >= 2 {
            if let Some(root) = self.variables.get(parts[1]) {
                let mut current = root;
                for part in &parts[2..] {
                    current = Self::get_field(current, part)?;
                }
                return Some(current);
            }
        }

        // 显式 step_ 前缀：只从 step_outputs 查找
        if let Some(step_id) = root_key.strip_prefix("step_") {
            // 1. 完整 key 查 step_outputs（如 step_1）
            if let Some(root) = self.step_outputs.get(root_key) {
                let mut current = root;
                for part in &parts[1..] {
                    current = Self::get_field(current, part)?;
                }
                return Some(current);
            }
            // 2. strip 后的 key 查 step_outputs（如 "1"）
            if let Some(root) = self.step_outputs.get(step_id) {
                let mut current = root;
                for part in &parts[1..] {
                    current = Self::get_field(current, part)?;
                }
                return Some(current);
            }
            return None;
        }

        // 旧语法兼容：无前缀 key（如 {{body}}）→ 先查 step_outputs，再查 variables
        // 这样模板中 {{body}} 仍然能找到，但不会和 step_3.body 的 body 字段冲突
        if let Some(root) = self.step_outputs.get(root_key) {
            let mut current = root;
            for part in &parts[1..] {
                current = Self::get_field(current, part)?;
            }
            return Some(current);
        }
        if let Some(root) = self.variables.get(root_key) {
            let mut current = root;
            for part in &parts[1..] {
                current = Self::get_field(current, part)?;
            }
            return Some(current);
        }
        None
    }

    fn interpolate(&self, s: &str) -> String {
        let mut result = String::new();
        let mut remaining = s;

        while let Some(start) = remaining.find("{{") {
            result.push_str(&remaining[..start]);
            if let Some(end) = remaining[start + 2..].find("}}") {
                let key = remaining[start + 2..start + 2 + end].trim();
                if let Some(val) = self.resolve_var(key) {
                    match val {
                        serde_json::Value::String(sv) => result.push_str(sv),
                        other => result.push_str(&other.to_string()),
                    }
                } else {
                    // 保留未解析的模板原文，让用户能看到哪里出了问题
                    warn!("variable not found: '{}' → kept as-is", key);
                    result.push_str("{{");
                    result.push_str(key);
                    result.push_str("}}");
                }
                remaining = &remaining[start + 2 + end + 2..];
            } else {
                result.push_str(remaining);
                return result;
            }
        }
        result.push_str(remaining);
        result
    }
}

// ─── 辅助函数 ───
pub fn rhai_to_json(val: &rhai::Dynamic) -> serde_json::Value {
    if val.is::<rhai::INT>() {
        serde_json::json!(val.clone().as_int().unwrap_or(0))
    } else if val.is::<bool>() {
        serde_json::Value::Bool(val.clone().as_bool().unwrap_or(false))
    } else if val.is::<f64>() {
        serde_json::json!(val.clone().as_float().unwrap_or(0.0))
    } else if val.is::<String>() {
        serde_json::Value::String(val.clone().into_string().unwrap_or_default())
    } else if val.is::<()>() {
        serde_json::Value::Null
    } else if val.is::<rhai::Array>() {
        let arr = val.clone().into_array().unwrap_or_default();
        let json_arr: Vec<serde_json::Value> = arr.iter().map(rhai_to_json).collect();
        serde_json::Value::Array(json_arr)
    } else if val.is::<rhai::Map>() {
        let map: rhai::Map = val.clone().cast();
        let mut obj = serde_json::Map::new();
        for (k, v) in map {
            obj.insert(k.to_string(), rhai_to_json(&v));
        }
        serde_json::Value::Object(obj)
    } else {
        serde_json::Value::String(val.to_string())
    }
}

/// serde_json::Value → rhai::Dynamic
pub fn json_to_rhai(val: &serde_json::Value) -> rhai::Dynamic {
    match val {
        serde_json::Value::Null => rhai::Dynamic::UNIT,
        serde_json::Value::Bool(b) => rhai::Dynamic::from_bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rhai::Dynamic::from_int(i as rhai::INT)
            } else if let Some(f) = n.as_f64() {
                rhai::Dynamic::from_float(f)
            } else {
                rhai::Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => rhai::Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let rhai_arr: rhai::Array = arr.iter().map(json_to_rhai).collect();
            rhai::Dynamic::from(rhai_arr)
        }
        serde_json::Value::Object(obj) => {
            let mut map = rhai::Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_to_rhai(v));
            }
            rhai::Dynamic::from(map)
        }
    }
}
