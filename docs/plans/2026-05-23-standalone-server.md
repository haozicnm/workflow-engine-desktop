# Workflow Engine 独立 Web Server 架构迁移方案

> **For Hermes:** 在分支 `feat/standalone-server` 上逐任务实施。

**目标：** 剥离 Tauri 桌面壳，改为 Rust 内嵌 HTTP Server + 浏览器 UI，完全兼容 UOS 20 ARM64。

**北极星：** wf-cli 单二进制 = 执行引擎 + Web UI + CLI，零系统依赖，静态链接。

**架构变更：**

```
BEFORE (Tauri)                    AFTER (Standalone)
┌──────────────────┐              ┌──────────────────┐
│  Tauri WebView    │              │  浏览器 (外部)     │
│  ┌────────────┐  │              │  http://localhost │
│  │ Vue 前端    │  │              │  ┌────────────┐  │
│  │ invoke()   │  │              │  │ Vue 前端    │  │
│  └─────┬──────┘  │              │  │ fetch()    │  │
│        │ IPC     │              │  └─────┬──────┘  │
│  ┌─────┴──────┐  │              │        │ HTTP    │
│  │ Tauri 桥    │  │    ──→      │  ┌─────┴──────┐  │
│  │ commands/  │  │              │  │ axum HTTP   │  │
│  └─────┬──────┘  │              │  │ Server      │  │
│        │         │              │  │ /api/*      │  │
│  ┌─────┴──────┐  │              │  │ / (静态文件)  │  │
│  │ Engine     │  │              │  └─────┬──────┘  │
│  │ 执行引擎    │  │              │        │         │
│  └────────────┘  │              │  ┌─────┴──────┐  │
│  ┌────────────┐  │              │  │ Engine     │  │
│  │ system/    │  │              │  │ 执行引擎    │  │
│  │ tray/      │  │              │  └────────────┘  │
│  │ scheduler  │  │              │                  │
│  └────────────┘  │              └──────────────────┘
└──────────────────┘
```

**技术选型：**

| 层 | Before | After |
|----|--------|-------|
| HTTP Server | Tauri IPC | **axum** 0.7 (tokio 原生) |
| 静态文件 | Tauri 内嵌 | **tower-http** serve_dir |
| 实时推送 | Tauri emit | **SSE (Server-Sent Events)** |
| 前端调用 | `safeInvoke` → Tauri invoke | `safeInvoke` → `fetch()` |
| 前端事件 | `safeListen` → Tauri listen | `safeListen` → EventSource (SSE) |

**Cargo 增删：**

```diff
- tauri = { version = "2", optional = true }
- tauri-build = { version = "2", optional = true }
- rfd, opener, enigo, rdev, arboard  # GUI only
+ axum = "0.7"
+ tower-http = { version = "0.5", features = ["fs"] }
+ tokio = { version = "1", features = [..., "signal"] }
```

---

## Phase 1: Rust 后端 — HTTP Server 核心

### Task 1.1: 开分支 + 清理 Cargo.toml

**目标：** 创建功能分支，移除 GUI 依赖，添加 axum

**文件：**
- 创建: `.git` 分支 `feat/standalone-server`
- 修改: `src-tauri/Cargo.toml`

**操作：**
```bash
git checkout -b feat/standalone-server
```

编辑 `Cargo.toml`:
```toml
# [features] 简化为
[features]
default = ["cli"]
cli = []

# 添加
axum = "0.7"
tower-http = { version = "0.5", features = ["fs", "cors"] }
```

删除: `tauri`, `tauri-build`, `rfd`, `opener`, `enigo`, `rdev`, `arboard`

### Task 1.2: 创建 server 模块骨架

**目标：** 启动 axum HTTP server，监听 `localhost:19127`

**文件：**
- 创建: `src-tauri/src/server/mod.rs`
- 创建: `src-tauri/src/server/routes.rs`

```rust
// server/mod.rs
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

pub async fn start(app: Arc<App>, port: u16) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let router = Router::new()
        .nest("/api", routes::api_router(app))
        .layer(CorsLayer::permissive());
    
    println!("Workflow Engine → http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
```

验证: `cargo check --bin wf-cli --no-default-features --features cli`

### Task 1.3: 创建 API 路由 — 替代 Tauri commands

**目标：** 把 `commands/` 下的 Tauri 命令转为 REST API

**文件：**
- 修改: `src-tauri/src/server/routes.rs`
- 创建: `src-tauri/src/server/handlers.rs`

**路由表：**

| 前端调用 (before) | API 端点 (after) | 方法 |
|---|---|---|
| `invoke("list_workflows")` | `/api/workflows` | GET |
| `invoke("get_workflow", {id})` | `/api/workflows/:id` | GET |
| `invoke("create_workflow", {...})` | `/api/workflows` | POST |
| `invoke("update_workflow", {id,...})` | `/api/workflows/:id` | PUT |
| `invoke("delete_workflow", {id})` | `/api/workflows/:id` | DELETE |
| `invoke("run_workflow", {id})` | `/api/workflows/:id/run` | POST |
| `invoke("run_status", {run_id})` | `/api/runs/:run_id` | GET |
| `invoke("list_runs")` | `/api/runs` | GET |
| `invoke("stop_run", {run_id})` | `/api/runs/:run_id/stop` | POST |
| `invoke("list_schedules")` | `/api/schedules` | GET |
| `invoke("create_schedule", {...})` | `/api/schedules` | POST |
| `invoke("list_library")` | `/api/library` | GET |
| `invoke("validate_workflow", {yaml})` | `/api/validate` | POST |
| `invoke("get_plugins")` | `/api/plugins` | GET |

每个 handler 直接调用现有 `commands/` 里的逻辑（去掉 `tauri::State` 参数，改为传 `Arc<App>`）。

### Task 1.4: SSE 事件流 — 替代 Tauri emit

**目标：** 前端能实时收到 `step_update`、`run_complete` 事件

**文件：**
- 创建: `src-tauri/src/server/sse.rs`

**设计：**
```rust
// 全局事件广播器（tokio::broadcast channel）
pub struct EventBus {
    tx: broadcast::Sender<Event>,
}

pub enum Event {
    StepUpdate { run_id: String, step_id: String, status: String },
    RunComplete { run_id: String, success: bool },
    VarSnapshot { run_id: String, vars: Value },
}
```

SSE 端点：`GET /api/events?run_id=xxx`
- 客户端连上后持续推送该 run 的事件
- 使用 `axum::response::sse::Sse` + `tokio_stream`

### Task 1.5: 静态文件服务 + SPA fallback

**目标：** 前端 build 产物由 Rust 直接 serve

**文件：**
- 修改: `src-tauri/src/server/mod.rs`

```rust
// 生产模式：serve 内置静态文件
let static_dir = std::env::var("WF_STATIC_DIR")
    .unwrap_or_else(|_| "dist".to_string());

let router = Router::new()
    .nest("/api", routes::api_router(app))
    .fallback_service(
        ServeDir::new(&static_dir)
            .fallback(ServeFile::new(format!("{}/index.html", static_dir)))
    );
```

### Task 1.6: 集成到 wf-cli

**目标：** `wf-cli serve` 启动完整服务

**文件：**
- 修改: `src-tauri/src/cli.rs` (添加 `serve` 子命令)
- 修改: `src-tauri/src/bin/wf-cli.rs`

```rust
// cli.rs 新增
#[derive(Subcommand)]
enum Commands {
    // ... 现有命令
    /// 启动 Web 界面
    Serve {
        #[arg(default_value = "19127")]
        port: u16,
        #[arg(long)]
        open: bool,  // 自动打开浏览器
    },
}
```

`serve` 命令逻辑：
1. `App::new()` 初始化
2. `server::start(app, port)` 启动 HTTP + SSE
3. 可选：`opener::open(format!("http://localhost:{port}"))`
4. 等待 Ctrl+C

---

## Phase 2: 前端 — Tauri invoke → HTTP fetch

### Task 2.1: 改造 safeInvoke

**目标：** `safeInvoke` 优先 HTTP，Tauri 作为 fallback

**文件：**
- 修改: `src/utils/tauri.ts`

```typescript
// 新增 HTTP 模式
const API_BASE = typeof window !== 'undefined' 
  ? (import.meta.env.VITE_API_BASE || 'http://localhost:19127/api')
  : '';

export async function safeInvoke<T = unknown>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T | undefined> {
  // HTTP 模式优先
  if (API_BASE) {
    const method = command.startsWith('get_') || command.startsWith('list_') 
      ? 'GET' : 'POST';
    const url = `${API_BASE}/${commandToPath(command, args)}`;
    const res = await fetch(url, {
      method,
      headers: { 'Content-Type': 'application/json' },
      body: method !== 'GET' ? JSON.stringify(args) : undefined,
    });
    return res.json();
  }
  // Tauri fallback（保留不想删除的逻辑）
  if (isTauri) { /* 原逻辑 */ }
  return undefined;
}

// 命令名 → 路由映射
function commandToPath(cmd: string, args?: Record<string, unknown>): string {
  const map: Record<string, string> = {
    list_workflows: 'workflows',
    get_workflow: `workflows/${args?.id}`,
    create_workflow: 'workflows',
    update_workflow: `workflows/${args?.id}`,
    delete_workflow: `workflows/${args?.id}`,
    run_workflow: `workflows/${args?.id}/run`,
    list_runs: 'runs',
    run_status: `runs/${args?.run_id}`,
    stop_run: `runs/${args?.run_id}/stop`,
    list_schedules: 'schedules',
    create_schedule: 'schedules',
    list_library: 'library',
    validate_workflow: 'validate',
  };
  return map[cmd] || cmd;
}
```

**验证：** `npm run build` 不报错

### Task 2.2: 改造 safeListen → SSE

**目标：** 用 EventSource 替代 Tauri event listener

**文件：**
- 修改: `src/utils/tauri.ts`

```typescript
export function safeListen<T>(
  event: string,
  handler: (event: Event & { data: string }) => void,
  runId?: string,
): () => void {
  const url = runId 
    ? `${API_BASE.replace('/api', '')}/api/events?run_id=${runId}`
    : `${API_BASE.replace('/api', '')}/api/events`;
  const es = new EventSource(url);
  es.addEventListener(event, handler as any);
  return () => es.close();
}
```

### Task 2.3: 移除 @tauri-apps/api 依赖

**目标：** 删除 `package.json` 中的 tauri 依赖

**文件：**
- 修改: `package.json` — 删除 `@tauri-apps/api`
- 修改: `vite.config.ts` — 删除 tauri 插件（如有）

**验证：** `npm install && npm run build` 成功

---

## Phase 3: 清理 Tauri 残骸

### Task 3.1: 删除 Tauri-only 模块

**目标：** 删除不再需要的文件

**文件：**
- 删除: `src-tauri/src/commands/` (Tauri commands → 逻辑已迁到 server/handlers)
- 删除: `src-tauri/src/system/tray.rs`
- 删除: `src-tauri/src/system/scheduler.rs` (调度器用 cron 替代)
- 删除: `src-tauri/src/platform/linux.rs`
- 删除: `src-tauri/src/platform/recording.rs`
- 删除: `src-tauri/src/nodes/clipboard.rs`, `clipboard_container.rs`
- 删除: `src-tauri/src/nodes/print.rs` (改用 tracing)
- 删除: `src-tauri/src/main.rs` (Tauri 入口 → 不再需要)
- 删除: `src-tauri/src/ipc.rs` (WebSocket IPC → SSE 替代)
- 删除: `src-tauri/tauri.conf.json`
- 删除: `src-tauri/icons/`
- 删除: `src-tauri/.cargo/config.toml` (如有 Tauri 特定配置)

### Task 3.2: 修复 module 引用

**目标：** lib.rs 和 mod.rs 中去掉已删除模块的声明

**文件：**
- 修改: `src-tauri/src/lib.rs` — 删除 `mod commands; mod system; mod ipc; mod platform;`
- 修改: `src-tauri/src/nodes/mod.rs` — 删除 clipboard 相关

**验证：** `cargo check --bin wf-cli`

### Task 3.3: App 结构体精简

**目标：** App 不再需要 `tauri::AppHandle` 和 `Emitter` 相关字段

**文件：**
- 修改: `src-tauri/src/lib.rs`

```rust
pub struct App {
    pub db: Arc<Database>,
    pub templates_dir: PathBuf,
    // 删除: app_handle（Tauri specific）
    // 新增:
    pub event_bus: EventBus,
    pub cancel_tokens: CancelTokens,
}
```

**验证：** `cargo check --bin wf-cli`

---

## Phase 4: 编译与测试

### Task 4.1: 本地编译验证

**目标：** wf-cli 在本机（WSL x86_64）编译通过，启动 HTTP server

**命令：**
```bash
cd src-tauri
cargo build --bin wf-cli --release
./target/release/wf-cli serve
curl http://localhost:19127/api/workflows
```

**预期：** 返回 JSON 数组（空或含内置工作流）

### Task 4.2: 前端构建 + 集成测试

**目标：** 前端 build 到 dist/，wf-cli serve 出完整 UI

**命令：**
```bash
npm run build
WF_STATIC_DIR=../dist ../src-tauri/target/release/wf-cli serve
# 浏览器打开 http://localhost:19127
```

**预期：** 看到 Dashboard 页面，工作流列表正常加载

### Task 4.3: GitHub Actions ARM64 编译

**目标：** 更新 workflow，编译去 Tauri 版 wf-cli

**文件：**
- 修改: `.github/workflows/build-arm64.yml`

```yaml
# 只需 cli job，不需要 desktop job
- name: Build wf-cli
  run: cargo build --bin wf-cli --release --no-default-features --features cli
```

### Task 4.4: Docker UOS 测试

**目标：** 在 UOS ARM64 容器里验证二进制

**步骤：**
1. 下载 artifact `wf-cli-arm64`
2. `docker cp wf-cli uos-build:/usr/local/bin/`
3. `docker exec uos-build wf-cli --version`
4. `docker exec uos-build wf-cli serve`（验证能启动）

---

## 实施顺序

```
Phase 1 (Rust 后端) ─────────────┐
  Task 1.1 → 1.2 → 1.3 → 1.4    │  后端先行，前端不阻塞
           → 1.5 → 1.6           │
                                  ├──→ 1.6 后才进入 Phase 2
Phase 2 (前端) ──────────────────┘
  Task 2.1 → 2.2 → 2.3

Phase 3 (清理) ─── 2.x 完成后
  Task 3.1 → 3.2 → 3.3

Phase 4 (测试) ─── 全部完成后
  Task 4.1 → 4.2 → 4.3 → 4.4
```
