// engine/context.rs — 执行上下文
use crate::engine::workflow::Workflow;
use std::collections::HashMap;
use std::cmp::Reverse;
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
                engine.set_max_operations(100_000);
                engine.set_max_string_size(1024 * 1024);
                engine.set_max_array_size(10_000);
                engine.set_max_map_size(10_000);
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

    fn resolve_string(&self, s: &str) -> serde_json::Value {
        let trimmed = s.trim();
        if trimmed.starts_with("{{") && trimmed.ends_with("}}") && trimmed.len() > 4 {
            let inner = trimmed[2..trimmed.len()-2].trim();
            if !inner.contains("{{") {
                if let Some(val) = self.resolve_var(inner) {
                    return val.clone();
                }
                if let Some(result) = self.eval_simple_expr(inner) {
                    return result;
                }
            }
        }
        serde_json::Value::String(self.interpolate(s))
    }

    /// 简单表达式求值：支持 算术 +-*/ | 比较 > < >= <= == != | 逻辑 && || !
    fn eval_simple_expr(&self, expr: &str) -> Option<serde_json::Value> {
        let has_arith = expr.contains('+') || expr.contains('-')
            || expr.contains('*') || expr.contains('/');
        let has_logic = expr.contains('>') || expr.contains('<')
            || expr.contains('!') || expr.contains('=')
            || expr.contains("&&") || expr.contains("||");
        if !has_arith && !has_logic {
            return None;
        }

        // 替换变量为字面量
        let mut resolved = expr.to_string();
        let var_patterns: Vec<String> = {
            let mut vars = Vec::new();
            let mut i = 0;
            let chars: Vec<char> = expr.chars().collect();
            while i < chars.len() {
                if chars[i].is_alphabetic() || chars[i] == '_' {
                    let start = i;
                    while i < chars.len()
                        && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
                    {
                        i += 1;
                    }
                    let name = &expr[start..i];
                    if name != "true" && name != "false" {
                        vars.push(name.to_string());
                    }
                } else {
                    i += 1;
                }
            }
            vars
        };

        let mut sorted_vars = var_patterns;
        sorted_vars.sort_by_key(|b| Reverse(b.len()));
        sorted_vars.dedup();
        for var in &sorted_vars {
            if let Some(val) = self.resolve_var(var) {
                let literal = match val {
                    serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0).to_string(),
                    serde_json::Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
                    serde_json::Value::Bool(true) => "true".to_string(),
                    serde_json::Value::Bool(false) => "false".to_string(),
                    _ => continue,
                };
                resolved = resolved.replace(var.as_str(), &literal);
            }
        }

        // 安全校验
        let allowed = "0123456789+-*/.<>=!&|() \"truefalse";
        if resolved.chars().any(|c| !allowed.contains(c)) {
            return None;
        }

        Self::evaluate(&resolved)
    }

    /// 表达式求值：多级运算符优先级，返回 JSON Value
    fn evaluate(expr: &str) -> Option<serde_json::Value> {
        let expr = expr.trim();
        if expr.is_empty() {
            return None;
        }

        // 一元 !
        if let Some(rest) = expr.strip_prefix('!') {
            let val = Self::evaluate(rest)?;
            return Some(serde_json::Value::Bool(!is_truthy(&val)));
        }

        // 去掉全包裹括号
        if let Some(inner) = strip_outer_parens(expr) {
            return Self::evaluate(inner);
        }

        // 1. || (最低)
        if let Some(pos) = Self::find_op(expr, "||") {
            let left = Self::evaluate(&expr[..pos])?;
            let right = Self::evaluate(&expr[pos + 2..])?;
            return Some(serde_json::Value::Bool(is_truthy(&left) || is_truthy(&right)));
        }

        // 2. &&
        if let Some(pos) = Self::find_op(expr, "&&") {
            let left = Self::evaluate(&expr[..pos])?;
            let right = Self::evaluate(&expr[pos + 2..])?;
            return Some(serde_json::Value::Bool(is_truthy(&left) && is_truthy(&right)));
        }

        // 3. 比较 == != >= <= > <
        let comp_ops: [(&str, usize); 6] = [
            ("==", 2), ("!=", 2), (">=", 2), ("<=", 2), (">", 1), ("<", 1),
        ];
        for (op, len) in &comp_ops {
            if let Some(pos) = Self::find_op(expr, op) {
                let lv = Self::evaluate(&expr[..pos])?;
                let rv = Self::evaluate(&expr[pos + len..])?;
                let result = match *op {
                    "==" => json_eq(&lv, &rv),
                    "!=" => !json_eq(&lv, &rv),
                    ">" => json_cmp(&lv, &rv) == Some(std::cmp::Ordering::Greater),
                    "<" => json_cmp(&lv, &rv) == Some(std::cmp::Ordering::Less),
                    ">=" => matches!(
                        json_cmp(&lv, &rv),
                        Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                    ),
                    "<=" => matches!(
                        json_cmp(&lv, &rv),
                        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                    ),
                    _ => false,
                };
                return Some(serde_json::Value::Bool(result));
            }
        }

        // 4. 算术 + - * /
        Self::compute_arithmetic(expr).map(|n| {
            if (n - n.round()).abs() < 1e-10 {
                serde_json::json!(n as i64)
            } else {
                serde_json::json!(n)
            }
        }).or_else(|| {
            // 纯字面量
            let cleaned = expr.trim();
            if cleaned == "true" { return Some(serde_json::Value::Bool(true)); }
            if cleaned == "false" { return Some(serde_json::Value::Bool(false)); }
            if let Ok(n) = cleaned.parse::<f64>() {
                return if (n - n.round()).abs() < 1e-10 {
                    Some(serde_json::json!(n as i64))
                } else {
                    Some(serde_json::json!(n))
                };
            }
            if cleaned.starts_with('"') && cleaned.ends_with('"') && cleaned.len() >= 2 {
                return Some(serde_json::Value::String(
                    cleaned[1..cleaned.len() - 1].to_string(),
                ));
            }
            None
        })
    }

    /// 在表达式顶层（非括号/非引号内）找运算符位置
    fn find_op(expr: &str, op: &str) -> Option<usize> {
        let mut depth: i32 = 0;
        let chars: Vec<char> = expr.chars().collect();
        let op_len = op.chars().count();
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '(' => depth += 1,
                ')' => depth -= 1,
                '"' => {
                    i += 1;
                    while i < chars.len() && chars[i] != '"' {
                        if chars[i] == '\\' { i += 1; }
                        i += 1;
                    }
                }
                _ if depth == 0 && i + op_len <= chars.len() => {
                    let slice: String = chars[i..i + op_len].iter().collect();
                    if slice == op {
                        // 防止单个 = 匹配到 == 的第一个字符
                        if op == "=" && i + 1 < chars.len() && chars[i + 1] == '=' {
                            i += 2;
                            continue;
                        }
                        return Some(i);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        None
    }

    /// 四则运算
    fn compute_arithmetic(expr: &str) -> Option<f64> {
        let expr = expr.trim();
        if expr.is_empty() { return None; }

        let mut depth: i32 = 0;
        let mut op_pos: Option<(usize, char)> = None;
        let chars: Vec<char> = expr.chars().collect();

        for i in 0..chars.len() {
            match chars[i] {
                '(' => depth += 1,
                ')' => depth -= 1,
                '+' | '-' if depth == 0 => {
                    if i > 0 && (chars[i - 1].is_ascii_digit() || chars[i - 1] == ')' || chars[i - 1] == ' ') {
                        op_pos = Some((i, chars[i]));
                    } else if chars[i] == '-' && i == 0 {
                        continue;
                    } else {
                        op_pos = Some((i, chars[i]));
                    }
                }
                '*' | '/' if depth == 0
                    && op_pos.is_none() => {
                        op_pos = Some((i, chars[i]));
                    }
                _ => {}
            }
        }

        if let Some((pos, op)) = op_pos {
            let left = Self::compute_arithmetic(&expr[..pos])?;
            let right = Self::compute_arithmetic(&expr[pos + 1..])?;
            return match op {
                '+' => Some(left + right),
                '-' => Some(left - right),
                '*' => Some(left * right),
                '/' => if right == 0.0 { None } else { Some(left / right) },
                _ => None,
            };
        }

        let cleaned: String = expr.chars()
            .filter(|c| !c.is_whitespace() && *c != '(' && *c != ')')
            .collect();
        cleaned.parse::<f64>().ok()
    }

    pub fn resolve_var(&self, key: &str) -> Option<&serde_json::Value> {
        let parts: Vec<&str> = key.split('.').collect();
        let root_key = parts[0];

        let root = if let Some(step_id) = root_key.strip_prefix("step_") {
            self.step_outputs.get(step_id)
        } else {
            self.variables.get(root_key)
        };

        let mut current = root?;
        for part in &parts[1..] {
            current = current.get(*part)?;
        }
        Some(current)
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

fn is_truthy(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
    }
}

fn json_eq(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Number(na), serde_json::Value::Number(nb)) => {
            (na.as_f64().unwrap_or(0.0) - nb.as_f64().unwrap_or(0.0)).abs() < 1e-10
        }
        (serde_json::Value::Bool(ba), serde_json::Value::Bool(bb)) => ba == bb,
        (serde_json::Value::Null, serde_json::Value::Null) => true,
        _ => *a == *b,
    }
}

fn json_cmp(a: &serde_json::Value, b: &serde_json::Value) -> Option<std::cmp::Ordering> {
    if let (serde_json::Value::Number(na), serde_json::Value::Number(nb)) = (a, b) {
        let fa = na.as_f64().unwrap_or(0.0);
        let fb = nb.as_f64().unwrap_or(0.0);
        Some(if fa < fb {
            std::cmp::Ordering::Less
        } else if fa > fb {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        })
    } else {
        None
    }
}

fn strip_outer_parens(expr: &str) -> Option<&str> {
    if !expr.starts_with('(') || !expr.ends_with(')') {
        return None;
    }
    let mut depth: i32 = 0;
    let chars: Vec<char> = expr.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c == '(' { depth += 1; }
        if c == ')' { depth -= 1; }
        if depth == 0 && i < chars.len() - 1 {
            return None; // 括号不配对全包裹
        }
    }
    if depth == 0 {
        Some(&expr[1..expr.len() - 1])
    } else {
        None
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
