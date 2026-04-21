// engine/error.rs — 错误类型
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("执行错误: {0}")]
    ExecutionError(String),

    #[error("超时: {0}")]
    TimeoutError(String),

    #[error("节点 {node_id} 失败: {reason}")]
    NodeFailed { node_id: String, reason: String },

    #[error("数据层错误: {0}")]
    DataError(String),
}
