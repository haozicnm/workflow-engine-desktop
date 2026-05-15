# Workflow Engine Desktop — 跨平台改造方案对比

> 创建于 2026-04-27 | 目标：同时适配 Windows 和 Linux

---

## 当前状态速览

| 模块 | Windows | Linux | 跨平台方式 |
|------|:--:|:--:|------|
| 键鼠操作 | PowerShell + user32.dll | enigo crate | ✅ trait 已统一 |
| 桌面录制 | desktop_recorder.py (sidecar) | rdev crate | ⚠️ trait 已定义，Windows 未走 trait |
| 窗口管理 | PowerShell | **缺失** | 🔴 无 trait，250 行硬编码 PS |
| Tauri 框架 | ✅ | ✅ | 自带跨平台 |
| 浏览器 / Excel / Word | ✅ | ✅ | calamine / Playwright 自带 |

---

## 四种方案对比

### 总览

| 维度 | A: CLI 工具 | B: 纯 Rust 绑定 | C: Python 桥接 | D: RPA 原生库 |
|------|:-----------:|:---------------:|:-------------:|:-----------:|
| **新增代码** | ~300 行 | ~800 行 | ~200 行 | ~500 行 |
| **外部依赖** | xdotool / wmctrl | 0（编译进二进制） | Python 3.8+ | libxdo / uinput |
| **二进制大小** | +0 KB | +2~5 MB | +0 KB | +200 KB |
| **X11 支持** | ✅ 完美 | ✅ | ✅ | ✅ 原生 |
| **Wayland 支持** | ⚠️ 换 ydotool | ⚠️ 需额外实现 | ⚠️ 有限 | ⚠️ 有限 |
| **开发周期** | 1-2 天 | 3-5 天 | 1 天 | 2-3 天 |
| **可维护性** | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ |
| **用户体验** | 需 apt install | 开箱即用 | 需 Python 环境 | 开箱即用 |

---

### 方案 A：CLI 工具调用 ⭐ 推荐

**思路**：Rust 通过 `std::process::Command` 调用系统级工具。

| 平台 | 窗口管理 | 安装 |
|------|---------|------|
| Linux X11 | `xdotool` + `wmctrl` | `apt install xdotool wmctrl` |
| Linux Wayland | `ydotool` + `wlrctl` | `apt install ydotool` |
| Windows | 保留 PowerShell | 已内置 |

```rust
// 示例：Linux 窗口查找
impl WindowBackend for LinuxWindowBackend {
    fn activate(&self, title: &str) -> Result<()> {
        Command::new("xdotool")
            .args(["search", "--name", title, "windowactivate", "windowfocus"])
            .status()?;
        Ok(())
    }
}
```

```
架构示意：

  nodes/window.rs
        │
  platform/traits.rs  ← WindowBackend trait
       ╱        ╲
  Windows          Linux
  PowerShell       xdotool / wmctrl
  + user32.dll
```

**✅ 优点**

- 代码最少（~300 行），和现有 `platform/linux.rs` 的 enigo 模式完全一致
- xdotool 是 Linux 桌面自动化的事实标准，久经考验
- 调试方便：命令行直接测试，不需要重新编译
- 不增加二进制体积
- 与现有 trait 架构无缝衔接

**❌ 缺点**

- 用户需额外安装系统工具（启动时可检测 + 引导安装）
- Wayland 需要换用 ydotool（trait 里做 X11/Wayland 检测即可）

---

### 方案 B：纯 Rust 原生绑定

**思路**：直接链接 X11/Wayland 的 Rust binding crate。

| 平台 | Crate | 编译产物 |
|------|-------|---------|
| Linux X11 | `x11rb` | ~2 MB |
| Linux Wayland | `wayland-client` + `smithay-client-toolkit` | ~3 MB |
| Windows | 保留 PS 或加 `windows-rs` | ~5 MB |

```rust
// 示例：X11 窗口查找（需约 60 行）
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

impl WindowBackend for X11Backend {
    fn find(&self, title: &str) -> Result<Vec<WindowInfo>> {
        let (conn, _) = RustConnection::connect(None)?;
        // 遍历窗口树，匹配标题...
    }
}
```

**✅ 优点**

- 零外部依赖，二进制自包含，下载即用
- 类型安全，编译期检查
- 用户体验最好

**❌ 缺点**

- `x11rb` 学习曲线陡峭，X11 协议非常底层
- Wayland 是完全不同的协议，需要两套代码
- 二进制膨胀 2-5 MB
- 开发周期长（3-5 天），调试困难

---

### 方案 C：Python 桥接

**思路**：所有平台操作走 Python sidecar，Rust 只做调度。

```
nodes/window.rs → spawn python → pygetwindow / pyautogui → 返回 JSON
```

```python
# 窗口管理全部在 Python 端
import subprocess, json

def find_windows(title: str):
    result = subprocess.run(
        ["xdotool", "search", "--name", title],
        capture_output=True, text=True
    )
    return json.dumps(parse_windows(result.stdout))
```

**✅ 优点**

- 代码最少（~200 行），Python 生态丰富
- 和现有 `desktop_recorder.py` 风格一致
- 跨平台逻辑集中在 Python

**❌ 缺点**

- 运行时依赖 Python（用户必须装）
- subprocess 开销：每次窗口操作都要启 Python 进程
- 两套语言维护负担
- 和现有 trait 架构理念不一致（之前辛苦把键鼠操作从 PS 搬到 trait，窗口又开倒车）

---

### 方案 D：RPA 专用系统库

**思路**：调用 Linux 底层的输入/窗口子系统接口。

| 层级 | 库 |
|------|-----|
| 窗口操作 | `libxdo`（C 库，xdotool 的底层实现） |
| 输入事件 | `uinput` + `evdev`（Linux 内核接口） |

```rust
// libxdo 的 Rust FFI 调用
extern "C" {
    fn xdo_search_windows(xdo: *mut xdo_t, ...) -> i32;
}

impl WindowBackend for XdoWindowBackend {
    fn find(&self, title: &str) -> Result<Vec<WindowInfo>> {
        unsafe { /* FFI 调用 */ }
    }
}
```

**✅ 优点**

- 性能最优：直接系统调用
- 比纯 X11 协议简单（libxdo 封装好了）
- 专业 RPA 工具的做法

**❌ 缺点**

- 需要 FFI + unsafe Rust
- 需要编译 C 依赖（libxdo-dev）
- 仍然要区分 X11/Wayland
- 过度工程化（桌面自动化不需要这个级别的性能）

---

## 推荐方案：A（CLI 工具）+ 渐进增强

### 理由

1. 和现有 `InputBackend` trait 架构**完全一致**，团队没有认知负担
2. 代码最少、风险最低、交付最快
3. trait 抽象层保证了**未来可以无痛替换**底层实现为方案 B 或 D
4. xdotool 几乎每个 Linux 桌面用户都已安装
5. 如果未来觉得外部依赖是痛点，只需替换 `platform/linux_window.rs` 一个文件

### 实施路径

```
阶段 1（半天）  → 加 WindowBackend trait + Linux xdotool 实现
阶段 2（1小时）  → nodes/window.rs 改为调 trait
阶段 3（2小时）  → Windows 录制包装进 RecordingBackend trait
阶段 4（1小时）  → 启动时检测 xdotool 是否存在，提示安装
阶段 5（可选）    → Wayland 检测 + ydotool 降级路径
阶段 6（未来可选）→ 替换为纯 Rust 实现，零外部依赖
```

### 涉及文件

| 文件 | 操作 | 行数 |
|------|------|:--:|
| `platform/traits.rs` | 新增 `WindowBackend` trait | +30 |
| `platform/mod.rs` | 注册全局 WindowBackend 单例 | +15 |
| `platform/windows_window.rs` | 搬现 window.rs 的 PS 代码 | ~120（重构） |
| `platform/linux_window.rs` | xdotool 实现 | ~100（新增） |
| `nodes/window.rs` | 改为 `platform::window()` 调用 | ~60（重构） |
| `platform/recording.rs` | 加 Windows RecordingBackend | +50 |
| **合计** | | **~375 行** |

---

## 结论

| 方案 | 适合场景 |
|------|---------|
| **A（CLI 工具）** | 🏆 追求快速交付、低风险、和现有架构一致 |
| B（纯 Rust） | 追求极致用户体验、零依赖、愿意投入时间 |
| C（Python 桥接） | 团队以 Python 为主、不在意两语言维护 |
| D（系统库） | 需要极致性能、专业 RPA 产品定位 |

**对于 Workflow Engine Desktop 当前阶段，方案 A 是最务实的选择。**
