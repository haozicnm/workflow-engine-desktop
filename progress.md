# 进度日志

## 2026-04-27 上午 (WSL)

- ✅ 完成代码审查，梳理架构和问题
- ✅ 完成录制→生成流程全部开发：
  - `recording_converter.rs` — 操作→YAML 转换引擎（含智能合并）
  - `desktop_recorder.py` — Windows 桌面录制 sidecar
  - `recording.rs` v2 — 双通道录制节点
  - `RecordingBar.vue` — 前端录制控制栏
  - 编辑器集成 + Tauri 命令注册
- ✅ Rust `cargo check` 0 errors 0 warnings
- ✅ Rust `cargo build --release` 编译成功
- ✅ 前端 `npm run build` 成功 (138 modules, 1.75s)
- ⏳ Windows 构建待用户回家执行
- 📝 创建 task_plan.md / findings.md / progress.md

## 2026-04-28 上午 (WSL)

- ✅ v1.3 错误恢复策略 (on_error: fail / ignore / branch)
  - `workflow.rs`: 新增 ErrorStrategy 枚举 + Step.on_error 字段
  - `scheduler.rs`: 错误时根据策略处理（ignore 继续 / branch 跳转 / fail 终止）
  - `executor.rs`: 传递 on_error 字段
  - 19/20 tests pass（1 个失败是已有的 map_node 模板解析 bug）
- ✅ v1.3 审批超时 fallback (timeout_action: approve / reject / fail)
  - `approval.rs`: 超时时可按配置自动通过或拒绝，而非直接报错
- ✅ v1.3 变量实时监视窗
  - 后端 `scheduler.rs`: 新增 `variable-update` 事件，每步执行后推送变量快照
  - 前端 `Editor.vue`: 监听 `variable-update`，执行时侧栏实时显示全局变量+步骤输出
- ✅ v1.3 步骤搜索/过滤（Ctrl+F）
  - `Editor.vue`: Ctrl+F 弹出搜索栏，搜索步骤名称/类型/ID/配置
  - `StepCanvas.vue`: 不匹配步骤自动淡化（opacity 0.25）
- ✅ v1.3 并发工作流限制（Semaphore）
  - `lib.rs`: App 新增 `run_semaphore`（默认 10 并发，通过 `MAX_CONCURRENT_WORKFLOWS` 环境变量配置）
  - `run.rs`: `run_start` 获取 permit，超限直接返回错误
