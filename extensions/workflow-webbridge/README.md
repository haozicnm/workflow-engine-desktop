# Workflow WebBridge

浏览器自动化桥接 — 为 Workflow Engine 提供 CDP 控制能力。

## 架构

```
workflow-engine  ←  WebSocket :19527/ws/browser  ←  Extension  →  CDP  →  浏览器
```

**零配置** — 扩展自动连接 workflow-engine，不需要 Native Messaging。

## 安装

1. 打开 `chrome://extensions/`
2. 开启「开发者模式」
3. 点击「加载已解压的扩展程序」
4. 选择 `extensions/workflow-webbridge/` 目录
5. 完成 ✓

## 使用

扩展启动后自动连接 `ws://127.0.0.1:19527/ws/browser`。

workflow-engine 通过 WebSocket 发送 JSON 命令：

```json
{"id": "1", "action": "navigate", "params": {"url": "https://example.com"}}
```

扩展返回结果：

```json
{"id": "1", "success": true, "data": {"success": true, "url": "https://example.com", "tabId": 123}}
```

## 可用工具（18 个，与 Kimi WebBridge 完全兼容）

| 工具 | 说明 | 参数 |
|------|------|------|
| `navigate` | 导航到 URL | `url`, `newTab?` |
| `snapshot` | 获取页面元素树 + @eN ref | — |
| `click` | 点击元素 | `selector`（CSS 或 @eN） |
| `fill` | 填写表单 | `selector`, `value` |
| `evaluate` | 执行 JavaScript | `code` |
| `screenshot` | 截图 | `format?`, `quality?` |
| `save_as_pdf` | 保存为 PDF | — |
| `mouse_click` | 鼠标点击 | `selector` 或 `x,y` |
| `key_type` | 键盘输入 | `text`, `delay?` |
| `send_keys` | 发送按键 | `key`, `modifiers?` |
| `find_tab` | 查找标签页 | `url?`, `title?`, `index?` |
| `list_tabs` | 列出标签页 | — |
| `close_tab` | 关闭标签页 | `tabId?` |
| `close_session` | 关闭会话 | — |
| `network` | 网络监听 | `action`（enable/disable） |
| `upload` | 上传文件 | `selector`, `filePaths` |
| `cdp` | 直接 CDP 命令 | `method`, `params?` |
| `download` | 下载文件 | `url?`, `selector?`, `saveAs?` |

## Popup 面板

点击扩展图标查看：
- **连接状态** — 绿色圆点 = 已连接，红色 = 未连接
- **运行状态** — 版本号、活跃标签 ID、CDP 附加数、会话列表
- **标签页列表** — 当前浏览器打开的标签页概览
- **端口配置** — 可修改 WebSocket 端口（默认 19529）

## @eN ref 系统

```bash
# 1. snapshot 获取元素树（自动标记交互元素）
→ {"action": "snapshot"}
← {"tree": [...], "refs": {"e1": {"role": "link", "name": "Learn more"}, ...}}

# 2. 用 @eN 点击
→ {"action": "click", "params": {"selector": "@e1"}}

# 3. 用 @eN 填写
→ {"action": "fill", "params": {"selector": "@e3", "value": "hello"}}
```

## 与 Kimi WebBridge 的区别

| | Kimi WebBridge | Workflow WebBridge |
|---|---|---|
| 通信 | HTTP :10086 + Native Messaging | WebSocket :19527 |
| 安装 | 需要装 native host | 只装扩展 |
| 依赖 | Kimi 服务端 | 无 |
| 功能 | 17 个工具 | 17 个工具（完全兼容） |
| @eN ref | ✅ | ✅ |
