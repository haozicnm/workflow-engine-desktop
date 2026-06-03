# AGENTS.md — Workflow Engine

## Purpose

Workflow Engine 是一个可视化工作流自动化引擎，由 Rust 后端 + Vue 3 前端组成。
目标：让用户通过拖拽节点构建自动化工作流，替代商业工具（影刀、n8n）。

## Project Overview

- **后端**：Rust (tokio async) + axum HTTP 服务器
- **前端**：Vue 3 + Vite + Pinia + TailwindCSS + shadcn-vue
- **数据库**：SQLite (rusqlite, WAL 模式)
- **脚本引擎**：Rhai (内置表达式求值)
- **浏览器自动化**：Playwright Python sidecar (39 种动作)
- **当前版本**：7.7.0

## Build & Test Commands

### 后端 (src-tauri/)

```bash
# 检查编译
cd src-tauri && cargo check --workspace

# Clippy 静态分析（CI 要求 -D warnings）
cd src-tauri && cargo clippy --workspace -- -D warnings

# 运行测试
cd src-tauri && cargo test --lib

# 构建 debug 版本
cd src-tauri && cargo build

# 构建 release 版本
cd src-tauri && cargo build --release
```

### 前端

```bash
# 安装依赖
npm install

# 开发服务器
npm run dev

# 类型检查 + 构建
npm run build

# 运行测试
npm test

# 测试 watch 模式
npm run test:watch
```

### CI

- CI 工作流改为**手动触发**（workflow_dispatch）
- 不要在 push/PR 时自动触发构建
- 手动触发时检查：`cargo clippy --workspace -- -D warnings` + `cargo test --lib`

## Architecture Boundaries

### 后端目录结构

```
src-tauri/src/
├── main.rs           # 入口，启动 HTTP 服务器
├── lib.rs            # 库入口
├── cli.rs            # CLI 参数解析（~2000行，注意不要过度扩展）
├── engine/           # 核心引擎
│   ├── executor.rs   # 步骤执行器
│   ├── parser.rs     # 工作流解析器
│   ├── registry.rs   # 节点注册表
│   ├── workflow.rs   # 工作流定义
│   ├── yaml_format.rs # YAML 导出
│   ├── template_manager.rs # 模板管理
│   ├── workflow_manager.rs # 工作流管理
│   └── skill_generator.rs  # Skill 生成器
├── nodes/            # 节点实现
│   ├── mod.rs        # 节点类型注册
│   ├── shell_node.rs # Shell 节点（无沙箱！）
│   ├── http_node.rs  # HTTP 节点
│   ├── mcp_node.rs   # MCP 协议节点
│   └── ...
├── server/           # HTTP 服务器
│   ├── routes.rs     # 路由定义
│   ├── handlers.rs   # 请求处理器
│   └── ...
├── data/             # 数据层
│   ├── models.rs     # 数据模型
│   └── database.rs   # SQLite 连接池
├── commands/         # Tauri 命令（桌面模式）
├── platform/         # 平台特定代码
└── system/           # 系统集成
```

### 前端目录结构

```
src/
├── App.vue           # 根组件
├── main.ts           # 入口
├── stores/           # Pinia 状态管理
├── components/       # UI 组件
│   ├── ui/           # shadcn-vue 组件
│   └── ...           # 业务组件
├── views/            # 页面视图
├── composables/      # Vue 组合式函数
├── types/            # TypeScript 类型
├── utils/            # 工具函数
├── locales/          # i18n 翻译
└── assets/           # 静态资源
```

### 关键约束

1. **前端零 any**：TypeScript 严格模式，不允许 `any` 类型
2. **后端 unwrap 警告**：生产代码避免 `unwrap()`，用 `?` 或 `.unwrap_or_default()`
3. **节点自描述**：所有节点必须在 `node-schema.json` 中定义 schema
4. **版本号统一**：`Cargo.toml` + `tauri.conf.json` + `package.json` 三处必须一致
5. **YAML 格式**：工作流导出使用 `FORMAT_VERSION="1.0"`

## Forbidden Actions

1. **不要修改 `config.yaml`**：这是用户配置文件，用 `config.yaml.template` 作为参考
2. **不要在 WSL 中编译 Windows 二进制**：推 GitHub 让 CI 构建
3. **不要自动触发 CI**：只推 commit，不触发 release 构建
4. **不要在 shell 节点中执行不可信代码**：shell 节点无沙箱，等同于直接执行
5. **不要硬编码 API key**：使用环境变量或配置文件
6. **不要修改 node_modules/ 或 dist/**：这些是生成目录

## Security Notes

1. **Shell 节点无沙箱**：执行任意命令，等同于用户直接操作终端
2. **默认绑定 0.0.0.0**：生产环境建议绑定 127.0.0.1
3. **API 无认证**：当前无鉴权机制，不建议暴露公网
4. **SQLite 文件**：存储工作流数据，注意备份

## Key Concepts

### 节点系统

- 82 种内置节点，分为 Rust 原生和 Python sidecar 两类
- 节点类型定义在 `node-schema.json`
- 节点通过结构化端口类型约束（`any/string/number/boolean/array/object`）
- 浏览器节点通过 Playwright Python sidecar 执行

### 工作流格式

```yaml
format_version: "1.0"
name: "工作流名称"
steps:
  - id: "step_1"
    type: "shell"
    config:
      command: "echo hello"
    outputs:
      result: "{{step_1.output}}"
```

### 变量系统

- `{{变量名}}` 语法引用变量
- 支持 JSONPath 表达式：`{{data.items[0].name}}`
- 变量作用域：全局变量 > 步骤输出 > 循环变量

### 模板系统

- 模板 = 工作流 + 锁定标记（🔒）
- 锁定的工作流不可编辑不可删除
- 模板存储在 `templates/` 目录

## API Endpoints

### 节点相关

- `GET /api/blocks` — 获取所有节点定义
- `GET /api/blocks?q=keyword` — 搜索节点
- `GET /api/blocks/categories` — 获取节点分类
- `GET /api/blocks/{type}` — 获取单个节点详情（含 action_definitions）

### 工作流相关

- `GET /api/workflows` — 列出所有工作流
- `POST /api/workflows` — 创建工作流
- `GET /api/workflows/{id}` — 获取工作流详情
- `PUT /api/workflows/{id}` — 更新工作流
- `DELETE /api/workflows/{id}` — 删除工作流（检查锁定）
- `POST /api/workflows/{id}/run` — 运行工作流
- `GET /api/workflows/{id}/export-yaml` — 导出 YAML

### 模板相关

- `GET /api/templates` — 列出所有模板
- `GET /api/templates?q=keyword&category=xxx` — 搜索模板
- `POST /api/templates` — 创建模板
- `POST /api/templates/import` — 导入模板

## Agent Integration

### Skill 文档

- `workflow-engine-agent` Skill 文档指导 Agent 如何使用本项目
- Agent 通过 API 发现节点 → 理解参数 → 组合工作流 → 生成 YAML

### 积木哲学

- 节点 = 积木，封装内部实现，暴露清晰接口
- 用户拖 UI，Agent 写 YAML，同一套积木，两条路径
- 设计原则：能用 shell 做的就不留专用节点

## Development Workflow

### 添加新节点

1. 在 `node-schema.json` 中定义节点 schema
2. 在 `src-tauri/src/nodes/` 中实现节点逻辑
3. 在 `registry.rs` 中注册节点
4. 更新 `docs/node-io-spec.md`
5. 运行 `cargo test --lib` 验证

### 修改工作流格式

1. 更新 `workflow.rs` 中的 `FORMAT_VERSION`
2. 更新 `yaml_format.rs` 导出器
3. 更新 `parser.rs` 解析器
4. 确保向后兼容

### 发布流程

1. 本地修改 + 测试
2. 只推 commit（不触发 CI）
3. 多个 commit 打包成一个版本
4. 手动触发 CI 构建
5. 打 tag 触发 release 构建

## Known Issues

1. **cli.rs 过大**（2000行）：需要拆分
2. **40处 unwrap()**：生产代码需要处理
3. **前端零测试**：需要补充
4. **安全审计缺失**：shell 节点无沙箱、API 无认证

## References

- `docs/ARCHITECTURE.md` — 详细架构文档
- `docs/node-io-spec.md` — 节点 I/O 规范
- `docs/HARNESS.md` — MCP Sidecar 开发 SOP
- `docs/workflow-engine-agent-skill.md` — Agent 集成文档
- `CHANGELOG.md` — 版本变更记录
- `node-schema.json` — 节点定义文件

## Conventions

### 代码风格

- **Rust**：遵循 `cargo clippy -- -D warnings`
- **TypeScript**：ESLint + Prettier
- **YAML**：2 空格缩进
- **Markdown**：中文文档，英文代码注释

### 命名规范

- 节点类型：`snake_case`（如 `shell_node`）
- API 路径：`kebab-case`（如 `/api/blocks`）
- 前端组件：`PascalCase`（如 `StepCard.vue`）
- 常量：`UPPER_SNAKE_CASE`

### Git 提交

- 使用 Conventional Commits 格式
- 类型：`feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`
- 中文描述，英文代码

## Decision Memory

当做出重要架构决策时，记录在 `docs/decisions/` 目录：
- 文件名：`NNNN-简短描述.md`
- 格式：Context → Decision → Consequences

## Failure Memory

当遇到不应重复的失败时，记录在 `docs/failures/` 目录：
- 文件名：`NNNN-简短描述.md`
- 格式：What Failed → Why → Current Replacement → Agent Guidance
