#!/usr/bin/env python3
"""Desktop Recorder Sidecar — Windows 桌面键鼠录制器

通过 stdin/stdout JSON 协议与 Rust 核心通信。
每行一个 JSON 对象（NDJSON 协议）。

请求 → {"id":"uuid","action":"start","params":{}}
响应 ← {"id":"uuid","success":true,"data":{"actions":[...]}}

支持的操作:
  start              — 开始录制全局键鼠操作
  stop               — 停止录制，返回操作列表
  status             — 查看录制状态
  shutdown           — 关闭 sidecar

录制原理：
  使用 Windows SetWindowsHookEx API 通过 ctypes 捕获全局键盘和鼠标事件。
  不需要额外的依赖包，仅使用 Python 标准库。

限制：
  - 仅支持 Windows（依赖 user32.dll）
  - 某些安全软件可能阻止全局钩子
  - 录制时不应执行敏感操作（密码等会在操作列表中明文记录）
"""
import sys
import os
import json
import asyncio
import ctypes
from ctypes import wintypes
import time
import threading

# ═══════════════════════════════════════════
# Windows API 类型定义
# ═══════════════════════════════════════════

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

# Hook 类型常量
WH_KEYBOARD_LL = 13
WH_MOUSE_LL = 14
WM_KEYDOWN = 0x0100
WM_KEYUP = 0x0101
WM_SYSKEYDOWN = 0x0104
WM_SYSKEYUP = 0x0105
WM_LBUTTONDOWN = 0x0201
WM_LBUTTONUP = 0x0202
WM_RBUTTONDOWN = 0x0204
WM_RBUTTONUP = 0x0205
WM_MBUTTONDOWN = 0x0207
WM_MBUTTONUP = 0x0208
WM_MOUSEWHEEL = 0x020A
WM_MOUSEMOVE = 0x0200

# 虚拟键码 → 可读名称
VK_NAMES = {
    0x08: "Backspace", 0x09: "Tab", 0x0D: "Enter",
    0x10: "Shift", 0x11: "Ctrl", 0x12: "Alt",
    0x13: "Pause", 0x14: "CapsLock", 0x1B: "Escape",
    0x20: "Space", 0x21: "PageUp", 0x22: "PageDown",
    0x23: "End", 0x24: "Home",
    0x25: "Left", 0x26: "Up", 0x27: "Right", 0x28: "Down",
    0x2D: "Insert", 0x2E: "Delete",
    0x5B: "LWin", 0x5C: "RWin",
    0x70: "F1", 0x71: "F2", 0x72: "F3", 0x73: "F4",
    0x74: "F5", 0x75: "F6", 0x76: "F7", 0x77: "F8",
    0x78: "F9", 0x79: "F10", 0x7A: "F11", 0x7B: "F12",
}

# 修饰键记录
modifiers = {"ctrl": False, "alt": False, "shift": False}

# ═══════════════════════════════════════════
# 录制状态
# ═══════════════════════════════════════════

recording = False
recorded_actions: list = []
last_mouse_move_time = 0
MOUSE_MOVE_THROTTLE_MS = 200  # 鼠标移动采样的最小间隔（ms）
text_buffer = ""  # 键盘输入文本缓冲
last_key_time = 0
KEY_FLUSH_TIMEOUT = 1.0  # 按键文本缓冲刷新超时（秒）

# Hook 句柄
_keyboard_hook_id = None
_mouse_hook_id = None

# LowLevelKeyboardProc 回调类型
KBDLLHOOKSTRUCT = ctypes.Structure  # 前向声明

# 回调函数原型
HOOKPROC = ctypes.WINFUNCTYPE(ctypes.c_long, ctypes.c_int, wintypes.WPARAM, ctypes.c_void_p)

# 保持回调引用防止被 GC
_keyboard_proc = None
_mouse_proc = None


def _get_key_name(vk_code: int) -> str:
    """获取虚拟键的可读名称"""
    if vk_code in VK_NAMES:
        return VK_NAMES[vk_code]
    # 使用 ToUnicode 获取字符
    return ""


def _get_key_char(vk_code: int) -> str:
    """获取键码对应的字符（考虑当前键盘布局）"""
    try:
        buf = ctypes.create_unicode_buffer(5)
        keyboard_state = (ctypes.c_ubyte * 256)()
        # 设置 Shift 状态
        if modifiers["shift"]:
            keyboard_state[0x10] = 0x80
        if modifiers["ctrl"]:
            keyboard_state[0x11] = 0x80
        result = user32.ToUnicode(vk_code, 0, keyboard_state, buf, 5, 0)
        if result > 0:
            return buf.value
    except:
        pass
    return ""


def _add_action(action_type: str, **kwargs):
    """添加一条录制操作"""
    global recorded_actions
    action = {
        "type": action_type,
        "source": "desktop",
        "timestamp": int(time.time() * 1000),
        **kwargs,
    }
    recorded_actions.append(action)


def _handle_key_down(vk_code: int):
    """处理键盘按下事件"""
    global text_buffer, last_key_time, modifiers

    # 更新修饰键状态
    if vk_code == 0x10:
        modifiers["shift"] = True
        return
    elif vk_code == 0x11:
        modifiers["ctrl"] = True
        return
    elif vk_code == 0x12:
        modifiers["alt"] = True
        return

    now = time.time()

    # 快捷键（Ctrl/Alt + 键）
    if modifiers["ctrl"] or modifiers["alt"]:
        parts = []
        if modifiers["ctrl"]:
            parts.append("Ctrl")
        if modifiers["alt"]:
            parts.append("Alt")
        if modifiers["shift"]:
            parts.append("Shift")

        key_name = _get_key_name(vk_code) or chr(vk_code) if 0x30 <= vk_code <= 0x5A else f"VK{vk_code}"
        parts.append(key_name)

        # 检查是否为停止快捷键（Ctrl+Shift+R）
        if modifiers["ctrl"] and modifiers["shift"] and vk_code == ord('R'):
            return  # 停止快捷键不作为操作记录

        _add_action("hotkey", keys="+".join(parts))
        return

    # 获取输入字符
    char = _get_key_char(vk_code)

    # 特殊处理：Enter / Tab / 退格
    if vk_code == 0x0D:  # Enter
        if text_buffer:
            _add_action("type", text=text_buffer)
            text_buffer = ""
        _add_action("hotkey", keys="Enter")
        return
    elif vk_code == 0x09:  # Tab
        if text_buffer:
            _add_action("type", text=text_buffer)
            text_buffer = ""
        _add_action("hotkey", keys="Tab")
        return
    elif vk_code == 0x08:  # Backspace
        text_buffer = text_buffer[:-1] if text_buffer else ""
        return

    if char and not modifiers["ctrl"] and not modifiers["alt"]:
        # 文本输入：合并到缓冲区（连续输入合并为一条 type 操作）
        if now - last_key_time > KEY_FLUSH_TIMEOUT and text_buffer:
            _add_action("type", text=text_buffer)
            text_buffer = ""
        text_buffer += char
        last_key_time = now


def _handle_key_up(vk_code: int):
    """处理键盘释放事件"""
    global modifiers, text_buffer
    if vk_code == 0x10:
        modifiers["shift"] = False
    elif vk_code == 0x11:
        modifiers["ctrl"] = False
    elif vk_code == 0x12:
        modifiers["alt"] = False


def _keyboard_hook(nCode: int, wParam: wintypes.WPARAM, lParam: ctypes.c_void_p) -> int:
    """低级键盘钩子回调"""
    global recording, text_buffer

    if nCode >= 0 and recording:
        vk_code = ctypes.cast(lParam, ctypes.POINTER(ctypes.c_long)).contents.value

        # 停止快捷键: Ctrl+Shift+R → 忽略此按键
        if wParam == WM_KEYDOWN and modifiers["ctrl"] and modifiers["shift"] and vk_code == 0x52:
            # 不记录，直接透传
            pass
        elif wParam in (WM_KEYDOWN, WM_SYSKEYDOWN):
            _handle_key_down(vk_code)
        elif wParam in (WM_KEYUP, WM_SYSKEYUP):
            _handle_key_up(vk_code)

    return user32.CallNextHookEx(None, nCode, wParam, lParam)


def _handle_mouse(wParam: wintypes.WPARAM, lParam: ctypes.c_void_p):
    """处理鼠标事件"""
    global last_mouse_move_time

    # 获取鼠标坐标
    point = ctypes.c_long(lParam)
    x = point.value & 0xFFFF
    y = (point.value >> 16) & 0xFFFF

    # 处理有符号坐标（高位扩展）
    if x >= 32768:
        x -= 65536
    if y >= 32768:
        y -= 65536

    now = int(time.time() * 1000)

    if wParam == WM_LBUTTONDOWN:
        _add_action("click", x=float(x), y=float(y), button="left")
    elif wParam == WM_RBUTTONDOWN:
        _add_action("click", x=float(x), y=float(y), button="right")
    elif wParam == WM_MBUTTONDOWN:
        _add_action("click", x=float(x), y=float(y), button="middle")
    elif wParam == WM_MOUSEWHEEL:
        # 滚轮数据在高位
        delta = (point.value >> 16) & 0xFFFF
        if delta >= 32768:
            delta -= 65536
        scroll_amount = delta // 120  # WHEEL_DELTA = 120
        _add_action("scroll", amount=int(scroll_amount))
    elif wParam == WM_MOUSEMOVE:
        # 鼠标移动：采样节流
        if now - last_mouse_move_time >= MOUSE_MOVE_THROTTLE_MS:
            _add_action("move", x=float(x), y=float(y))
            last_mouse_move_time = now


def _mouse_hook(nCode: int, wParam: wintypes.WPARAM, lParam: ctypes.c_void_p) -> int:
    """低级鼠标钩子回调"""
    global recording
    if nCode >= 0 and recording:
        _handle_mouse(wParam, lParam)
    return user32.CallNextHookEx(None, nCode, wParam, lParam)


def _start_recording():
    """启动键盘和鼠标钩子"""
    global _keyboard_proc, _mouse_proc, _keyboard_hook_id, _mouse_hook_id

    if recording:
        return {"success": False, "error": "已在录制中"}

    try:
        # 检查是否为 Windows
        if sys.platform != "win32":
            return {"success": False, "error": "桌面录制仅支持 Windows"}

        _keyboard_proc = HOOKPROC(_keyboard_hook)
        _mouse_proc = HOOKPROC(_mouse_hook)

        module_handle = kernel32.GetModuleHandleW(None)

        _keyboard_hook_id = user32.SetWindowsHookExW(
            WH_KEYBOARD_LL, _keyboard_proc, module_handle, 0
        )
        _mouse_hook_id = user32.SetWindowsHookExW(
            WH_MOUSE_LL, _mouse_proc, module_handle, 0
        )

        if not _keyboard_hook_id:
            return {"success": False, "error": f"键盘钩子注册失败，错误码: {kernel32.GetLastError()}"}
        if not _mouse_hook_id:
            user32.UnhookWindowsHookEx(_keyboard_hook_id)
            return {"success": False, "error": f"鼠标钩子注册失败，错误码: {kernel32.GetLastError()}"}

        global recording
        recording = True
        return {"success": True, "data": {"message": "录制已开始"}}

    except Exception as e:
        return {"success": False, "error": f"启动录制失败: {e}"}


def _stop_recording():
    """停止录制，卸载钩子"""
    global recording, text_buffer, recorded_actions

    if not recording:
        return {"success": False, "error": "未在录制中"}

    # 刷新文本缓冲
    if text_buffer:
        _add_action("type", text=text_buffer)
        text_buffer = ""

    # 卸载钩子
    if _keyboard_hook_id:
        user32.UnhookWindowsHookEx(_keyboard_hook_id)
    if _mouse_hook_id:
        user32.UnhookWindowsHookEx(_mouse_hook_id)

    recording = False

    # 后处理：合并相邻的 scroll 操作
    actions = _post_process(recorded_actions)
    count = len(actions)
    recorded_actions = []  # 清空

    return {
        "success": True,
        "data": {
            "message": f"录制已停止，共 {count} 个操作",
            "actions": actions,
            "count": count,
        },
    }


def _get_status():
    """获取录制状态"""
    return {
        "success": True,
        "data": {
            "recording": recording,
            "action_count": len(recorded_actions),
            "modifiers": dict(modifiers),
        },
    }


def _post_process(actions: list) -> list:
    """后处理：合并连续 scroll"""
    if not actions:
        return actions

    merged = []
    pending_scroll = 0

    for action in actions:
        if action["type"] == "scroll":
            pending_scroll += action.get("amount", 0)
        else:
            if pending_scroll != 0:
                merged.append({
                    "type": "scroll",
                    "source": "desktop",
                    "amount": pending_scroll,
                    "timestamp": action.get("timestamp", 0),
                })
                pending_scroll = 0
            merged.append(action)

    # 处理末尾滚动
    if pending_scroll != 0:
        merged.append({
            "type": "scroll",
            "source": "desktop",
            "amount": pending_scroll,
            "timestamp": int(time.time() * 1000),
        })

    return merged


# ═══════════════════════════════════════════
# 主循环
# ═══════════════════════════════════════════

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
                _stop_recording()
                response = {"id": req_id, "success": True, "data": {"message": "关闭"}}
                print(json.dumps(response), flush=True)
                break

            elif action == "start":
                result = _start_recording()
            elif action == "stop":
                result = _stop_recording()
            elif action == "status":
                result = _get_status()
            else:
                result = {"success": False, "error": f"未知操作: {action}"}

            response = {"id": req_id, **result}
        except json.JSONDecodeError as e:
            response = {"id": "", "success": False, "error": f"JSON 解析错误: {e}"}
        except Exception as e:
            import traceback
            response = {
                "id": request.get("id", "") if "request" in dir() else "",
                "success": False,
                "error": str(e),
                "traceback": traceback.format_exc(),
            }

        print(json.dumps(response), flush=True)


if __name__ == "__main__":
    asyncio.run(main())
