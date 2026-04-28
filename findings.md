# 研究发现

> 项目：Workflow Engine Desktop

## 代码审查发现 (2026-04-27)

### 架构优点
- Rust trait + registry 模式实现节点插件化，扩展性好
- AtomicBool + CancellationToken 双重取消机制
- 断点/单步/变量快照调试能力完善
- 变量模板替换支持嵌套路径 `{{a.b.c}}` 和步骤引用
- 浏览器 sidecar 有健康检查 + 自动重启
- WAL 模式 + 连接池，Schema 版本迁移设计
- 审批节点用 oneshot channel 实现，有超时保护

### 代码问题
1. `scheduler.rs` 步骤查找 O(n)，大工作流应改 HashMap
2. 审批节点等待逻辑正确但 scheduler 中 emit 和 execute 之间无耦合
3. 桌面录制仅支持 Windows（合理，设计如此）
4. 前端 `step_test` 调用录制时跨会话共享通过全局 static 实现

### 架构依赖
- Tauri 2 (Rust GUI)
- Vue 3 + Vite 6 + TypeScript (前端)
- SQLite via rusqlite + r2d2 (数据)
- Rhai (表达式/脚本引擎)
- Playwright via Python sidecar (浏览器)
- calamine + rust_xlsxwriter (Excel)
- zip (Word 文档 XML 内嵌)

## 影刀RPA 对标分析 (2026-04-27)

| 影刀特性 | Workflow Engine 现状 | 差距 |
|----------|---------------------|------|
| 操作录制→生成流程 | ✅ v2.0 完成 | 对齐 |
| 元素捕获选择器 | ❌ 手动写 selector | 缺失 |
| AI 自然语言生成 | ❌ | 缺失 |
| 应用市场 | ❌ 仅有简单模板 | 缺失 |
| 变量实时监视 | ❌ 仅调试快照 | 需改进 |
| 学院/社区 | ❌ | 不适配（本地工具） |
| 企业权限/审计 | ❌ | 暂不需要 |
| 错误恢复策略 | 仅重试 | 需 ignore/branch |
| 通知渠道 | 仅系统 toast | 需扩展 |
