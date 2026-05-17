# C 阶段：CLI↔桌面打通 — 统一控制面设计

## 目标

```
cron → wf-cli → IPC → 桌面应用(daemon) → 引擎执行
                                    ↓
                             前端实时可见（步骤、变量、可暂停/调参）
```

**桌面应用是唯一执行引擎，wf-cli 是它的 thin client。**

---

## 一、现状 vs 目标

| 维度 | 现状 | 目标 |
|------|------|------|
| 执行入口 | CLI 自己跑引擎（app_handle=None） | CLI 发给桌面，桌面统一执行 |
| 前端可见性 | CLI 执行对前端完全不可见 | 所有执行前端实时可见 |
| 人参与 | 无 | 暂停/跳过/修改变量/重试单个步骤 |
| 定时调度 | 只有 DB 记录，无触发器 | 桌面内置 cron daemon 或对接系统 cron |
| 进程模型 | 桌面关了就没了 | 托盘常驻，后台持续运行 |

---

## 二、IPC 选型：WebSocket (localhost)

| 方案 | 跨平台 | 复杂度 | 结论 |
|------|--------|--------|------|
| Unix socket | Linux/macOS only | 低 | ❌ 不支持 Windows |
| Named pipe | Windows only | 中 | ❌ 不跨平台 |
| stdin/stdout | 全平台 | 低但僵化 | ❌ 只能一对一，不支持多客户端 |
| **WebSocket** | 全平台 | 中 | ✅ 支持多客户端、事件推送、断线重连 |

**选择：localhost WebSocket，固定端口 19527 + 随机 token 认证。**

- 桌面启动时生成随机 token，写入 `~/.hermes/daemon-token`
- wf-cli 读取 token，连接 ws://127.0.0.1:19527
- 所有通信 JSON 格式

---

## 三、消息协议

### 3.1 wf-cli → 桌面

```jsonc
// 运行工作流
{"type": "run", "id": "req-1", "workflow_id": "xxx", "vars": {"key": "value"}}

// 运行模板
{"type": "library_run", "id": "req-2", "template": "daily-monitor", "params": {"source": "..."}}

// 查询状态
{"type": "status", "id": "req-3", "run_id": "xxx"}

// 列出运行中
{"type": "list_runs", "id": "req-4"}

// 控制运行中的工作流
{"type": "control", "id": "req-5", "run_id": "xxx", "action": "pause|resume|cancel"}
```

### 3.2 桌面 → wf-cli

```jsonc
// 请求确认
{"type": "ack", "id": "req-1", "run_id": "new-run-id"}

// 步骤更新（实时推送）
{"type": "step_update", "run_id": "xxx", "step_id": "step_1", "status": "running|completed|failed|skipped"}

// 变量快照
{"type": "var_snapshot", "run_id": "xxx", "variables": {}, "step_outputs": {}}

// 运行结束
{"type": "run_complete", "run_id": "xxx", "status": "completed|failed", "elapsed_secs": 12.5, "error": null}

// 错误
{"type": "error", "id": "req-1", "message": "工作流不存在"}
```

---

## 四、实现步骤

### C1：桌面 WebSocket Server（Rust 侧）

1. 添加 `tokio-tungstenite` 依赖
2. 在 Tauri `setup()` 中启动 WebSocket 监听 `127.0.0.1:19527`
3. 生成随机 token → 写入 `~/.hermes/daemon-token`
4. 实现协议处理：
   - `run` → 调用现有 scheduler，实时推送 step_update
   - `library_run` → 加载模板 → 参数替换 → 调用 run
   - `status` → 查询 DB 或内存状态
   - `control` → 通过 RunControl 暂停/恢复/取消

### C2：wf-cli 改为 remote client

1. wf-cli 新增全局 flag `--remote`（默认 true）
2. 连接前检查 daemon 是否存活（读 token + ping）
3. 如果 daemon 未启动，尝试 `wf-cli daemon start` 或回退到本地执行
4. `wf-cli run` → 建立 WS 连接 → 发送 run 请求 → 流式打印步骤状态
5. `wf-cli library run` → 同上
6. `wf-cli status` → 远程查询
7. `wf-cli daemon status|start|stop` → 管理后台进程

### C3：桌面 Tray Daemon 模式

1. 关闭窗口 → 隐藏到托盘，不退出进程
2. 托盘菜单：显示/隐藏窗口、暂停所有运行、退出
3. WebSocket server 在托盘模式下持续运行
4. 开机自启（可选）

### C4：调度触发器实现

当前 `wf-cli library schedule` 只写入 DB，没有触发器。两种方案：

**方案 A：系统 cron 触发 wf-cli**
- 简单，但跨平台配置麻烦
- Windows Task Scheduler / Linux cron / macOS launchd

**方案 B：桌面应用内置 cron daemon**
- 启动时加载 DB 中的 schedules
- 用 tokio timer 触发
- 跨平台一致
- 可以暂停/启用/禁用

**推荐方案 B**，workload 小，已经在依赖中。

### C5：端到端验证

1. 创建模板 → library schedule → 等待 cron tick → 前端看到执行
2. wf-cli run → 前端实时看到步骤进度
3. 前端暂停 → wf-cli 收到 paused 状态
4. 关闭窗口 → 托盘继续运行 → 重新打开窗口看到最新状态

---

## 五、依赖新增

```toml
tokio-tungstenite = "0.24"    # WebSocket server/client
```

---

## 六、不在此阶段的

- 模板组合/编排（`daily-routine` 调用子模板） → D 阶段
- 执行历史查询 UI → D 阶段
- 多桌面实例协作 → 远期
- 远程监控（非 localhost） → 远期

---

## 七、风险

- WebSocket 在 localhost 上性能足够，但需处理断线重连
- 桌面进程崩溃 → wf-cli 需有超时和重试
- token 文件权限需限制为 600
