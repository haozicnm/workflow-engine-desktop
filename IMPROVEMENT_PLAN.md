# v1.2.0 全面改进执行计划

## 概览
基于代码审查报告，6个阶段、23项技术债 + 易用性改进，全部执行。

## 阶段一：安全加固 🔴
- [ ] 1.1 Rhai 沙箱 (`nodes/script.rs`)
- [ ] 1.2 PowerShell/hotkey 白名单 (`nodes/mouse_keyboard.rs`, `nodes/window.rs`)
- [ ] 1.3 Python sidecar 命令注入防护 (`nodes/browser.rs`)
- [ ] 1.4 js-yaml 安全模式 (`stores/workflow.ts`)

## 阶段二：稳定性 🔴
- [ ] 2.1 unwrap 全面消除 (多文件)
- [ ] 2.2 竞态条件修复 (`engine/state.rs`, `engine/executor.rs`)
- [ ] 2.3 错误处理统一 anyhow (`commands/`, `nodes/`)
- [ ] 2.4 Python sidecar 健康检查 + 自动重启 (`nodes/browser.rs`)

## 阶段三：架构 🟡
- [ ] 3.1 Store 一拆三 (`stores/`)
- [ ] 3.2 StepConfigDialog 字段提取 (`config/node-fields.ts`)
- [ ] 3.3 describeCron 去重 (`utils/cron.ts`)
- [ ] 3.4 Dashboard 组件拆分 (子组件)
- [ ] 3.5 DB 连接池 / 迁移框架 (`data/db.rs`)
- [ ] 3.6 Rust clone 优化 (`engine/executor.rs`, `nodes/http.rs`)

## 阶段四：前端质量 🟡
- [ ] 4.1 TypeScript 类型安全 (`any` 消除)
- [ ] 4.2 事件监听清理 (`pages/Editor.vue`)
- [ ] 4.3 全局错误边界 (`App.vue`)
- [ ] 4.4 YAML 同步策略优化 (`pages/Editor.vue`)

## 阶段五：锦上添花 🟢
- [ ] 5.1 步骤列表虚拟滚动 (`components/StepCanvas.vue`)
- [ ] 5.2 单元测试（前端 Vitest + Rust #[cfg(test)]）
- [ ] 5.3 日志文件持久化 (tracing-subscriber)
- [ ] 5.4 模板测试数据清理
- [ ] 5.5 节点插件化声明机制

## 阶段六：易用性改进 🆕
- [ ] 6.1 YAML 语法高亮 (CodeMirror/Monaco)
- [ ] 6.2 全局变量编辑 UI
- [ ] 6.3 工作流搜索/过滤
- [ ] 6.4 步骤拖拽排序改进
- [ ] 6.5 快捷键增强 (Ctrl+D 复制步骤, Delete 删除)
- [ ] 6.6 自动保存草稿
- [ ] 6.7 撤销/重做
- [ ] 6.8 批量执行工作流
- [ ] 6.9 执行结果通知增强
- [ ] 6.10 暗色模式
