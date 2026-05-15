# 浏览器自动化加强 — 实施方案

> **Status:** 待审批 | **日期:** 2026-05-01

**目标:** 将浏览器节点从"一个通用下拉"拆为专用节点 + 交互式录制 + 元素拾取

**现有基础:**
- Python sidecar 已完成 30+ Playwright 动作（含录制：click/fill 捕获）
- Rust `recording_converter.rs` 已完成录制→工作流转换（686行）
- 前端 BrowserNode 只暴露了 7 个 action

---

## 阶段 1: 拆分专用节点（2-3h）

### 1.1 新增 LGraphNode 子类

在 `src/nodes/litegraph-nodes.ts` 添加以下节点类，每个继承 `WorkflowNode`，颜色用 `COLOR_DATA`：

| 节点 | type | 输入针脚 | 输出针脚 | widgets |
|------|------|---------|---------|---------|
| **Navigate** | `browser_navigate` | `url`(string) | `data`(object) | url(string), wait_until(combo: load/domcontentloaded/networkidle) |
| **Click** | `browser_click` | `selector`(string) | `data`(object) | selector(string) |
| **Fill** | `browser_fill` | `selector`(string), `value`(string) | `data`(object) | selector(string), value(string) |
| **Extract** | `browser_extract` | `selector`(string) | `data`(array) | selector(string), mode(combo: text/html/table/links/attribute), attribute(string, shown when mode=attribute) |
| **Screenshot** | `browser_screenshot` | — | `data`(object) | path(string), full_page(toggle) |
| **Evaluate** | `browser_evaluate` | `script`(string) | `result`(any) | script(text) |
| **Scroll** | `browser_scroll` | — | `data`(object) | direction(combo: bottom/top), times(number) |
| **Wait** | `browser_wait` | `selector`(string) | `data`(object) | selector(string), timeout_ms(number) |
| **PDF** | `browser_pdf` | — | `data`(object) | path(string) |

保留原 `browser` 节点作为万能兜底（用户选 `execute` 走自定义 action）。

### 1.2 注册

在 `registerAllNodes()` 注册所有新 type。

### 1.3 模板更新

更新 `templates/web-content-fetch.json` 使用新节点（navigate → extract 链式）。

---

## 阶段 2: 交互式录制（3-4h）

### 2.1 前端录制控制

在 `LiteGraphEditor.vue` 工具栏添加：
- **录制按钮** `🔴 录制` / `⏹ 停止`（toggle）
- 调用 `invoke('browser_recording_start')` / `invoke('browser_recording_stop')`
- 停止后获取操作列表→调用 Rust `convert_recording()` →生成 nodes/edges → `store.load()` 加载到画布

### 2.2 Rust 命令

在 `src-tauri/src/commands/` 新增：
```
browser_recording_start → sidecar.send_action("recording_start")
browser_recording_stop  → sidecar.send_action("recording_stop") → convert → return workflow JSON
```

录制转换器已有 (`recording_converter.rs`)，只需接上命令行。

### 2.3 录制过程中的视觉反馈

浏览器页面注入红色边框（`.wf-recording-border { outline: 2px dashed red }`）提示正在录制。

---

## 阶段 3: 浏览器元素拾取（4-5h）

### 3.1 Sidecar 新增 `pick` 模式

在 `playwright_driver.py` 新增 `_pick` 函数：

```python
async def _pick(params):
    """注入元素拾取 UI——hover 高亮，click 返回 selector"""
    page = _get_page()
    await page.evaluate("""
    () => {
        // 注入高亮样式
        const style = document.createElement('style');
        style.id = 'wf-picker-style';
        style.textContent = `
            .wf-pick-hover { outline: 2px solid #58a6ff !important; }
            .wf-pick-selected { outline: 2px solid #3fb950 !important; }
        `;
        document.head.appendChild(style);
        
        let hoverEl = null;
        document.addEventListener('mouseover', (e) => {
            if (hoverEl) hoverEl.classList.remove('wf-pick-hover');
            hoverEl = e.target;
            hoverEl.classList.add('wf-pick-hover');
            e.stopPropagation();
        }, true);
        
        document.addEventListener('click', (e) => {
            e.preventDefault();
            e.stopPropagation();
            // 生成智能选择器
            const sel = generateSelector(e.target);
            window.__wfPickResult = sel;
            // 清理
            style.remove();
            if (hoverEl) hoverEl.classList.remove('wf-pick-hover');
        }, true);
        
        function generateSelector(el) {
            // 复用已有的 selector 生成逻辑
            return el.id ? '#' + CSS.escape(el.id) : el.tagName.toLowerCase()
                + (el.className ? '.' + el.className.trim().split(/\\s+/)[0] : '');
        }
    }
    """)
    # 轮询等待结果
    for _ in range(300):  # 30 秒超时
        await asyncio.sleep(0.1)
        result = await page.evaluate("() => window.__wfPickResult")
        if result:
            return {"success": True, "data": {"selector": result}}
    return {"success": False, "error": "拾取超时"}
```

### 3.2 前端拾取按钮

在 `LiteGraphEditor.vue` 工具栏添加 `🎯 拾取` 按钮：
- 点击→调用 `invoke('browser_pick_element')`
- 返回 selector → 填入当前选中节点的 selector widget
- 或者在画布空白处长按右键弹出"拾取元素"选项

### 3.3 Rust 命令

```
browser_pick_element → sidecar.send_action("pick") → return selector string
```

---

## 实施顺序

```
阶段 1（拆分节点）→ 阶段 2（录制）→ 阶段 3（拾取）
```

每个阶段独立可测，不互相阻塞。

---

## 风险 & 注意事项

1. **录制与 Tauri 的 WebView 冲突** — 录制需要在 Playwright 控制的浏览器中操作，不能在编辑器自身的 WebView 中。需在 sidecar 的浏览器窗口操作，编辑器只发指令。
2. **元素拾取的 selector 质量** — 生成的 CSS selector 可能在页面刷新后失效，回退到 XPath 或 data-* 属性更稳定。
3. **录制转换的 YAML vs JSON** — `recording_converter.rs` 输出 YAML（v1.x 格式），需改为输出 JSON（v2.0 格式）以兼容 LiteGraph 编辑器。
