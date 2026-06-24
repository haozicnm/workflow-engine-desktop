// server/auth.rs — 可选的 Bearer token 认证中间件
//
// 当 WORKFLOW_ENGINE_TOKEN 环境变量被设置时：
//   - POST / PUT / DELETE 请求需要 `Authorization: Bearer <token>` 头
//   - GET 请求免认证（只读安全）
// 当环境变量未设置时：所有请求放行（默认行为，向后兼容）

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

/// 从环境变量读取的认证令牌（空=认证关闭）
fn expected_token() -> Option<String> {
    std::env::var("WORKFLOW_ENGINE_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
}

/// 写操作需要认证的方法
fn is_write_method(method: &axum::http::Method) -> bool {
    method == axum::http::Method::POST
        || method == axum::http::Method::PUT
        || method == axum::http::Method::DELETE
        || method == axum::http::Method::PATCH
}

/// 认证中间件：仅在 token 被设置时对写操作检查 Bearer 令牌
pub async fn auth_middleware(request: Request, next: Next) -> Result<Response, (StatusCode, String)> {
    let token = match expected_token() {
        Some(t) => t,
        None => return Ok(next.run(request).await), // 未配置 token，跳过
    };

    // GET/HEAD 等只读请求放行
    if !is_write_method(request.method()) {
        return Ok(next.run(request).await);
    }

    // 检查 Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // 常量时间比较（防止时间侧信道攻击）
    let expected = format!("Bearer {}", token);
    if constant_time_eq(auth_header.as_bytes(), expected.as_bytes()) {
        Ok(next.run(request).await)
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            r#"{"error":"Unauthorized: missing or invalid Bearer token. Set Authorization: Bearer <WORKFLOW_ENGINE_TOKEN>"}"#.to_string(),
        ))
    }
}

/// 常量时间比较（不提前返回，防止时间侧信道）
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        // 长度不同也要比较完整内容，避免泄露长度信息
        let _ = a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y));
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
