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

import collections

# 全局浏览器状态
_context = None  # PersistentContext（launch_persistent_context，管理 cookies/登录态/headers）
_pages = []      # 所有打开的页面
_current_page_idx = 0
_playwright = None
_extra_headers = {}

# CDP 事件管道
_cdp_session = None
_cdp_queue = collections.deque()


def _detect_browser_channel() -> str:
    """自动检测系统可用的 Chromium 内核浏览器（跳过 Edge，已知兼容性问题）"""
    # Edge headless 在部分版本上不稳定（exit code 21 崩溃），跳过
    if shutil.which("chrome") or shutil.which("google-chrome"):
        return "chrome"
    if shutil.which("msedge") or shutil.which("microsoft-edge"):
        return "msedge"

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
            # 健康检查
            case "ping":
                return {"success": True, "data": {"message": "pong"}}
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
            # 智能等待
            case "wait_network_idle":
                return await _wait_network_idle(params)
            case "wait_load_state":
                return await _wait_load_state(params)
            case "wait_url_contains":
                return await _wait_url_contains(params)
            # 动作验证
            case "verify":
                return await _verify(params)
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
                return await _pick_next(params)
            case "pick_stop":
                return await _pick_stop()
            # v1.6 文件下载
            case "download":
                return await _download(params)
            # v1.7 办公自动化新动作
            case "upload":
                return await _upload(params)
            case "keyboard":
                return await _keyboard(params)
            case "double_click":
                return await _double_click(params)
            case "drag_to":
                return await _drag_to(params)
            case "context_menu":
                return await _context_menu(params)
            case "switch_frame":
                return await _switch_frame(params)
            case "handle_dialog":
                return await _handle_dialog(params)
            case "scroll_to_element":
                return await _scroll_to_element(params)
            case "reset_state":
                return await _reset_state(params)
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
    # 启动 CDP 事件监听
    asyncio.create_task(_start_cdp_listener())
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


async def _wait_network_idle(params: dict) -> dict:
    """等待网络空闲（无飞行中的请求）"""
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}
    timeout_ms = params.get("timeout_ms", 30000)
    try:
        await page.wait_for_load_state("networkidle", timeout=timeout_ms / 1000)
        return {"success": True, "data": {"state": "networkidle"}}
    except Exception as e:
        return {"success": False, "error": f"等待网络空闲超时: {e}"}


async def _wait_load_state(params: dict) -> dict:
    """等待页面加载状态: load / domcontentloaded / networkidle"""
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}
    state = params.get("state", "load")
    timeout_ms = params.get("timeout_ms", 30000)
    try:
        await page.wait_for_load_state(state, timeout=timeout_ms / 1000)
        return {"success": True, "data": {"state": state}}
    except Exception as e:
        return {"success": False, "error": f"等待加载状态 {state} 超时: {e}"}


async def _wait_url_contains(params: dict) -> dict:
    """等待 URL 包含指定字符串"""
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}
    substring = params.get("substring", "")
    if not substring:
        return {"success": False, "error": "wait_url_contains 缺少 substring 参数"}
    timeout_ms = params.get("timeout_ms", 30000)
    start = asyncio.get_event_loop().time()
    while (asyncio.get_event_loop().time() - start) * 1000 < timeout_ms:
        if substring in page.url:
            return {"success": True, "data": {"url": page.url, "matched": substring}}
        await asyncio.sleep(0.2)
    return {"success": False, "error": f"等待 URL 包含 '{substring}' 超时 (当前: {page.url})"}


async def _verify(params: dict) -> dict:
    """验证最近操作的健康状态（检查 CDP 事件队列中的错误）"""
    global _cdp_queue
    issues = []
    events_snapshot = list(_cdp_queue)
    _cdp_queue.clear()

    for event_str in events_snapshot:
        try:
            event = json.loads(event_str) if isinstance(event_str, str) else event_str
        except (json.JSONDecodeError, TypeError):
            continue
        method = event.get("event", "")
        data = event.get("data", {})

        if method == "Network.loadingFailed":
            issues.append({
                "type": "network_error",
                "url": data.get("url", ""),
                "error": data.get("errorText", ""),
            })
        elif method == "Network.responseReceived":
            resp = data.get("response", {})
            status = resp.get("status", 0)
            if status >= 400:
                issues.append({
                    "type": "http_error",
                    "url": resp.get("url", ""),
                    "status": status,
                    "statusText": resp.get("statusText", ""),
                })
        elif method == "Runtime.exceptionThrown":
            exc = data.get("exceptionDetails", {})
            issues.append({
                "type": "js_exception",
                "text": exc.get("text", ""),
                "url": exc.get("url", ""),
            })

    if issues:
        return {
            "success": True,
            "data": {
                "clean": False,
                "issues": issues,
                "count": len(issues),
                "message": f"发现 {len(issues)} 个问题",
            },
        }
    return {"success": True, "data": {"clean": True, "issues": [], "message": "无异常"}}


# ═══════════════════════════════════════════
# v1.6 文件下载
# ═══════════════════════════════════════════

_pending_download = None  # asyncio.Future[Download]

async def _download(params: dict) -> dict:
    """触发下载：可选点击选择器 → 等待下载完成 → 保存到指定目录

    params:
      save_dir: 保存目录（默认当前目录）
      click_selector: 可选，在此之前点击这个元素触发下载
      timeout_ms: 等待下载超时（默认 30000）
    """
    global _pending_download
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    save_dir = params.get("save_dir", ".")
    click_selector = params.get("click_selector")
    timeout_ms = params.get("timeout_ms", 30000)

    # 设置下载监听
    download_future = asyncio.get_event_loop().create_future()

    async def _on_download(download):
        try:
            suggested = download.suggested_filename
            save_path = os.path.join(save_dir, suggested)
            os.makedirs(save_dir, exist_ok=True)
            await download.save_as(save_path)
            if not download_future.done():
                download_future.set_result({
                    "filename": suggested,
                    "path": save_path,
                    "url": download.url,
                })
        except Exception as e:
            if not download_future.done():
                download_future.set_result({"error": str(e)})

    page.on("download", lambda d: asyncio.create_task(_on_download(d)))

    # 可选：点击触发下载
    if click_selector:
        try:
            await page.click(click_selector)
        except Exception as e:
            return {"success": False, "error": f"点击下载按钮失败: {e}"}

    # 等待下载完成
    try:
        result = await asyncio.wait_for(download_future, timeout=timeout_ms / 1000)
        if "error" in result:
            return {"success": False, "error": result["error"]}
        return {"success": True, "data": result}
    except asyncio.TimeoutError:
        return {"success": False, "error": "下载超时：未检测到文件下载"}


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

    # 停止 CDP 监听
    await _stop_cdp_listener()

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


async def _reset_state(_params: dict) -> dict:
    """重置浏览器状态：关闭多余页面，保留一个空白页

    用于多个 browser 容器共享 sidecar 时，清理上一容器遗留的状态：
    - 关闭所有已打开的 dialog（关闭页面时自动处理）
    - 关闭除 index 0 外的所有页面
    - 将 index 0 页面导航到 about:blank
    """
    global _pages, _current_page_idx

    if not _pages:
        return {"success": True, "data": {"message": "无活跃页面，无需重置"}}

    try:
        # 先 list 所有页面
        all_pages = list(_pages)
        closed_count = 0

        # 从后往前关闭（避免索引变化），保留 index 0
        for i in range(len(all_pages) - 1, 0, -1):
            try:
                await all_pages[i].close()
                closed_count += 1
            except Exception:
                pass

        # 更新全局页面列表
        _pages = [all_pages[0]] if all_pages else []
        _current_page_idx = 0

        # 导航到 about:blank 重置页面状态
        if _pages:
            try:
                await _pages[0].goto("about:blank", wait_until="commit")
            except Exception:
                pass

        return {
            "success": True,
            "data": {
                "message": f"已清理 {closed_count} 个多余页面，当前保留 1 个空白页",
                "closed_pages": closed_count,
            },
        }
    except Exception as e:
        return {"success": False, "error": f"重置状态失败: {e}"}


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
                const candidates = [];
                const tag = el.tagName.toLowerCase();

                function tryAdd(sel, method, score) {
                    try {
                        if (document.querySelectorAll(sel).length === 1) {
                            candidates.push({selector: sel, method: method, score: score});
                        }
                    } catch(e) {}
                }

                // 1. #id (score 100)
                if (el.id) {
                    tryAdd('#' + CSS.escape(el.id), 'id', 100);
                }

                // 2. [data-testid] (score 90)
                const testId = el.getAttribute('data-testid');
                if (testId) {
                    tryAdd('[data-testid="' + CSS.escape(testId) + '"]', 'data-testid', 90);
                }

                // 3. Text-based :has-text (score 85)
                const text = el.textContent.trim();
                if (text && text.length < 50) {
                    try {
                        const matchingEls = Array.from(document.querySelectorAll(tag)).filter(
                            function(e) { return e.textContent.trim() === text; }
                        );
                        if (matchingEls.length === 1) {
                            candidates.push({selector: tag + ':has-text("' + text.replace(/"/g, '\\\\"') + '")', method: 'has-text', score: 85});
                        }
                    } catch(e) {}
                }

                // 4. Unique class combo (score 80)
                if (typeof el.className === 'string' && el.className.trim()) {
                    const classes = el.className.trim().split(/\\s+/);
                    if (classes.length > 1) {
                        tryAdd(tag + '.' + classes.map(function(c) { return CSS.escape(c); }).join('.'), 'multi-class', 80);
                    }
                    for (const cls of classes) {
                        if (!cls) continue;
                        tryAdd(tag + '.' + CSS.escape(cls), 'class', 80);
                    }
                }

                // 5. [data-id] / [data-key] (score 75)
                const dataId = el.getAttribute('data-id');
                if (dataId) {
                    tryAdd('[data-id="' + CSS.escape(dataId) + '"]', 'data-id', 75);
                }
                const dataKey = el.getAttribute('data-key');
                if (dataKey) {
                    tryAdd('[data-key="' + CSS.escape(dataKey) + '"]', 'data-key', 75);
                }

                // 6. aria role + label (score 70)
                const role = el.getAttribute('role');
                const ariaLabel = el.getAttribute('aria-label');
                if (role && ariaLabel) {
                    tryAdd('[role="' + CSS.escape(role) + '"][aria-label="' + CSS.escape(ariaLabel) + '"]', 'aria', 70);
                } else if (role) {
                    tryAdd('[role="' + CSS.escape(role) + '"]', 'role', 70);
                }

                // 7. nth-of-type path (score 50, last resort)
                const path = [];
                let cur = el;
                while (cur && cur !== document.body && cur !== document.documentElement) {
                    const parent = cur.parentElement;
                    if (!parent) break;
                    const siblings = Array.from(parent.children).filter(
                        function(c) { return c.tagName === cur.tagName; }
                    );
                    if (siblings.length > 1) {
                        const idx = siblings.indexOf(cur) + 1;
                        path.unshift(cur.tagName.toLowerCase() + ':nth-of-type(' + idx + ')');
                    } else {
                        path.unshift(cur.tagName.toLowerCase());
                    }
                    cur = parent;
                }
                candidates.push({selector: path.join(' > '), method: 'nth-of-type-path', score: 50});

                candidates.sort(function(a, b) { return b.score - a.score; });
                return {candidates: candidates.slice(0, 5)};
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
                var result = buildSelector(e.target);
                window.__wfPickResult = result;
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

        # 轮询等待用户点击（可配置超时，默认 30 秒）
        timeout_ms = params.get("timeout_ms", 30000)
        iterations = max(1, int(timeout_ms / 100))
        for _ in range(iterations):
            await asyncio.sleep(0.1)
            result = await page.evaluate("window.__wfPickResult")
            if result:
                return {"success": True, "data": result}

        # 超时，清理注入的 JS
        try:
            await page.evaluate("window.__wfPickCleanup && window.__wfPickCleanup()")
        except Exception:
            pass
        return {"success": False, "error": f"元素选择超时（{timeout_ms}ms 内未点击任何元素）"}

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
        window.__wfPickedElementInfo = null;

        const style = document.createElement('style');
        style.id = '__wf-pick-style';
        style.textContent = `
            .wf-pick-hover { outline: 2px solid #58a6ff !important; outline-offset: 1px; }
            .wf-pick-selected { outline: 2px solid #3fb950 !important; outline-offset: 1px; }
            #__wf-pick-tooltip {
                position: fixed; z-index: 999999; pointer-events: none;
                background: rgba(20,20,20,0.92); color: #f0f0f0;
                font: 11px/1.4 -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, monospace;
                padding: 6px 10px; border-radius: 6px;
                max-width: 360px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
                box-shadow: 0 2px 12px rgba(0,0,0,0.4);
                transition: opacity 0.1s; opacity: 0;
            }
            #__wf-pick-tooltip .__wf-tip-tag { color: #79c0ff; font-weight: 600; }
            #__wf-pick-tooltip .__wf-tip-id { color: #d2a8ff; }
            #__wf-pick-tooltip .__wf-tip-cls { color: #7ee787; }
            #__wf-pick-tooltip .__wf-tip-text { color: #e6edf3; font-style: italic; }
        `;
        document.head.appendChild(style);

        // Create tooltip element
        let _tooltip = document.getElementById('__wf-pick-tooltip');
        if (!_tooltip) {
            _tooltip = document.createElement('div');
            _tooltip.id = '__wf-pick-tooltip';
            document.body.appendChild(_tooltip);
        }

        let _hovered = null;

        function buildSelector(el) {
            const candidates = [];
            const tag = el.tagName.toLowerCase();

            function tryAdd(sel, method, score) {
                try {
                    if (document.querySelectorAll(sel).length === 1) {
                        candidates.push({selector: sel, method: method, score: score});
                    }
                } catch(e) {}
            }

            if (el.id) { tryAdd('#' + CSS.escape(el.id), 'id', 100); }

            const testId = el.getAttribute('data-testid');
            if (testId) { tryAdd('[data-testid="' + CSS.escape(testId) + '"]', 'data-testid', 90); }

            const text = el.textContent.trim();
            if (text && text.length < 50) {
                try {
                    const matchingEls = Array.from(document.querySelectorAll(tag)).filter(
                        function(e) { return e.textContent.trim() === text; }
                    );
                    if (matchingEls.length === 1) {
                        candidates.push({selector: tag + ':has-text("' + text.replace(/"/g, '\\\\"') + '")', method: 'has-text', score: 85});
                    }
                } catch(e) {}
            }

            if (typeof el.className === 'string' && el.className.trim()) {
                const classes = el.className.trim().split(/\\s+/);
                if (classes.length > 1) {
                    tryAdd(tag + '.' + classes.map(function(c) { return CSS.escape(c); }).join('.'), 'multi-class', 80);
                }
                for (const cls of classes) {
                    if (!cls) continue;
                    tryAdd(tag + '.' + CSS.escape(cls), 'class', 80);
                }
            }

            const dataId = el.getAttribute('data-id');
            if (dataId) { tryAdd('[data-id="' + CSS.escape(dataId) + '"]', 'data-id', 75); }
            const dataKey = el.getAttribute('data-key');
            if (dataKey) { tryAdd('[data-key="' + CSS.escape(dataKey) + '"]', 'data-key', 75); }

            const role = el.getAttribute('role');
            const ariaLabel = el.getAttribute('aria-label');
            if (role && ariaLabel) {
                tryAdd('[role="' + CSS.escape(role) + '"][aria-label="' + CSS.escape(ariaLabel) + '"]', 'aria', 70);
            } else if (role) {
                tryAdd('[role="' + CSS.escape(role) + '"]', 'role', 70);
            }

            const path = [];
            let cur = el;
            while (cur && cur !== document.body && cur !== document.documentElement) {
                const parent = cur.parentElement;
                if (!parent) break;
                const siblings = Array.from(parent.children).filter(function(c) { return c.tagName === cur.tagName; });
                if (siblings.length > 1) {
                    const idx = siblings.indexOf(cur) + 1;
                    path.unshift(cur.tagName.toLowerCase() + ':nth-of-type(' + idx + ')');
                } else {
                    path.unshift(cur.tagName.toLowerCase());
                }
                cur = parent;
            }
            candidates.push({selector: path.join(' > '), method: 'nth-of-type-path', score: 50});

            candidates.sort(function(a, b) { return b.score - a.score; });
            return {candidates: candidates.slice(0, 5)};
        }

        window.__wfPickMouseover = function(e) {
            if (_hovered) _hovered.classList.remove('wf-pick-hover');
            _hovered = e.target;
            _hovered.classList.add('wf-pick-hover');

            // Build tooltip content
            const el = e.target;
            const tag = el.tagName.toLowerCase();
            const id = el.id || '';
            const clsStr = (typeof el.className === 'string') ? el.className.trim() : '';
            const classes = clsStr ? clsStr.split(/\s+/).slice(0, 2) : [];
            const rawText = (el.textContent || '').trim().replace(/\s+/g, ' ');
            const text = rawText.length > 60 ? rawText.slice(0, 60) + '…' : rawText;
            const rect = el.getBoundingClientRect();

            let html = '<span class="__wf-tip-tag">&lt;' + tag;
            if (id) html += ' <span class="__wf-tip-id">#' + id + '</span>';
            if (classes.length) html += ' <span class="__wf-tip-cls">.' + classes.join('.') + '</span>';
            html += '&gt;</span>';
            if (text) html += ' <span class="__wf-tip-text">"' + text + '"</span>';
            html += ' <span style="color:#8b949e;font-size:10px">[' + Math.round(rect.width) + '×' + Math.round(rect.height) + ']</span>';

            _tooltip.innerHTML = html;
            _tooltip.style.opacity = '1';

            // Position tooltip near cursor, keeping it in viewport
            let tx = e.clientX + 12;
            let ty = e.clientY + 18;
            if (tx + 360 > window.innerWidth) tx = e.clientX - 360 - 8;
            if (ty + 40 > window.innerHeight) ty = e.clientY - 40;
            _tooltip.style.left = tx + 'px';
            _tooltip.style.top = ty + 'px';
        };

        window.__wfPickClick = function(e) {
            e.stopPropagation();
            e.preventDefault();
            if (_hovered) _hovered.classList.remove('wf-pick-hover');
            const result = buildSelector(e.target);
            e.target.classList.add('wf-pick-selected');
            setTimeout(() => e.target.classList.remove('wf-pick-selected'), 600);
            window.__wfPickResult = result;
            // Store element info for retrieval
            const el = e.target;
            const elTag = el.tagName.toLowerCase();
            const elId = el.id || '';
            const elClsStr = (typeof el.className === 'string') ? el.className.trim() : '';
            const elClasses = elClsStr ? elClsStr.split(/\s+/) : [];
            const elText = (el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 200);
            // Build outerHTML preview (truncated)
            let outerHtml = '';
            try { outerHtml = el.outerHTML; } catch(ex) {}
            if (outerHtml.length > 500) outerHtml = outerHtml.slice(0, 500) + '…';
            window.__wfPickedElementInfo = {
                tag: elTag,
                id: elId,
                classes: elClasses,
                text: elText,
                html_preview: outerHtml
            };
            // Hide tooltip on click
            if (_tooltip) _tooltip.style.opacity = '0';
            // 不调用 cleanup，保持拾取模式
        };

        window.__wfPickCleanup = function() {
            document.removeEventListener('mouseover', window.__wfPickMouseover, true);
            document.removeEventListener('click', window.__wfPickClick, true);
            const s = document.getElementById('__wf-pick-style');
            if (s) s.remove();
            const tt = document.getElementById('__wf-pick-tooltip');
            if (tt) tt.remove();
            if (_hovered) _hovered.classList.remove('wf-pick-hover');
            window.__wfPickActive = false;
            window.__wfPickResult = null;
            window.__wfPickedElementInfo = null;
        };

        document.addEventListener('mouseover', window.__wfPickMouseover, true);
        document.addEventListener('click', window.__wfPickClick, true);
    })()
    """)


async def _pick_start(params: dict) -> dict:
    """开始连续拾取：复用已有浏览器 或 打开可见浏览器"""
    global _pick_session_active

    page = _get_page()
    if page is None:
        # 浏览器未启动 → 开一个可见的（picker 需要用户交互）
        if _context is None:
            await _launch({"headless": False})
        else:
            # 浏览器在但没页面 → 新建标签页
            try:
                page = await _context.new_page()
                _pages.append(page)
            except Exception as e:
                return {"success": False, "error": f"无法创建新页面: {e}"}
        if page is None:
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


async def _pick_next(params: dict | None = None) -> dict:
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

        # 轮询等待点击（可配置超时，默认 30 秒）
        timeout_ms = (params or {}).get("timeout_ms", 30000)
        iterations = max(1, int(timeout_ms / 100))
        for _ in range(iterations):
            await asyncio.sleep(0.1)
            result = await page.evaluate("window.__wfPickResult")
            if result:
                # Retrieve element info stored by the click handler
                element_info = await page.evaluate("window.__wfPickedElementInfo")
                data = result if isinstance(result, dict) else {"selector": result}
                if element_info:
                    data["element"] = element_info
                return {"success": True, "data": data}

        return {"success": False, "error": f"拾取超时（{timeout_ms}ms）"}
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



# ═══════════════════════════════════════════
# CDP 事件管道 — 浏览器事件 → stdout NDJSON
# ═══════════════════════════════════════════

async def _start_cdp_listener():
    """启动 CDP 会话，订阅浏览器关键事件"""
    global _cdp_session
    page = _get_page()
    if page is None or async_playwright is None:
        return

    try:
        _cdp_session = await page.context.new_cdp_session(page)
    except Exception as e:
        print(json.dumps({"type": "event", "event": "cdp.error",
                          "data": {"error": f"CDP 会话创建失败: {e}"}}), flush=True)
        return

    # 订阅事件列表
    event_names = [
        "Network.responseReceived",
        "Network.requestWillBeSent",
        "Network.loadingFinished",
        "Network.loadingFailed",
        "Runtime.exceptionThrown",
        "Runtime.consoleAPICalled",
        "Page.frameNavigated",
        "Page.loadEventFired",
        "Page.frameStoppedLoading",
        "Page.domContentEventFired",
    ]

    for name in event_names:
        domain = name.split(".")[0]
        try:
            await _cdp_session.send(f"{domain}.enable")
        except Exception:
            pass
        _cdp_session.on(name, lambda params, n=name: _on_cdp_event(n, params))

    print(json.dumps({"type": "event", "event": "cdp.ready",
                      "data": {"subscribed": len(event_names)}}), flush=True)


def _on_cdp_event(event_name: str, params: dict):
    """CDP 事件回调 → 全局队列（CPython GIL 保证 deque 操作线程安全）"""
    global _cdp_queue
    _cdp_queue.append(json.dumps({
        "type": "event",
        "event": event_name,
        "data": params,
    }))


async def _drain_cdp_events():
    """排空事件队列到 stdout（main 循环每次处理后调用）"""
    global _cdp_queue
    while _cdp_queue:
        try:
            msg = _cdp_queue.popleft()
        except IndexError:
            break
        print(msg, flush=True)


async def _stop_cdp_listener():
    """停止 CDP 监听"""
    global _cdp_session
    if _cdp_session:
        try:
            await _cdp_session.detach()
        except Exception:
            pass
        _cdp_session = None


# ═══════════════════════════════════════════════
# v1.7 办公自动化新动作
# ═══════════════════════════════════════════════

async def _upload(params: dict) -> dict:
    """文件上传：设置 input[type=file] 的文件

    params:
      selector: 文件输入框选择器（通常是 input[type=file]）
      file_paths: 文件路径列表（str 或 [str]）
    """
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector", "")
    if not selector:
        return {"success": False, "error": "upload 缺少 selector 参数"}

    file_paths = params.get("file_paths", params.get("file_path", ""))
    if isinstance(file_paths, str):
        file_paths = [file_paths]

    # 校验文件存在
    for p in file_paths:
        if not os.path.isfile(p):
            return {"success": False, "error": f"文件不存在: {p}"}

    try:
        await page.set_input_files(selector, file_paths)
        return {"success": True, "data": {"uploaded": file_paths}}
    except Exception as e:
        return {"success": False, "error": f"文件上传失败: {e}"}


async def _keyboard(params: dict) -> dict:
    """键盘操作：按键或快捷键

    params:
      key: 按键名（如 "Enter", "Tab", "Escape", "Control+a", "Control+s"）
      text: 可选，直接输入文本（不经过元素）
      delay: 按键间隔毫秒（默认 0）
    """
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    key = params.get("key", "")
    text = params.get("text", "")
    delay = params.get("delay", 0)

    try:
        if key:
            await page.keyboard.press(key, delay=delay)
        if text:
            await page.keyboard.type(text, delay=delay)
        return {"success": True, "data": {"key": key, "text": text}}
    except Exception as e:
        return {"success": False, "error": f"键盘操作失败: {e}"}


async def _double_click(params: dict) -> dict:
    """双击元素

    params:
      selector: CSS 选择器
      timeout_ms: 超时毫秒（默认 5000）
    """
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector", "")
    if not selector:
        return {"success": False, "error": "double_click 缺少 selector 参数"}

    timeout_ms = params.get("timeout_ms", 5000)

    try:
        await page.dblclick(selector, timeout=timeout_ms)
        return {"success": True, "data": {"selector": selector}}
    except Exception as e:
        return {"success": False, "error": f"双击失败: {e}"}


async def _drag_to(params: dict) -> dict:
    """拖拽：从源元素拖到目标元素

    params:
      source: 源元素选择器
      target: 目标元素选择器
      source_position: 可选，源元素内相对位置 {"x": 0, "y": 0}
      target_position: 可选，目标元素内相对位置 {"x": 0, "y": 0}
    """
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    source = params.get("source", "")
    target = params.get("target", "")
    if not source or not target:
        return {"success": False, "error": "drag_to 缺少 source 或 target 参数"}

    source_pos = params.get("source_position")
    target_pos = params.get("target_position")

    try:
        # Playwright drag_to
        src_el = await page.query_selector(source)
        if not src_el:
            return {"success": False, "error": f"源元素未找到: {source}"}
        tgt_el = await page.query_selector(target)
        if not tgt_el:
            return {"success": False, "error": f"目标元素未找到: {target}"}

        kwargs = {}
        if source_pos:
            kwargs["source_position"] = source_pos
        if target_pos:
            kwargs["target_position"] = target_pos

        await src_el.drag_to(tgt_el, **kwargs)
        return {"success": True, "data": {"source": source, "target": target}}
    except Exception as e:
        return {"success": False, "error": f"拖拽失败: {e}"}


async def _context_menu(params: dict) -> dict:
    """右键菜单：在指定元素上触发右键

    params:
      selector: CSS 选择器
      timeout_ms: 超时毫秒（默认 5000）
    """
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector", "")
    if not selector:
        return {"success": False, "error": "context_menu 缺少 selector 参数"}

    timeout_ms = params.get("timeout_ms", 5000)

    try:
        await page.click(selector, button="right", timeout=timeout_ms)
        return {"success": True, "data": {"selector": selector}}
    except Exception as e:
        return {"success": False, "error": f"右键菜单失败: {e}"}


async def _switch_frame(params: dict) -> dict:
    """iframe 切换：切换到指定 iframe 或回到主文档

    params:
      selector: iframe 选择器（为空或 "main" 则回到主文档）
    """
    global _current_frame
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector", "")

    try:
        if not selector or selector == "main":
            # 回到主文档
            _current_frame = None
            return {"success": True, "data": {"frame": "main"}}

        frame = page.frame_locator(selector)
        if frame is None:
            return {"success": False, "error": f"iframe 未找到: {selector}"}
        _current_frame = frame
        return {"success": True, "data": {"frame": selector}}
    except Exception as e:
        return {"success": False, "error": f"iframe 切换失败: {e}"}


async def _handle_dialog(params: dict) -> dict:
    """弹窗处理：接受或拒绝 alert/confirm/prompt

    params:
      action: "accept" 或 "reject"（默认 accept）
      prompt_text: 可选，prompt 弹窗的输入文本
    """
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    action = params.get("action", "accept")
    prompt_text = params.get("prompt_text", "")

    try:
        # 注册一次性对话框处理器
        async def handler(dialog):
            if action == "accept":
                if prompt_text:
                    await dialog.accept(prompt_text)
                else:
                    await dialog.accept()
            else:
                await dialog.dismiss()

        page.once("dialog", lambda d: asyncio.create_task(handler(d)))
        return {"success": True, "data": {"action": action}}
    except Exception as e:
        return {"success": False, "error": f"弹窗处理设置失败: {e}"}


async def _scroll_to_element(params: dict) -> dict:
    """滚动到指定元素：确保元素可见

    params:
      selector: CSS 选择器
      behavior: "smooth" 或 "instant"（默认 instant）
      block: "start", "center", "end", "nearest"（默认 center）
    """
    page = _get_page()
    if page is None:
        return {"success": False, "error": "浏览器未启动"}

    selector = params.get("selector", "")
    if not selector:
        return {"success": False, "error": "scroll_to_element 缺少 selector 参数"}

    behavior = params.get("behavior", "instant")
    block = params.get("block", "center")

    try:
        await page.evaluate(f"""
            document.querySelector({json.dumps(selector)})
                ?.scrollIntoView({{behavior: "{behavior}", block: "{block}"}})
        """)
        return {"success": True, "data": {"selector": selector}}
    except Exception as e:
        return {"success": False, "error": f"滚动到元素失败: {e}"}


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
            # 收集 CDP 事件（不再 drain 到 stdout，而是嵌入响应）
            cdp_events = []
            while _cdp_queue:
                try:
                    cdp_events.append(json.loads(_cdp_queue.popleft()))
                except (IndexError, json.JSONDecodeError):
                    break
            response = {"id": req_id, "events": cdp_events, **result}
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
