# Workflow Engine GUI 问题诊断报告

**审查日期：** 2026-06-21  
**审查范围：** `gui.rs` + `tauri.conf.json` + `main.rs` + `lib.rs` + `tray.rs` + `tauri.ts` + `useNodeSchema.ts` + `App.vue`  
**构建状态：** `cargo build --bin workflow-engine-gui --features gui` 超时（180s），需更长时间

---

## 一、总体结论

GUI 版本存在 **3 个 🔴 会导致功能异常的问题**、**2 个 🟡 会导致体验下降的问题**、**2 个 🟢 建议优化**。最关键的问题是：**前端加载时后端 HTTP 服务器可能还没启动**（竞争条件），以及 `tauri.conf.json` 的资源路径配置错误。

---

## 二、🔴 会导致 GUI 功能异常的问题（3 个）

### 2.1 前端与后端 HTTP 服务器的竞争条件

| 项目 | 详情 |
|------|------|
| **位置** | `gui.rs:122-151`（HTTP 服务器启动）+ `App.vue:32-35`（`syncNodeSchema` 调用） |
| **问题** | `gui.rs` 在 `setup` 闭包的 `std::thread::spawn` 中启动 HTTP 服务器。`App.vue` 的 `onMounted` 在 WebView 加载完成后立即调用 `syncNodeSchema()`，向后端 `http://localhost:19529/api/nodes/schema` 请求。如果 WebView 加载快于 HTTP 服务器启动，`syncNodeSchema` 请求失败，前端显示 "0 节点"，画布无法使用。 |
| **影响** | 用户打开 GUI 后，左侧节点面板为空，无法添加任何节点 |
| **修复** | 在 `gui.rs` 中确保 HTTP 服务器完全启动后再继续；或在 `App.vue` 的 `syncNodeSchema` 中添加重试逻辑：

```typescript
// useNodeSchema.ts 修改
export async function syncNodeSchema(retryCount = 5): Promise<number> {
  if (_loaded) return _mergedDefs.length
  
  for (let attempt = 1; attempt <= retryCount; attempt++) {
    try {
      const resp = await fetch(`${API_BASE}/api/nodes/schema`)
      if (resp.ok) {
        const schema = await resp.json()
        if (schema?.nodes?.length) {
          // ... 正常处理
          return _mergedDefs.length
        }
      }
    } catch (e) {
      console.warn(`[NodeSchema] 尝试 ${attempt}/${retryCount} 失败，1秒后重试...`)
      if (attempt < retryCount) {
        await new Promise(r => setTimeout(r, 1000))
      }
    }
  }
  console.error('[NodeSchema] 所有重试失败，后端可能未启动')
  return 0
}
```

### 2.2 `tauri.conf.json` 资源路径错误

| 项目 | 详情 |
|------|------|
| **位置** | `tauri.conf.json:61-63` |
| **问题** | `bundle.resources` 中 `"../templates/data/**/*": "templates/data/"` 路径错误。`tauri.conf.json` 在 `src-tauri/` 目录中，`../templates/` 指向**项目根目录**的 `templates/`，但实际 `templates/` 在 `src-tauri/templates/` 目录中。 |
| **影响** | `cargo tauri build` 时资源文件找不到，构建失败；或运行时模板数据缺失 |
| **修复** | 改为 `"templates/data/**/*": "templates/data/"`（相对于 `src-tauri/` 目录）：

```json
"resources": {
    "sidecars/*": "sidecars/",
    "templates/data/**/*": "templates/data/"
}
```

### 2.3 前端 `fetch` 到 `localhost:19529` 的 CORS/CSP 限制

| 项目 | 详情 |
|------|------|
| **位置** | `tauri.conf.json:24`（CSP）+ `useNodeSchema.ts:18`（API_BASE） |
| **问题** | `tauri.conf.json` 的 `csp` 允许 `connect-src 'self' http: ...`，但某些 WebView 实现（特别是 Windows 上的 Edge WebView2）对 `http://localhost:19529` 的跨域请求仍有严格限制。如果 `csp` 中 `http:` 不被正确解析，前端 `fetch` 会被拒绝。 |
| **影响** | 前端无法连接到后端 HTTP 服务器，所有 API 调用失败 |
| **修复** | 将 `connect-src` 中的 `http:` 改为具体的 `http://localhost:19529`，或添加 `http://localhost:*`：

```json
"csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' http://localhost:19529 http://localhost:1420 http: https: ws: wss: tauri: ipc: asset: https://asset.localhost; img-src 'self' data: https: asset: https://asset.localhost; font-src 'self' data:"
```

---

## 三、🟡 会导致体验下降的问题（2 个）

### 3.1 构建超时

| 项目 | 详情 |
|------|------|
| **位置** | `cargo build --bin workflow-engine-gui --features gui` |
| **问题** | Tauri 2 的 GUI 构建包含 WebView2 绑定、Windows 系统库链接等，耗时远超纯 Rust 后端。180 秒超时不够完成构建。 |
| **影响** | 本地开发或 CI 构建时频繁超时 |
| **修复** | 本地构建增加超时：`cargo build --bin workflow-engine-gui --features gui` 预计需要 5-10 分钟。CI 中确保 GitHub Actions 的 job 超时（默认 6 小时，通常足够）。 |

### 3.2 HTTP 服务器启动失败时无用户提示

| 项目 | 详情 |
|------|------|
| **位置** | `gui.rs:146-149` |
| **问题** | 如果 `127.0.0.1:19529` 被占用，HTTP 服务器启动失败，只记录 `warn` 日志，没有弹出 Toast 或对话框告知用户。前端会一直尝试连接但失败。 |
| **影响** | 用户看到空白或部分功能不可用的界面，不知道原因 |
| **修复** | 在 `setup` 中添加端口占用检测，如果失败则弹出 Tauri 对话框：

```rust
// gui.rs 修改
use tauri::Manager;

if let Err(e) = tokio::net::TcpListener::bind(bind_addr).await {
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let _ = app_handle
            .dialog()
            .message(format!("HTTP 端口 {} 被占用，部分功能可能不可用。请关闭其他 Workflow Engine 实例后重试。", bind_addr.port()))
            .title("启动警告")
            .show();
    });
}
```

---

## 四、🟢 建议优化（2 个）

### 4.1 `tauri.conf.json` 的 `bundle.targets` 配置与 `release.yml` 不一致

| 项目 | 详情 |
|------|------|
| **位置** | `tauri.conf.json:29-33` + `release.yml:323-386` |
| **问题** | `tauri.conf.json` 配置了 `appimage` + `deb` + `nsis` 三种打包目标，但 `release.yml` 使用的是手动复制文件 + Inno Setup 脚本。`nsis` 和 `nsis` 是两种不同的 Windows 安装程序格式。 |
| **建议** | 统一使用 `cargo tauri build`（Tauri 自带的打包）替代 `release.yml` 中的手动打包。或者从 `tauri.conf.json` 中移除 `nsis`，只保留 `appimage` + `deb`。 |

### 4.2 `tauri.conf.json` 中 `version` 字段冗余

| 项目 | 详情 |
|------|------|
| **位置** | `tauri.conf.json:3` |
| **问题** | Tauri 2 的 `generate_context!()` 宏会优先读取 `Cargo.toml` 的版本号。`tauri.conf.json` 中的 `version` 字段是冗余的，如果两者不一致，可能导致构建警告。 |
| **建议** | 将 `tauri.conf.json` 的 `version` 改为 `"0.0.0"` 或 `"0.0.1"`，明确声明这是占位符，实际版本由 `Cargo.toml` 控制。或者移除 `version` 字段（Tauri 2 允许从 `Cargo.toml` 自动读取）。 |

---

## 五、其他观察

### 5.1 前端 `API_BASE` 检测逻辑正确

`useNodeSchema.ts` 中的 `(window as any).__TAURI_INTERNALS__` 检测在 Tauri 2 中有效。`API_BASE` 在 Tauri 模式下为 `http://localhost:19529`，在浏览器模式下为 `http://localhost:19528`。逻辑正确。

### 5.2 `gui.rs` 的 `static_dir` 自动定位正确

`gui.rs` 的 `static_dir` 逻辑优先检查 exe 同目录的 `dist/`，然后检查 `../../dist/`（开发模式）。这确保了开发模式和生产模式都能正确找到前端资源。

### 5.3 `tray.rs` 系统托盘实现正确

`TrayIconBuilder` 配置正确，菜单项和双击事件处理完整。`get_webview_window("main")` 能找到默认窗口（第一个窗口的 label 默认为 `"main"`）。

### 5.4 `safeInvoke` 命令映射完整

`tauri.ts` 中的 `FIXED_ROUTES` 和 `DYNAMIC_ROUTES` 覆盖了 `gui.rs` `invoke_handler` 中注册的所有命令。前后端命令名一致。

### 5.5 `gui.rs` 的 `app_for_http` clone 安全

`App` 实现了 `#[derive(Clone)]`，所有字段都是 `Arc` 类型。`clone()` 是浅拷贝，共享底层数据，线程安全。

---

## 六、修复优先级清单

### 🔴 立即修复（影响功能）

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 1 | 前端与后端竞争条件 | `useNodeSchema.ts` | 添加 `syncNodeSchema` 重试逻辑（最多 5 次，间隔 1 秒） |
| 2 | `tauri.conf.json` 资源路径错误 | `tauri.conf.json` | `"../templates/data/**/*"` → `"templates/data/**/*"` |
| 3 | CSP 限制 `fetch` | `tauri.conf.json` | `connect-src` 添加 `http://localhost:19529` |

### 🟡 短期修复（影响体验）

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 4 | 构建超时 | — | 本地构建增加超时到 10 分钟；CI 保持默认 6 小时 |
| 5 | HTTP 启动失败无提示 | `gui.rs` | 端口占用时弹出 Tauri 对话框 |

### 🟢 建议优化

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 6 | `bundle.targets` 与 `release.yml` 不一致 | `tauri.conf.json` | 移除 `nsis`，或统一使用 `cargo tauri build` |
| 7 | `version` 字段冗余 | `tauri.conf.json` | 改为 `"0.0.0"` 占位符 |

---

## 七、调试建议

如果 GUI 打开后功能异常，按以下顺序排查：

1. **检查浏览器控制台（F12）**
   - 是否有 `fetch` 到 `localhost:19529` 失败的错误？
   - 是否有 `[NodeSchema] 加载失败` 的警告？
   - 是否有 CSP 相关的错误？

2. **检查后端日志**
   - `%APPDATA%/workflow-engine/logs/`（Windows）或 `~/.local/share/workflow-engine/logs/`（Linux）
   - 搜索 `HTTP 端口` 或 `绑定失败`

3. **检查端口占用**
   - Windows: `netstat -ano | findstr 19529`
   - Linux: `lsof -i :19529`

4. **检查 `dist/` 目录**
   - 确保 exe 同目录有 `dist/index.html`
   - 确保 `dist/` 中有完整的前端构建产物

5. **检查 `tauri.conf.json` 版本**
   - 确认与 `Cargo.toml` 一致

---

*审查方法：代码走读（gui.rs 163 行 + tauri.conf.json 68 行 + tray.rs 58 行 + tauri.ts 417 行 + useNodeSchema.ts 151 行 + App.vue 386 行）+ 编译验证 + 构建时序分析*
