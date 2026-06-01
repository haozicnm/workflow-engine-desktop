# Changelog

## v7.6.0 (2026-06-01)

### ⚠️ Breaking Changes

- **节点精简：78 → 36（-54%）** — 删除 42 个冗余节点，功能由 shell/script 替代
- 删除的节点类型：file_delete/append/mkdir/copy/move/glob、data_length/default/merge、json_parse、text_template、6 个类型转换、7 个数组操作、regex_replace、notify、3 个幽灵节点、10 个 MCP 重复节点
- 现有工作流如使用已删除节点，需改用 shell 节点等价命令

### ✨ New Features

- **积木自描述系统** — 36 个节点全部有 params schema，UI 自动渲染配置表单，Agent 读 schema 即可搭工作流
- **积木发现 API** — `GET /api/blocks` 列出所有积木，`GET /api/blocks/{type}` 查看详情
- **工作流组装 API** — `POST /api/workflows/assemble` Agent 提交 YAML 验证+保存
- **模板库** — 5 个预制模板（http-to-file、excel-pipeline、web-scrape、file-batch、shell-pipeline）
- **模板 API** — `GET /api/templates` / `POST /api/templates/{name}/instantiate`
- **组合助手** — `POST /api/compose/chain` 自动串联积木
- **前端自动渲染** — 配置表单根据 params schema field_type 自动选择组件

### 🔧 Improvements

- Shell timeout_secs 现在真正执行超时（`tokio::time::timeout`，上限 3600s）
- regex_extract + regex_match 合并为统一 regex 节点（mode 参数）
- WebBridge 成为唯一浏览器控制路径（移除 Playwright sidecar 和 Kimi Browser）
- Release 构建隐藏 Windows 控制台窗口

### 🐛 Bug Fixes

- Shell timeout_secs 参数未生效（子进程永不被 kill）
- 配置文件非原子写入（改为 write_to_temp → fsync → rename）
- 前端 vue-tsc 编译错误（ContainerType 缺少 logic）
- 端口绑定重试 3 次（处理僵尸 socket 延迟释放）
- 跨平台 Python 检测（Windows: python→python3→py）

### 🧹 Cleanup

- 移除 42 个冗余节点（MCP 重复、文件操作、数组操作、类型转换等）
- 清理后端/前端死代码引用（~300 行）
- 前端控制台移除 Playwright 残留
- 移除 browser_channel、browser_executable_path 配置项

### 📦 Dependencies

- 无变更

---

## v7.5.1 (2026-05-31)

- 修复版本号硬编码（7.3.0 → 7.5.0）
- CI clippy 修复（explicit_auto_deref）

## v7.5.0 (2026-05-30)

- WebBridge Chrome 扩展集成
- 浏览器自动化容器节点
- 工作流执行引擎优化
