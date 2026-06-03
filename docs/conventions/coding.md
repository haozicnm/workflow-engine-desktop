# 编码约定

> 最后更新：YYYY-MM-DD

## 代码风格

### Rust

- 遵循 `cargo clippy -- -D warnings`
- 使用 `?` 操作符处理错误，避免 `unwrap()`
- 异步函数使用 `async/await`
- 错误类型使用 `thiserror` 定义

### TypeScript

- 使用 ESLint + Prettier
- 严格模式，不允许 `any` 类型
- 组件使用 Composition API
- 状态管理使用 Pinia

### YAML

- 2 空格缩进
- 字符串使用双引号
- 布尔值使用 `true`/`false`

## 命名规范

- 节点类型：`snake_case`（如 `shell_node`）
- API 路径：`kebab-case`（如 `/api/blocks`）
- 前端组件：`PascalCase`（如 `StepCard.vue`）
- 常量：`UPPER_SNAKE_CASE`
- 变量/函数：`camelCase`

## Git 提交

- 使用 Conventional Commits 格式
- 类型：`feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`
- 中文描述，英文代码
- 每个提交一个逻辑变更

## 文档

- 中文文档，英文代码注释
- Markdown 格式
- 重要决策记录在 `docs/decisions/`
- 失败记录在 `docs/failures/`

## 测试

- 后端：`cargo test --lib`
- 前端：`npm test`
- 新功能必须有测试
- Bug 修复必须有回归测试

## 安全

- 不要硬编码 API key
- 使用环境变量或配置文件
- Shell 节点无沙箱，谨慎使用
- 生产环境绑定 127.0.0.1
