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

/// 判断请求来源是否是本机（应用自身 WebView / 浏览器 dev 模式）。
///
/// HTTP 服务只监听 127.0.0.1，但**浏览器里的任意网页**同样能访问 loopback，
/// 所以 "只监听本机" 不等于 "只有本应用能访问"。这里用于：
///   1. CORS 响应头只回给本地来源（阻断跨站读取工作流/配置）
///   2. 拒绝来自外部 Origin 的写请求（阻断跨站 POST /api/runs 之类）
///
/// 非浏览器客户端（CLI、webhook 调用方、curl）不发送 Origin，不受影响。
pub fn is_local_origin(origin: &axum::http::HeaderValue) -> bool {
    let Ok(o) = origin.to_str() else { return false };
    let o = o.trim();
    // 本机开发服务器（端口不固定）
    if let Some(rest) = o.strip_prefix("http://") {
        if let Some(host) = rest.split('/').next() {
            if let Some(port) = host.strip_prefix("localhost:") {
                return port.chars().all(|c| c.is_ascii_digit());
            }
            if let Some(port) = host.strip_prefix("127.0.0.1:") {
                return port.chars().all(|c| c.is_ascii_digit());
            }
        }
    }
    // Tauri 2 WebView 来源
    o == "tauri://localhost"
        || o.starts_with("http://tauri.localhost")
        || o.starts_with("https://tauri.localhost")
}

/// 认证中间件
///
/// 1. 跨站写请求防护（无论是否配置 token 都生效）
/// 2. 配置了 WORKFLOW_ENGINE_TOKEN 时，写操作还需 `Authorization: Bearer <token>`
pub async fn auth_middleware(request: Request, next: Next) -> Result<Response, (StatusCode, String)> {
    // ── 1. 跨站写请求防护 ──
    // 之前完全没有任何 Origin 校验，配合 CorsLayer::permissive()，
    // 用户访问的任意网页都可以 fetch 到 loopback 上的 API。
    if is_write_method(request.method()) {
        if let Some(origin) = request.headers().get(header::ORIGIN) {
            if !is_local_origin(origin) {
                return Err((
                    StatusCode::FORBIDDEN,
                    r#"{"error":"Forbidden: cross-origin write requests are not allowed"}"#.to_string(),
                ));
            }
        }
    }

    // ── 2. Bearer token（可选） ──
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
        // 长度不同直接判否；内容比较已无意义，但保持不提前返回的写法以免误导
        let _ = a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y));
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn hv(s: &str) -> HeaderValue {
        HeaderValue::from_str(s).unwrap()
    }

    #[test]
    fn local_origins_are_allowed() {
        assert!(is_local_origin(&hv("http://localhost:1420")));
        assert!(is_local_origin(&hv("http://localhost:5173")));
        assert!(is_local_origin(&hv("http://127.0.0.1:19529")));
        assert!(is_local_origin(&hv("tauri://localhost")));
        assert!(is_local_origin(&hv("http://tauri.localhost")));
    }

    #[test]
    fn foreign_origins_are_rejected() {
        // 这些是跨站读取/触发的来源，必须拒绝
        assert!(!is_local_origin(&hv("https://evil.example")));
        assert!(!is_local_origin(&hv("http://localhost.evil.example")));
        assert!(!is_local_origin(&hv("http://127.0.0.1.evil.example")));
        assert!(!is_local_origin(&hv("null")));
        assert!(!is_local_origin(&hv("http://localhost"))); // 无端口 → 非 dev 服务器
        assert!(!is_local_origin(&hv("https://localhost:1420")));
    }
}
