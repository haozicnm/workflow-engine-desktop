// nodes/error_utils.rs — 统一错误处理工具
//
// 提供标准化的错误响应格式，所有节点使用统一的错误结构：
// {
//   "error": "错误信息",
//   "code": "ERROR_CODE",
//   "suggestion": "建议操作"
// }

use serde_json::json;

/// 标准错误响应结构
#[derive(Debug, Clone)]
pub struct NodeError {
    pub message: String,
    pub code: String,
    pub suggestion: String,
}

impl NodeError {
    pub fn new(message: impl Into<String>, code: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: code.into(),
            suggestion: suggestion.into(),
        }
    }

    /// 转换为 serde_json::Value
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "error": self.message,
            "code": self.code,
            "suggestion": self.suggestion,
        })
    }

    /// 转换为 anyhow::Error
    pub fn to_error(&self) -> anyhow::Error {
        anyhow::anyhow!("{}", self.message)
    }
}

/// 常用错误代码常量
pub mod error_codes {
    pub const MISSING_PARAMETER: &str = "MISSING_PARAMETER";
    pub const INVALID_PARAMETER: &str = "INVALID_PARAMETER";
    pub const TIMEOUT: &str = "TIMEOUT";
    pub const NETWORK_ERROR: &str = "NETWORK_ERROR";
    pub const PERMISSION_DENIED: &str = "PERMISSION_DENIED";
    pub const FILE_NOT_FOUND: &str = "FILE_NOT_FOUND";
    pub const EXECUTION_FAILED: &str = "EXECUTION_FAILED";
    pub const VARIABLE_NOT_FOUND: &str = "VARIABLE_NOT_FOUND";
    pub const TYPE_MISMATCH: &str = "TYPE_MISMATCH";
    pub const UNKNOWN_ERROR: &str = "UNKNOWN_ERROR";
}

/// 创建缺少参数的错误
pub fn missing_parameter(param_name: &str, node_type: &str) -> NodeError {
    NodeError::new(
        format!("{} 节点缺少 {} 参数", node_type, param_name),
        error_codes::MISSING_PARAMETER,
        format!("请在 config 中添加 {} 参数", param_name),
    )
}

/// 创建参数无效的错误
pub fn invalid_parameter(param_name: &str, expected: &str, actual: &str) -> NodeError {
    NodeError::new(
        format!("参数 {} 无效: 期望 {}, 实际 {}", param_name, expected, actual),
        error_codes::INVALID_PARAMETER,
        format!("请检查 {} 参数的值", param_name),
    )
}

/// 创建超时错误
pub fn timeout(operation: &str, timeout_secs: u64) -> NodeError {
    NodeError::new(
        format!("{} 超时 ({}秒)", operation, timeout_secs),
        error_codes::TIMEOUT,
        "请检查网络连接或增加超时时间".to_string(),
    )
}

/// 创建网络错误
pub fn network_error(details: &str) -> NodeError {
    NodeError::new(
        format!("网络错误: {}", details),
        error_codes::NETWORK_ERROR,
        "请检查网络连接和目标地址".to_string(),
    )
}

/// 创建文件未找到错误
pub fn file_not_found(path: &str) -> NodeError {
    NodeError::new(
        format!("文件不存在: {}", path),
        error_codes::FILE_NOT_FOUND,
        "请检查文件路径是否正确".to_string(),
    )
}

/// 创建执行失败错误
pub fn execution_failed(operation: &str, details: &str) -> NodeError {
    NodeError::new(
        format!("{} 执行失败: {}", operation, details),
        error_codes::EXECUTION_FAILED,
        "请检查输入参数和操作是否正确".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_to_json() {
        let error = missing_parameter("url", "http");
        let json = error.to_json();
        
        assert_eq!(json["error"], "http 节点缺少 url 参数");
        assert_eq!(json["code"], "MISSING_PARAMETER");
        assert_eq!(json["suggestion"], "请在 config 中添加 url 参数");
    }

    #[test]
    fn test_error_helpers() {
        let missing = missing_parameter("command", "shell");
        assert_eq!(missing.code, "MISSING_PARAMETER");
        
        let invalid = invalid_parameter("port", "数字", "abc");
        assert_eq!(invalid.code, "INVALID_PARAMETER");
        
        let timeout_err = timeout("HTTP 请求", 30);
        assert_eq!(timeout_err.code, "TIMEOUT");
        
        let network = network_error("连接被拒绝");
        assert_eq!(network.code, "NETWORK_ERROR");
        
        let file = file_not_found("/tmp/test.txt");
        assert_eq!(file.code, "FILE_NOT_FOUND");
        
        let exec = execution_failed("Shell 命令", "权限不足");
        assert_eq!(exec.code, "EXECUTION_FAILED");
    }
}