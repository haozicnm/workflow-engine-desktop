// engine/context.rs — 执行上下文
use crate::engine::workflow::Workflow;
use std::collections::HashMap;
use anyhow::Result;

/// 执行上下文
#[derive(Clone)]
pub struct ExecutionContext {
    pub run_id: String,
    pub variables: HashMap<String, serde_json::Value>,
    pub step_outputs: HashMap<String, serde_json::Value>,
    /// 浏览器通道设置（auto / msedge / chrome / chromium）
    pub browser_channel: String,
}

impl ExecutionContext {
    pub fn new(run_id: &str, workflow: &Workflow) -> Self {
        let variables = workflow.variables.clone().unwrap_or_default();
        ExecutionContext {
            run_id: run_id.to_string(),
            variables,
            step_outputs: HashMap::new(),
            browser_channel: "auto".to_string(),
        }
    }

    /// 用 rhai 求值表达式（沙箱化：纯计算，无 I/O，有限资源）
    pub fn eval_expr(&self, expr: &str) -> Result<serde_json::Value, String> {
        thread_local! {
            static RHAI_ENGINE: rhai::Engine = {
                let mut engine = rhai::Engine::new();
                // 沙箱限制：防止死循环和资源滥用
                engine.set_max_operations(100_000);       // 防止死循环
                engine.set_max_string_size(1024 * 1024);  // 限制字符串 1MB
                engine.set_max_array_size(10_000);        // 限制数组大小
                engine.set_max_map_size(10_000);          // 限制 map 大小
                // 不注册任何 I/O 包，仅纯计算
                // doc comments are disabled by default in rhai >= 1.16
                engine
            };
        }

        RHAI_ENGINE.with(|engine| {
            let mut scope = rhai::Scope::new();
            for (k, v) in &self.variables {
                let dynamic = json_to_rhai(v);
                scope.push(k.clone(), dynamic);
            }
            for (k, v) in &self.step_outputs {
                let dynamic = json_to_rhai(v);
                scope.push(format!("step_{}", k), dynamic);
            }

            let result = engine.eval_with_scope::<rhai::Dynamic>(&mut scope, expr)
                .map_err(|e| format!("表达式求值失败: {}", e))?;

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

    /// 递归替换 config JSON 中的 {{变量}} 占位符
    ///
    /// 支持：
    ///   {{key}}       → ctx.variables[key]
    ///   {{step_xxx}}  → ctx.step_outputs[xxx]
    ///   {{__item}}    → 当前循环项
    ///   {{__index}}   → 当前循环索引
    ///   {{a.b.c}}     → 对象嵌套访问
    ///
    /// 如果整个值是单个 {{key}}，保留原始 JSON 类型；
    /// 如果 {{key}} 嵌在文本中，转为字符串拼接。
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
                let new_arr: Vec<serde_json::Value> = arr.iter()
                    .map(|v| self.resolve_config(v))
                    .collect();
                serde_json::Value::Array(new_arr)
            }
            other => other.clone(),
        }
    }

    /// 解析字符串中的变量引用
    fn resolve_string(&self, s: &str) -> serde_json::Value {
        let trimmed = s.trim();
        // 整个字符串是单个 {{key}} → 保留原始类型
        if trimmed.starts_with("{{") && trimmed.ends_with("}}") && trimmed.len() > 4 {
            let inner = trimmed[2..trimmed.len()-2].trim();
            if !inner.contains("{{") {
                if let Some(val) = self.resolve_var(inner) {
                    return val.clone();
                }
            }
        }
        // 否则：字符串插值
        serde_json::Value::String(self.interpolate(s))
    }

    /// 从上下文中解析变量
    /// 支持：
    ///   key       → variables[key]
    ///   step_xxx  → step_outputs[xxx]
    ///   a.b.c     → 嵌套对象访问
    pub fn resolve_var(&self, key: &str) -> Option<&serde_json::Value> {
        // 分割嵌套路径
        let parts: Vec<&str> = key.split('.').collect();
        let root_key = parts[0];

        // 查找根值
        let root = if let Some(step_id) = root_key.strip_prefix("step_") {
            self.step_outputs.get(step_id)
        } else {
            self.variables.get(root_key)
        };

        let mut current = root?;

        // 遍历嵌套路径
        for part in &parts[1..] {
            current = current.get(*part)?;
        }

        Some(current)
    }

    /// 字符串插值：将 {{key}} 替换为解析后的值
    fn interpolate(&self, s: &str) -> String {
        let mut result = String::new();
        let mut remaining = s;

        while let Some(start) = remaining.find("{{") {
            result.push_str(&remaining[..start]);
            if let Some(end) = remaining[start+2..].find("}}") {
                let key = remaining[start+2..start+2+end].trim();
                if let Some(val) = self.resolve_var(key) {
                    match val {
                        serde_json::Value::String(sv) => result.push_str(sv),
                        other => result.push_str(&other.to_string()),
                    }
                } else {
                    // 变量未找到，保留原始占位符
                    result.push_str("{{");
                    result.push_str(key);
                    result.push_str("}}");
                }
                remaining = &remaining[start+2+end+2..];
            } else {
                // 没有闭合的 }}，保留剩余部分
                result.push_str(remaining);
                return result;
            }
        }
        result.push_str(remaining);
        result
    }
}

/// rhai::Dynamic → serde_json::Value
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
