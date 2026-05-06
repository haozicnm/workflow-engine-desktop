#!/usr/bin/env python3
"""Playwright Sidecar — 浏览器自动化驱动

通过 stdin/stdout JSON 协议与 Rust 核心通信。
每行一个 JSON 对象（NDJSON 协议）。

请求 → {"id":"uuid","action":"navigate","params":{"url":"https://example.com"}}
响应 ← {"id":"uuid","success":true,"data":{"title":"Example","url":"..."}}

支持的操作:
  launch              — 启动浏览器 (params: headless?, browser?, channel?)
  navigate            — 导航到 URL (params: url, wait_until?)
  click               — 点击元素 (params: selector, wait_ms?)
  fill                — 填写表单 (params: selector, value)
  text                — 获取单个元素文本 (params: selector)
  attr                — 获取单个元素属性 (params: selector, attribute)
  screenshot          — 截图 (params: path, full_page?)
  evaluate            — 执行 JS (params: script)
  wait                — 等待元素 (params: selector, timeout_ms?)
  select              — 下拉选择 (params: selector, value)
  check               — 勾选复选框 (params: selector)
  pick                — 元素选择器 (hover 高亮，点击返回最佳 CSS 选择器)
  close               — 关闭浏览器

  === 新增动作 (v1.1) ===
  extract_text        — 批量提取元素文本 (params: selector) → [str]
  extract_html        — 批量提取元素 HTML (params: selector) → [str]
  extract_table       — 提取表格为二维数组 (params: selector?) → [[str]]
  extract_links       — 提取页面所有链接 (params: selector?) → [{text,href}]
  extract_attribute   — 批量提取元素属性 (params: selector, attribute) → [str]
  scroll_to           — 滚动页面 (params: to="bottom", times?, delay_ms?)
  pdf                 — 生成 PDF (params: path)
  cookies             — Cookie 管理 (params: action="get"|"set"|"clear", cookies?)
  set_headers         — 设置额外 HTTP 头 (params: headers)
  new_page            — 新建页面标签 (params: url?) → {page_index}
  close_page          — 关闭页面标签 (params: index?)
  switch_page         — 切换页面标签 (params: index)
  pages               — 列出所有页面标签 → [{index, url, title}]
  back                — 后退
  forward             — 前进
  reload              — 刷新页面
  current_url         — 获取当前 URL → {url}
  get_title           — 获取页面标题 → {title}
"""
import sys
import os
import json
import asyncio
import shutil
import traceback
import random
import base64

try:
    from playwright.async_api import async_playwright
except ImportError:
    async_playwright = None

# 全局浏览器状态
_context = None  # PersistentContext（launch_persistent_context，管理 cookies/登录态/headers）
_pages = []      # 所有打开的页面
_current_page_idx = 0
_playwright = None
_extra_headers = {}


def _detect_browser_channel() -> str:
    """自动检测系统可用的 Chromium 内核浏览器"""
    if sys.platform == "win32":
        edge_paths = [
            os.path.join(os.environ.get("PROGRAMFILES(X86)", ""), "Microsoft", "Edge", "Application", "msedge.exe"),
            os.path.join(os.environ.get("PROGRAMFILES", ""), "Microsoft", "Edge", "Application", "msedge.exe"),
        ]
        for p in edge_paths:
            if os.path.isfile(p):
                return "msedge"

    if shutil.which("msedge") or shutil.which("microsoft-edge"):
        return "msedge"
    if shutil.which("chrome") or shutil.which("google-chrome"):
        return "chrome"

    return ""


def _get_page():
    """获取当前活动页面"""
    global _pages, _current_page_idx
    if not _pages:
        return None
    if _current_page_idx >= len(_pages):
        _current_page_idx = 0
    return _pages[_current_page_idx] if _pages else None


def _random_ua() -> str:
    """生成随机 User-Agent"""
    uas = [
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:126.0) Gecko/20100101 Firefox/126.0",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
    ]
    return random.choice(uas)


async def handle_action(action: str, params: dict) -> dict:
    """处理单个浏览器操作"""
    global _playwright

    try:
        match action:
            # 基础动作
            case "launch":
                return await _launch(params)
            case "navigate":
                return await _navigate(params)
            case "click":
                return await _click(params)
            case "fill":
                return await _fill(params)
            case "text":
                return await _get_text(params)
            case "attr":
                return await _get_attr(params)
            case "screenshot":
                return await _screenshot(params)
            case "evaluate":
                return await _evaluate(params)
            case "wait":
                return await _wait_for(params)
            case "select":
                return await _select(params)
            case "check":
                return await _check(params)
            case "close":
                return await _close()
            # 新增动作
            case "extract_text":
                return await _extract_text(params)
            case "extract_html":
                return await _extract_html(params)
            case "extract_table":
                return await _extract_table(params)
            case "extract_links":
                return await _extract_links(params)
            case "extract_attribute":
                return await _extract_attribute(params)
            case "scroll_to":
                return await _scroll_to(params)
            case "pdf":
                return await _pdf(params)
            case "cookies":
                return await _cookies(params)
            case "set_headers":
                return await _set_headers(params)
            case "new_page":
                return await _new_page(params)
            case "close_page":
                return await _close_page(params)
            case "switch_page":
                return await _switch_page(params)
            case "pages":
                return await _list_pages()
            case "back":
                return await _back()
            case "forward":
                return await _forward()
            case "reload":
                return await _reload()
            case "current_url":
                return await _current_url()
            case "get_title":
                return await _get_title()
            # v1.2 新动作
            case "ocr":
                return await _ocr(params)
            case "find_text":
                return await _find_text(params)
            case "recording_start":
                return await _recording_start(params)
            case "recording_stop":
                return await _recording_stop()
            # v1.3 预览模式
            case "preview":
                return await _preview(params)
            case "click_at_position":
                return await _click_at_position(params)
            # v1.4 元素选择器
            case "pick":
                return await _pick(params)
            # v1.5 连续拾取
            case "pick_start":
                return await _pick_start(params)
            case "pick_next":
                return await _pick_next()
            case "pick_stop":
                return await _pick_stop()
            case _:
                return {"success": False, "error": f"未知操作: {action}"}
    except Exception as e:
        return {"success": False, "error": str(e)}


async def _launch(params: dict) -> dict:
    global _pages, _playwright, _context, _extra_headers

    if async_playwright is None:
        return {"success": False, "error": "Playwright 未安装。请运行: pip install playwright"}

    if _context is not None:
        return {"success": True, "data": {"message": "浏览器已在运行"}}

    headless = params.get("headless", True)
    browser_type = params.get("browser", "chromium")

    raw_channel = params.get("channel", "")
    if raw_channel == "chromium":
        channel = ""
    elif raw_channel == "auto" or raw_channel == "":
        channel = _detect_browser_channel()
    else:
        channel = raw_channel

    # 代理配置
    proxy = params.get("proxy")
    proxy_opts = None
    if proxy:
        proxy_opts = {"server": proxy}
        if params.get("proxy_username"):
            proxy_opts["username"] = params["proxy_username"]
        if params.get("proxy_password"):
            proxy_opts["password"] = params["proxy_password"]

    _playwright = await async_playwright().start()
    browser_obj = getattr(_playwright, browser_type, None)
    if browser_obj is None:
        return {"success": False, "error": f"不支持的浏览器类型: {browser_type}"}


    # 计算 user_data_dir（持久化登录态/cookies/localStorage）
    user_data_dir = params.get("user_data_dir")
    if not user_data_dir:
        # 默认放在 exe 旁的 browser-data/ 目录
        import pathlib
        try:
            exe_dir = pathlib.Path(__file__).resolve().parent.parent
            user_data_dir = str(exe_dir / "browser-data")
        except Exception:
            user_data_dir = os.path.join(os.path.expanduser("~"), ".workflow-engine", "browser-data")
    os.makedirs(user_data_dir, exist_ok=True)

    # launch_persistent_context：浏览器+上下文合一，登录态自动持久化
    context_opts = {"headless": headless}
    if channel:
        context_opts["channel"] = channel
    if params.get("executable_path"):
        context_opts["executable_path"] = params["executable_path"]
    if proxy_opts:
        context_opts["proxy"] = proxy_opts

    # 视口
    viewport = params.get("viewport")
    if viewport:
        context_opts["viewport"] = viewport
    else:
        context_opts["viewport"] = {"width": 1280, "height": 720}

    # 额外 HTTP 头
    extra = params.get("extra_headers", {})
    _extra_headers = extra
    if extra:
        context_opts["extra_http_headers"] = extra

    _context = await browser_obj.launch_persistent_context(user_data_dir, **context_opts)

    # 确保至少有一个页面
    if not _context.pages:
        await _context.new_page()
    _pages = list(_context.pages)

    # 设置 cookies（如果传入了额外的 cookies）
    cookies = params.get("cookies")
    if cookies and isinstance(cookies, list):
        await _context.add_cookies(cookies)

    used = channel or "playwright-bundled"
    return {"success": True, "data": {"message": "浏览器已启动", "browser": browser_type, "channel": used, "user_data_dir": user_data_dir}}


async def _navigate(params: dict) -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动，请先执行 launch"}

    url = params.get("url")
    if not url:
        return {"success": False, "error": "navigate 缺少 url 参数"}

    wait_until = params.get("wait_until", "load")
    response = await page.goto(url, wait_until=wait_until)

    return {
        "success": True,
        "data": {
            "url": page.url,
            "title": await page.title(),
            "status": response.status if response else None,
        },
    }


async def _click(params: dict) -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector")
    if not selector:
        return {"success": False, "error": "click 缺少 selector 参数"}

    wait_ms = params.get("wait_ms", 0)
    await page.click(selector)
    if wait_ms > 0:
        await asyncio.sleep(wait_ms / 1000)

    return {"success": True, "data": {"selector": selector}}


async def _fill(params: dict) -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector")
    value = params.get("value", "")
    if not selector:
        return {"success": False, "error": "fill 缺少 selector 参数"}

    clear = params.get("clear", True)
    if clear:
        await page.fill(selector, value)
    else:
        await page.type(selector, value)

    return {"success": True, "data": {"selector": selector, "value": value}}


async def _get_text(params: dict) -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector")
    if not selector:
        return {"success": False, "error": "text 缺少 selector 参数"}

    element = await page.wait_for_selector(selector)
    text = await element.text_content() if element else ""

    return {"success": True, "data": {"text": text, "selector": selector}}


async def _get_attr(params: dict) -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector")
    attribute = params.get("attribute", "href")
    if not selector:
        return {"success": False, "error": "attr 缺少 selector 参数"}

    element = await page.wait_for_selector(selector)
    value = await element.get_attribute(attribute) if element else None

    return {"success": True, "data": {"attribute": attribute, "value": value, "selector": selector}}


async def _screenshot(params: dict) -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    path = params.get("path", "screenshot.png")
    full_page = params.get("full_page", False)

    await page.screenshot(path=path, full_page=full_page)

    return {"success": True, "data": {"path": path}}


async def _evaluate(params: dict) -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    script = params.get("script")
    if not script:
        return {"success": False, "error": "evaluate 缺少 script 参数"}

    result = await page.evaluate(script)

    try:
        json.dumps(result)
        serializable_result = result
    except (TypeError, ValueError):
        serializable_result = str(result)

    return {"success": True, "data": {"result": serializable_result}}


async def _wait_for(params: dict) -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector")
    if not selector:
        return {"success": False, "error": "wait 缺少 selector 参数"}

    timeout_ms = params.get("timeout_ms", 30000)
    element = await page.wait_for_selector(selector, timeout=timeout_ms)

    return {"success": True, "data": {"selector": selector, "found": element is not None}}


async def _select(params: dict) -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector")
    value = params.get("value")
    if not selector or value is None:
        return {"success": False, "error": "select 缺少 selector 或 value 参数"}

    await page.select_option(selector, value)

    return {"success": True, "data": {"selector": selector, "value": value}}


async def _check(params: dict) -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector")
    if not selector:
        return {"success": False, "error": "check 缺少 selector 参数"}

    checked = params.get("checked", True)
    if checked:
        await page.check(selector)
    else:
        await page.uncheck(selector)

    return {"success": True, "data": {"selector": selector, "checked": checked}}


async def _close() -> dict:
    global _pages, _playwright, _context, _extra_headers

    for p in _pages:
        try:
            await p.close()
        except:
            pass
    if _context:
        try:
            await _context.close()
        except:
            pass
    if _playwright:
        await _playwright.stop()

    _context = None
    _pages = []
    _current_page_idx = 0
    _playwright = None
    _extra_headers = {}

    return {"success": True, "data": {"message": "浏览器已关闭"}}


# ═══════════════════════════════════════════════
# 新增动作 (v1.1) — 网页数据抓取增强
# ═══════════════════════════════════════════════

async def _extract_text(params: dict) -> dict:
    """批量提取所有匹配元素的文本"""
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector")
    if not selector:
        return {"success": False, "error": "extract_text 缺少 selector 参数"}

    timeout_ms = params.get("timeout_ms", 10000)
    try:
        await page.wait_for_selector(selector, timeout=timeout_ms)
    except:
        pass

    elements = await page.query_selector_all(selector)
    texts = []
    for el in elements:
        t = await el.text_content()
        texts.append((t or "").strip())

    return {"success": True, "data": {"texts": texts, "count": len(texts), "selector": selector}}


async def _extract_html(params: dict) -> dict:
    """批量提取所有匹配元素的 HTML"""
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector")
    if not selector:
        return {"success": False, "error": "extract_html 缺少 selector 参数"}

    timeout_ms = params.get("timeout_ms", 10000)
    try:
        await page.wait_for_selector(selector, timeout=timeout_ms)
    except:
        pass

    elements = await page.query_selector_all(selector)
    htmls = []
    for el in elements:
        h = await el.inner_html()
        htmls.append(h)

    return {"success": True, "data": {"htmls": htmls, "count": len(htmls), "selector": selector}}


async def _extract_table(params: dict) -> dict:
    """提取 HTML 表格为二维数组（JSON）

    如果不指定 selector，自动找第一个 <table>
    """
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector", "table")
    timeout_ms = params.get("timeout_ms", 10000)
    try:
        await page.wait_for_selector(selector, timeout=timeout_ms)
    except:
        pass

    table_el = await page.query_selector(selector)
    if not table_el:
        return {"success": False, "error": f"找不到表格元素: {selector}"}

    # 用 JS 提取表格数据
    table_data = await page.evaluate("""
        (selector) => {
            const table = document.querySelector(selector);
            if (!table) return null;
            const rows = [];
            for (const tr of table.querySelectorAll('tr')) {
                const cells = [];
                for (const cell of tr.querySelectorAll('th, td')) {
                    cells.push(cell.textContent.trim());
                }
                rows.push(cells);
            }
            return rows;
        }
    """, selector)

    if table_data is None:
        return {"success": False, "error": "表格提取失败"}

    # 分离表头和数据
    headers = table_data[0] if table_data else []
    data_rows = table_data[1:] if len(table_data) > 1 else []

    return {
        "success": True,
        "data": {
            "headers": headers,
            "rows": data_rows,
            "raw": table_data,
            "row_count": len(data_rows),
            "selector": selector,
        }
    }


async def _extract_links(params: dict) -> dict:
    """提取页面中的所有链接

    params.selector: 限定范围（可选，默认整个页面）
    """
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    scope = params.get("selector", "body")

    links = await page.evaluate("""
        (scope) => {
            const container = document.querySelector(scope) || document.body;
            const anchors = container.querySelectorAll('a[href]');
            const results = [];
            for (const a of anchors) {
                const href = a.href;
                const text = a.textContent.trim();
                if (href && !href.startsWith('javascript:') && !href.startsWith('#')) {
                    results.push({ text, href });
                }
            }
            return results;
        }
    """, scope)

    return {"success": True, "data": {"links": links, "count": len(links)}}


async def _extract_attribute(params: dict) -> dict:
    """批量提取所有匹配元素的指定属性"""
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector")
    attribute = params.get("attribute", "href")
    if not selector:
        return {"success": False, "error": "extract_attribute 缺少 selector 参数"}

    timeout_ms = params.get("timeout_ms", 10000)
    try:
        await page.wait_for_selector(selector, timeout=timeout_ms)
    except:
        pass

    elements = await page.query_selector_all(selector)
    values = []
    for el in elements:
        v = await el.get_attribute(attribute)
        values.append(v)

    return {"success": True, "data": {"values": values, "count": len(values), "attribute": attribute, "selector": selector}}


async def _scroll_to(params: dict) -> dict:
    """滚动页面

    params:
      to: "bottom" | "top" | number (像素)
      times: 重复滚动次数（默认1，用于无限滚动加载更多）
      delay_ms: 每次滚动后等待毫秒数（默认1000）
    """
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    to = params.get("to", "bottom")
    times = params.get("times", 1)
    delay_ms = params.get("delay_ms", 1000)

    for i in range(times):
        if to == "bottom":
            await page.evaluate("window.scrollTo(0, document.body.scrollHeight)")
        elif to == "top":
            await page.evaluate("window.scrollTo(0, 0)")
        elif isinstance(to, (int, float)):
            await page.evaluate(f"window.scrollTo(0, {to})")

        if i < times - 1:
            await asyncio.sleep(delay_ms / 1000)

    # 等待最后一批内容加载
    await asyncio.sleep(delay_ms / 1000)

    scroll_height = await page.evaluate("document.body.scrollHeight")
    scroll_y = await page.evaluate("window.scrollY")

    return {"success": True, "data": {"scroll_height": scroll_height, "scroll_y": scroll_y, "times": times}}


async def _pdf(params: dict) -> dict:
    """生成当前页面的 PDF"""
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    path = params.get("path", "page.pdf")
    await page.pdf(path=path)

    return {"success": True, "data": {"path": path}}


async def _cookies(params: dict) -> dict:
    """Cookie 管理"""
    global _context
    if _context is None:
        return {"success": False, "error": "浏览器未启动"}

    action = params.get("action", "get")

    if action == "get":
        urls = params.get("urls")
        if urls:
            cookies = await _context.cookies(urls)
        else:
            cookies = await _context.cookies()
        return {"success": True, "data": {"cookies": cookies, "count": len(cookies)}}

    elif action == "set":
        cookies = params.get("cookies", [])
        if not isinstance(cookies, list):
            return {"success": False, "error": "cookies 必须是数组"}
        await _context.add_cookies(cookies)
        return {"success": True, "data": {"set": len(cookies)}}

    elif action == "clear":
        await _context.clear_cookies()
        return {"success": True, "data": {"message": "Cookie 已清除"}}

    return {"success": False, "error": f"未知 cookie 操作: {action}"}


async def _set_headers(params: dict) -> dict:
    """设置额外 HTTP 头"""
    global _context, _extra_headers
    if _context is None:
        return {"success": False, "error": "浏览器未启动"}

    headers = params.get("headers", {})
    _extra_headers.update(headers)
    await _context.set_extra_http_headers(_extra_headers)

    return {"success": True, "data": {"headers": _extra_headers}}


async def _new_page(params: dict) -> dict:
    """新建页面标签"""
    global _pages, _current_page_idx, _context
    if _context is None:
        return {"success": False, "error": "浏览器未启动"}

    page = await _context.new_page()
    _pages.append(page)
    _current_page_idx = len(_pages) - 1

    url = params.get("url")
    if url:
        await page.goto(url)

    return {"success": True, "data": {"page_index": _current_page_idx, "total": len(_pages)}}


async def _close_page(params: dict) -> dict:
    """关闭页面标签"""
    global _pages, _current_page_idx
    page = _get_page()
    if page is None:
        return {"success": False, "error": "没有打开的页面"}

    index = params.get("index", _current_page_idx)
    if index < 0 or index >= len(_pages):
        return {"success": False, "error": f"页面索引 {index} 超出范围"}

    await _pages[index].close()
    _pages.pop(index)

    if not _pages:
        _current_page_idx = 0
    elif _current_page_idx >= len(_pages):
        _current_page_idx = len(_pages) - 1

    return {"success": True, "data": {"closed": index, "remaining": len(_pages)}}


async def _switch_page(params: dict) -> dict:
    """切换到指定页面标签"""
    global _current_page_idx
    index = params.get("index", 0)
    if index < 0 or index >= len(_pages):
        return {"success": False, "error": f"页面索引 {index} 超出范围"}

    _current_page_idx = index
    page = _pages[index]

    return {"success": True, "data": {"index": index, "url": page.url, "title": await page.title()}}


async def _list_pages() -> dict:
    """列出所有打开的页面"""
    pages_info = []
    for i, p in enumerate(_pages):
        try:
            pages_info.append({
                "index": i,
                "url": p.url,
                "title": await p.title(),
                "active": i == _current_page_idx,
            })
        except:
            pages_info.append({"index": i, "url": "about:blank", "title": "", "active": i == _current_page_idx})

    return {"success": True, "data": {"pages": pages_info, "current": _current_page_idx}}


async def _back() -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}
    await page.go_back()
    return {"success": True, "data": {"url": page.url, "title": await page.title()}}


async def _forward() -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}
    await page.go_forward()
    return {"success": True, "data": {"url": page.url, "title": await page.title()}}


async def _reload() -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}
    await page.reload()
    return {"success": True, "data": {"url": page.url, "title": await page.title()}}


async def _current_url() -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}
    return {"success": True, "data": {"url": page.url}}


async def _get_title() -> dict:
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}
    return {"success": True, "data": {"title": await page.title()}}


# ═══════════════════════════════════════════
# v1.3 网页预览（点页面自动填选择器）
# ═══════════════════════════════════════════

# 预览截图的缩放比例（前端图片实际渲染宽度 / 原始截图宽度）
_preview_scale = 1.0
# 预览截图原始宽度
_preview_orig_width = 0


async def _preview(params: dict) -> dict:
    """预览页面：打开 URL → 截图 → 获取所有可见元素

    params:
        url: 目标 URL
        wait_until: 等待条件 (默认 networkidle)
        headless: 无头模式 (默认 true)
        viewport: 视口大小 (默认 {"width": 1280, "height": 720})
    """
    global _preview_scale, _preview_orig_width

    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    url = params.get("url")
    if not url:
        return {"success": False, "error": "preview 缺少 url 参数"}

    wait_until = params.get("wait_until", "networkidle")

    try:
        await page.goto(url, wait_until=wait_until, timeout=30000)
    except Exception as e:
        return {"success": False, "error": f"页面加载失败: {str(e)}"}

    # 等待页面稳定
    await asyncio.sleep(1.5)

    # 截图
    screenshot_bytes = await page.screenshot(full_page=False)
    screenshot_b64 = base64.b64encode(screenshot_bytes).decode("utf-8")

    # 获取页面信息
    title = await page.title()
    current_url = page.url

    # 获取视口尺寸
    viewport = await page.evaluate("""
        ({ width: window.innerWidth, height: window.innerHeight })
    """)
    _preview_orig_width = viewport["width"]

    # 获取所有可见元素的信息
    elements = await page.evaluate("""
        (() => {
            function buildSelector(el) {
                // 优先级：id > data-* 属性 > 唯一 class 组合 > nth-child 路径
                if (el.id) {
                    return '#' + CSS.escape(el.id);
                }

                // data- 属性选择器（如 data-testid, data-id）
                for (const attr of ['data-testid', 'data-id', 'data-key', 'data-name', 'data-field']) {
                    const val = el.getAttribute(attr);
                    if (val && val.length < 60) {
                        return el.tagName.toLowerCase() + '[' + attr + '="' + val + '"]';
                    }
                }

                const tag = el.tagName.toLowerCase();

                // 尝试用 class 选择器
                if (el.classList && el.classList.length > 0) {
                    const classes = Array.from(el.classList)
                        .filter(c => c && !c.match(/^\\d/) && c.length < 40)
                        .slice(0, 3);
                    if (classes.length > 0) {
                        return tag + '.' + classes.map(c => CSS.escape(c)).join('.');
                    }
                }

                // 回退：构建路径选择器
                const path = [];
                let current = el;
                while (current && current !== document.body && current !== document.documentElement && path.length < 5) {
                    let segment = current.tagName.toLowerCase();
                    if (current.id) {
                        segment = '#' + CSS.escape(current.id);
                        path.unshift(segment);
                        break;
                    }
                    const parent = current.parentElement;
                    if (parent) {
                        const siblings = Array.from(parent.children).filter(
                            c => c.tagName === current.tagName
                        );
                        if (siblings.length > 1) {
                            const idx = siblings.indexOf(current) + 1;
                            segment += ':nth-child(' + siblings.indexOf(current) + 1 + ')';
                        }
                    }
                    path.unshift(segment);
                    current = current.parentElement;
                }
                return path.join(' > ');
            }

            function isInteractive(el) {
                const interactiveTags = ['A', 'BUTTON', 'INPUT', 'SELECT', 'TEXTAREA', 'LABEL', 'SUMMARY', 'DETAILS'];
                const tag = el.tagName;
                if (interactiveTags.includes(tag)) return true;
                if (el.hasAttribute('onclick') || el.getAttribute('role') === 'button') return true;
                if (getComputedStyle(el).cursor === 'pointer') return true;
                return false;
            }

            function isContainer(el) {
                const children = el.children.length;
                if (children >= 2) return true;
                // UL/OL 即使子元素少也可能是容器
                if (['UL', 'OL', 'TBODY', 'TABLE'].includes(el.tagName)) return true;
                return false;
            }

            const results = [];
            const seen = new Set();

            document.querySelectorAll('*').forEach(el => {
                const rect = el.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) return;
                if (rect.x < 0 || rect.y < 0) return;
                if (rect.x >= window.innerWidth || rect.y >= window.innerHeight) return;
                if (el.tagName === 'HTML' || el.tagName === 'BODY') return;

                const selector = buildSelector(el);
                // 去重
                if (seen.has(selector)) return;
                seen.add(selector);

                const text = (el.textContent || '').trim().substring(0, 200);
                const elemType = isInteractive(el) ? 'interactive' : (isContainer(el) ? 'container' : 'text');

                results.push({
                    tag: el.tagName.toLowerCase(),
                    text: text,
                    selector: selector,
                    bbox: {
                        x: Math.round(rect.x),
                        y: Math.round(rect.y),
                        w: Math.round(rect.width),
                        h: Math.round(rect.height),
                    },
                    type: elemType,
                    child_count: el.children.length,
                });
            });

            // 按面积降序排列（大元素优先，方便选择容器）
            results.sort((a, b) => (b.bbox.w * b.bbox.h) - (a.bbox.w * a.bbox.h));

            return results;
        })()
    """)

    return {
        "success": True,
        "data": {
            "screenshot": screenshot_b64,
            "url": current_url,
            "title": title,
            "viewport": viewport,
            "elements": elements,
            "element_count": len(elements),
        }
    }


async def _click_at_position(params: dict) -> dict:
    """根据视口坐标点击元素，用于后端精确点击"""
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    x = params.get("x")
    y = params.get("y")
    if x is None or y is None:
        return {"success": False, "error": "click_at_position 缺少 x/y 参数"}

    await page.mouse.click(x, y)
    await asyncio.sleep(0.3)
    return {"success": True, "data": {"x": x, "y": y}}


async def _pick(params: dict) -> dict:
    """元素选择器：hover 时蓝色高亮，点击返回最佳 CSS 选择器

    选择器优先级：id > data-testid/data-id/data-key > 唯一 class > nth-child 路径

    params:
      url: 可选，先导航到此 URL 再开始选择（避免空白页）
    """
    page = _get_page()
    if page is None:
        # 如果没有页面，先启动浏览器
        await _launch({"headless": False})
        page = _get_page()
        if page is None:
            return {"success": False, "error": "浏览器未启动"}

    # 如果传了 URL，先导航过去并等待加载完成
    url = params.get("url")
    if url:
        try:
            await page.goto(url, wait_until="domcontentloaded", timeout=30000)
            # 额外等待网络空闲（最多 5 秒，不阻塞）
            try:
                await page.wait_for_load_state("networkidle", timeout=5000)
            except Exception:
                pass  # networkidle 超时不影响，domcontentloaded 已够用
        except Exception as e:
            return {"success": False, "error": f"导航失败: {e}"}

    try:
        await page.evaluate("""
        (() => {
            if (window.__wfPickActive) return;
            window.__wfPickActive = true;
            window.__wfPickResult = null;

            const style = document.createElement('style');
            style.id = '__wf-pick-style';
            style.textContent = `
                .wf-pick-hover { outline: 2px solid #58a6ff !important; outline-offset: 1px; }
                .wf-pick-selected { outline: 2px solid #3fb950 !important; outline-offset: 1px; }
            `;
            document.head.appendChild(style);

            let _hovered = null;

            function buildSelector(el) {
                // 优先级 1: id
                if (el.id) return '#' + CSS.escape(el.id);

                // 优先级 2: data-testid / data-id / data-key
                const testId = el.getAttribute('data-testid');
                if (testId) return '[data-testid="' + CSS.escape(testId) + '"]';
                const dataId = el.getAttribute('data-id');
                if (dataId) return '[data-id="' + CSS.escape(dataId) + '"]';
                const dataKey = el.getAttribute('data-key');
                if (dataKey) return '[data-key="' + CSS.escape(dataKey) + '"]';

                // 优先级 3: 唯一 class
                const tag = el.tagName.toLowerCase();
                if (typeof el.className === 'string' && el.className.trim()) {
                    const classes = el.className.trim().split(/\\s+/);
                    for (const cls of classes) {
                        if (!cls) continue;
                        const sel = tag + '.' + CSS.escape(cls);
                        if (document.querySelectorAll(sel).length === 1) return sel;
                    }
                }

                // 优先级 4: nth-child 路径
                const path = [];
                let cur = el;
                while (cur && cur !== document.body && cur !== document.documentElement) {
                    const parent = cur.parentElement;
                    if (!parent) break;
                    const siblings = Array.from(parent.children).filter(
                        c => c.tagName === cur.tagName
                    );
                    if (siblings.length > 1) {
                        const idx = siblings.indexOf(cur) + 1;
                        path.unshift(cur.tagName.toLowerCase() + ':nth-of-type(' + idx + ')');
                    } else {
                        path.unshift(cur.tagName.toLowerCase());
                    }
                    cur = parent;
                }
                return path.join(' > ');
            }

            window.__wfPickMouseover = function(e) {
                if (_hovered) _hovered.classList.remove('wf-pick-hover');
                _hovered = e.target;
                _hovered.classList.add('wf-pick-hover');
            };

            window.__wfPickClick = function(e) {
                e.stopPropagation();
                e.preventDefault();
                if (_hovered) _hovered.classList.remove('wf-pick-hover');
                const sel = buildSelector(e.target);
                window.__wfPickResult = sel;
                window.__wfPickCleanup();
            };

            window.__wfPickCleanup = function() {
                document.removeEventListener('mouseover', window.__wfPickMouseover, true);
                document.removeEventListener('click', window.__wfPickClick, true);
                const s = document.getElementById('__wf-pick-style');
                if (s) s.remove();
                if (_hovered) _hovered.classList.remove('wf-pick-hover');
                window.__wfPickActive = false;
            };

            document.addEventListener('mouseover', window.__wfPickMouseover, true);
            document.addEventListener('click', window.__wfPickClick, true);
        })()
        """)

        # 轮询等待用户点击（最多 30 秒）
        for _ in range(300):
            await asyncio.sleep(0.1)
            result = await page.evaluate("window.__wfPickResult")
            if result:
                return {"success": True, "data": {"selector": result}}

        # 超时，清理注入的 JS
        try:
            await page.evaluate("window.__wfPickCleanup && window.__wfPickCleanup()")
        except Exception:
            pass
        return {"success": False, "error": "元素选择超时（30 秒内未点击任何元素）"}

    except Exception as e:
        return {"success": False, "error": f"pick 失败: {e}"}


# ─── v1.5 连续拾取 ───
_pick_session_active = False

async def _inject_picker_js(page) -> None:
    """注入元素选择器 JS（共享逻辑）"""
    await page.evaluate("""
    (() => {
        if (window.__wfPickActive) {
            try { window.__wfPickCleanup(); } catch(e) {}
        }
        window.__wfPickActive = true;
        window.__wfPickResult = null;

        const style = document.createElement('style');
        style.id = '__wf-pick-style';
        style.textContent = `
            .wf-pick-hover { outline: 2px solid #58a6ff !important; outline-offset: 1px; }
            .wf-pick-selected { outline: 2px solid #3fb950 !important; outline-offset: 1px; }
        `;
        document.head.appendChild(style);

        let _hovered = null;

        function buildSelector(el) {
            if (el.id) return '#' + CSS.escape(el.id);
            const testId = el.getAttribute('data-testid');
            if (testId) return '[data-testid="' + CSS.escape(testId) + '"]';
            const dataId = el.getAttribute('data-id');
            if (dataId) return '[data-id="' + CSS.escape(dataId) + '"]';
            const dataKey = el.getAttribute('data-key');
            if (dataKey) return '[data-key="' + CSS.escape(dataKey) + '"]';
            const tag = el.tagName.toLowerCase();
            if (typeof el.className === 'string' && el.className.trim()) {
                const classes = el.className.trim().split(/\\s+/);
                for (const cls of classes) {
                    if (!cls) continue;
                    const sel = tag + '.' + CSS.escape(cls);
                    if (document.querySelectorAll(sel).length === 1) return sel;
                }
            }
            const path = [];
            let cur = el;
            while (cur && cur !== document.body && cur !== document.documentElement) {
                const parent = cur.parentElement;
                if (!parent) break;
                const siblings = Array.from(parent.children).filter(c => c.tagName === cur.tagName);
                if (siblings.length > 1) {
                    const idx = siblings.indexOf(cur) + 1;
                    path.unshift(cur.tagName.toLowerCase() + ':nth-of-type(' + idx + ')');
                } else {
                    path.unshift(cur.tagName.toLowerCase());
                }
                cur = parent;
            }
            return path.join(' > ');
        }

        window.__wfPickMouseover = function(e) {
            if (_hovered) _hovered.classList.remove('wf-pick-hover');
            _hovered = e.target;
            _hovered.classList.add('wf-pick-hover');
        };

        window.__wfPickClick = function(e) {
            e.stopPropagation();
            e.preventDefault();
            if (_hovered) _hovered.classList.remove('wf-pick-hover');
            const sel = buildSelector(e.target);
            e.target.classList.add('wf-pick-selected');
            setTimeout(() => e.target.classList.remove('wf-pick-selected'), 600);
            window.__wfPickResult = sel;
            // 不调用 cleanup，保持拾取模式
        };

        window.__wfPickCleanup = function() {
            document.removeEventListener('mouseover', window.__wfPickMouseover, true);
            document.removeEventListener('click', window.__wfPickClick, true);
            const s = document.getElementById('__wf-pick-style');
            if (s) s.remove();
            if (_hovered) _hovered.classList.remove('wf-pick-hover');
            window.__wfPickActive = false;
            window.__wfPickResult = null;
        };

        document.addEventListener('mouseover', window.__wfPickMouseover, true);
        document.addEventListener('click', window.__wfPickClick, true);
    })()
    """)


async def _pick_start(params: dict) -> dict:
    """开始连续拾取：打开浏览器 + 注入选择器"""
    global _pick_session_active

    page = _get_page()
    if page is None:
        await _launch({"headless": False})
        page = _get_page()
        if page is None:
            return {"success": False, "error": "浏览器未启动"}

    url = params.get("url")
    if url:
        try:
            await page.goto(url, wait_until="domcontentloaded", timeout=30000)
            try:
                await page.wait_for_load_state("networkidle", timeout=5000)
            except Exception:
                pass
        except Exception as e:
            return {"success": False, "error": f"导航失败: {e}"}

    try:
        await _inject_picker_js(page)
        _pick_session_active = True
        return {"success": True, "data": {"message": "连续拾取已开始"}}
    except Exception as e:
        return {"success": False, "error": f"pick_start 失败: {e}"}


async def _pick_next() -> dict:
    """等待用户点选下一个元素，返回 selector"""
    global _pick_session_active

    page = _get_page()
    if page is None:
        _pick_session_active = False
        return {"success": False, "error": "浏览器未启动"}

    if not _pick_session_active:
        return {"success": False, "error": "未在拾取模式，请先调用 pick_start"}

    try:
        # 清除上次结果
        await page.evaluate("window.__wfPickResult = null")

        # 轮询等待点击
        for _ in range(300):
            await asyncio.sleep(0.1)
            result = await page.evaluate("window.__wfPickResult")
            if result:
                return {"success": True, "data": {"selector": result}}

        # 超时
        await _pick_stop()
        return {"success": False, "error": "拾取超时（30 秒）"}
    except Exception as e:
        _pick_session_active = False
        return {"success": False, "error": f"pick_next 失败: {e}"}


async def _pick_stop() -> dict:
    """结束连续拾取，清理 JS"""
    global _pick_session_active
    _pick_session_active = False

    page = _get_page()
    if page:
        try:
            await page.evaluate("window.__wfPickCleanup && window.__wfPickCleanup()")
        except Exception:
            pass

    return {"success": True, "data": {"message": "连续拾取已结束"}}


# ═══════════════════════════════════════════
# v1.2 OCR 文字识别
# ═══════════════════════════════════════════

async def _ocr(params: dict) -> dict:
    """OCR 文字识别：截图 → 识别文字"""
    page = _get_page()
    if page is None:
        return {"success": False, "error": "无可用页面"}

    try:
        import pytesseract
        from PIL import Image
        import io
    except ImportError:
        return {"success": False, "error": "OCR 需要安装依赖: pip install pytesseract Pillow\n以及 Tesseract OCR 引擎"}

    try:
        region = params.get("region")
        if region:
            screenshot_bytes = await page.screenshot(clip={
                "x": region.get("x", 0), "y": region.get("y", 0),
                "width": region.get("width", 800), "height": region.get("height", 600),
            })
        else:
            screenshot_bytes = await page.screenshot()

        image = Image.open(io.BytesIO(screenshot_bytes))
        lang = params.get("lang", "chi_sim+eng")
        text = pytesseract.image_to_string(image, lang=lang)
        return {"success": True, "data": {"text": text.strip(), "length": len(text.strip())}}
    except Exception as e:
        return {"success": False, "error": f"OCR 失败: {e}"}


async def _find_text(params: dict) -> dict:
    """在页面上查找文字位置"""
    page = _get_page()
    if page is None:
        return {"success": False, "error": "无可用页面"}

    text = params.get("text", "")
    if not text:
        return {"success": False, "error": "缺少 text 参数"}

    try:
        locator = page.get_by_text(text, exact=False)
        count = await locator.count()
        results = []
        for i in range(min(count, 10)):
            el = locator.nth(i)
            box = await el.bounding_box()
            if box:
                results.append({
                    "x": box["x"] + box["width"] / 2, "y": box["y"] + box["height"] / 2,
                    "width": box["width"], "height": box["height"],
                    "text": (await el.text_content()) or "",
                })
        return {"success": True, "data": {"found": count, "locations": results}}
    except Exception as e:
        return {"success": False, "error": f"查找文字失败: {e}"}


# ═══════════════════════════════════════════
# v1.2 操作录制
# ═══════════════════════════════════════════

_recording = False
_recorded_actions: list = []


async def _recording_start(params: dict) -> dict:
    """开始录制浏览器操作"""
    global _recording
    page = _get_page()
    if page is None:
        return {"success": False, "error": "无可用页面"}

    _recording = True

    try:
        # 用 evaluate 代替 expose_function，在 JS 端维护全局操作数组
        # expose_function 在页面已加载后不可靠，改用 evaluate 读写全局变量
        await page.evaluate("window.__wfActions = []")

        await page.evaluate("""
        if (window.__wfRecording) return;
        window.__wfRecording = true;

        function record(type, data) {
            if (!window.__wfActions) window.__wfActions = [];
            window.__wfActions.push(Object.assign({ type }, data));
        }

        document.addEventListener('click', (e) => {
            const t = e.target;
            let sel = t.tagName.toLowerCase();
            if (t.id) sel = '#' + t.id;
            else if (t.className && typeof t.className === 'string')
                sel += '.' + t.className.trim().split(/\\s+/)[0];
            record('click', {selector:sel, x:e.clientX, y:e.clientY});
        }, true);

        document.addEventListener('change', (e) => {
            const t = e.target;
            if (['INPUT','TEXTAREA','SELECT'].includes(t.tagName)) {
                let sel = t.tagName.toLowerCase();
                if (t.id) sel = '#' + t.id;
                record('fill', {selector:sel, value:t.value});
            }
        }, true);
        """)
        return {"success": True, "data": {"message": "录制已开始，请在浏览器中操作"}}
    except Exception as e:
        _recording = False
        return {"success": False, "error": f"启动录制失败: {e}"}


async def _recording_stop() -> dict:
    """停止录制，返回操作列表"""
    global _recording
    _recording = False

    page = _get_page()
    if page is None:
        return {"success": True, "data": {"message": "录制已停止，共 0 个操作", "actions": []}}

    try:
        actions = await page.evaluate("window.__wfActions || []")
        # 清理
        await page.evaluate("delete window.__wfActions; delete window.__wfRecording")
    except Exception:
        actions = []

    return {"success": True, "data": {"message": f"录制已停止，共 {len(actions)} 个操作", "actions": actions}}


async def main():
    """主循环：从 stdin 读取 JSON 请求，处理后写入 stdout"""
    print(json.dumps({"type": "ready"}), flush=True)

    loop = asyncio.get_event_loop()

    while True:
        try:
            line = await loop.run_in_executor(None, sys.stdin.readline)
        except EOFError:
            break

        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
            req_id = request.get("id", "")
            action = request.get("action", "")
            params = request.get("params", {})

            if action == "shutdown":
                await _close()
                response = {"id": req_id, "success": True, "data": {"message": "关闭"}}
                print(json.dumps(response), flush=True)
                break

            result = await handle_action(action, params)
            response = {"id": req_id, **result}
        except json.JSONDecodeError as e:
            response = {
                "id": "",
                "success": False,
                "error": f"JSON 解析错误: {e}",
            }
        except Exception as e:
            response = {
                "id": request.get("id", "") if "request" in dir() else "",
                "success": False,
                "error": str(e),
                "traceback": traceback.format_exc(),
            }

        print(json.dumps(response), flush=True)


if __name__ == "__main__":
    asyncio.run(main())
