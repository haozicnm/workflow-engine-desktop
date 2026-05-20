# UI-TARS 学习改进方案

> 基于 [bytedance/UI-TARS-desktop](https://github.com/bytedance/UI-TARS-desktop)（34.7k⭐）架构分析
> 2026-05-19

---

## 一、当前架构 vs 目标架构

```
当前（workflow-engine）              目标（借鉴 UI-TARS）
─────────────────────────          ─────────────────────────
executor.rs                         executor.rs
  ├── register!(browser...)           └── OperatorRegistry
  ├── StepExecutor::execute()              ├── BrowserOp
  └── 直接调 parser                        ├── ExcelOp
      ↓                                    ├── WordOp
parser.rs                                   └── FileOp
  ├── convert_step()                   parser/
  └── 大段 if/else                         ├── StepParser (trait)
      ↓                                    ├── ContainerParser
nodes/                                      ├── IterationParser
  ├── browser_container.rs                  └── SimpleStepParser
  ├── excel_container.rs              actions/
  └── ...                                  ├── ActionDef (强类型)
                                           └── ActionMetadata (注册表)
```

---

## 二、四个改进方向（ABCD 顺序）

### A. Operator 抽象层 — 统一容器生命周期

**现状问题**：
- 每个容器节点直接注册到 executor，没有统一的 Operator 概念
- 初始化逻辑散落在各处（browser sidecar 启动、Excel 文件打开等）
- 没有 `supported_actions()` 声明——前端和后台各自维护一套 action 列表

**改进方案**：

```rust
/// Rust 侧 Operator trait
#[async_trait]
pub trait Operator: Send + Sync {
    /// 返回支持的 action 类型列表
    fn supported_actions(&self) -> &[&str];

    /// 惰性初始化（首次调用时触发）
    async fn ensure_initialized(&mut self) -> Result<()>;

    /// 执行单个 action
    async fn execute_action(
        &mut self,
        action: &Action,
        ctx: &mut ExecutionContext,
    ) -> Result<serde_json::Value>;

    /// 清理资源（Drop 时调用）
    async fn cleanup(&mut self) -> Result<()>;
}

// 具体实现
struct BrowserOperator { sidecar: Option<BrowserSidecar>, ... }
struct ExcelOperator { workbook: Option<ExcelWorkbook>, ... }
```

**优先级**：⚠️ 中等 — 改善架构但不影响功能

**收益**：
- 容器生命周期统一管理（init / execute / cleanup）
- `supported_actions()` → 前端自动同步，不再手动维护 action 列表
- 新加容器只需实现 trait，注册一行

---

### B. Action 类型强化 — 从 `Record<string, unknown>` 到强类型

**现状问题**：
- 前端和后端 action params 都是 `Record<string, unknown>`，无编译期类型检查
- 参数名拼写错误（`file_path` vs `filePath`）只能运行时发现
- 没有 category/description 元数据，前端全靠硬编码

**改进方案**：

```rust
/// 带元数据的 Action 定义
struct ActionDef {
    action_type: &'static str,    // "navigate", "click", "read_cell"
    category: ActionCategory,     // Mouse, Keyboard, File, Data
    description: &'static str,
    params: &'static [ParamDef],
    output_hint: &'static str,    // "{ url, title }"
}

enum ActionCategory {
    Navigation,
    Interaction,  // click, input, scroll
    DataRead,
    DataWrite,
    System,       // delay, notify
}
```

前端同步生成 TS 类型：

```typescript
// 自动从 Rust ActionDef 生成
export const ACTION_METADATA: Record<string, ActionMeta> = {
  navigate: { category: 'navigation', params: ['url'] },
  click:    { category: 'interaction', params: ['selector'] },
  read:     { category: 'data_read', params: ['cell', 'range'] },
}
```

**优先级**：🔴 高 — 直接减少 bug

**收益**：
- 编译期类型检查消灭参数拼写错误
- category 字段 → 前端自动分组展示（不再硬编码下拉顺序）
- output_hint → 统一变量引用提示

---

### C. Parser 责任链 — 消灭大段 if/else

**现状问题**：
- `parser.rs` 的 `convert_step()` 用大量 if/else 判断类型：
  ```
  if is_container → 类型名透传（v8）
  if is_iteration → 处理 body_steps
  if is_recursive → 跳过 actions 迁移
  if logic → conditionGroup 转换
  ```
- 加一种新节点类型要改 4 处

**改进方案**：

```rust
/// StepParser trait
trait StepParser {
    /// 是否能处理此类型
    fn can_parse(step_type: &str) -> bool;

    /// 解析步骤
    fn parse(step: &Step, is_recursive: bool) -> Result<ParsedStep>;
}

// 责任链
struct ParserChain {
    parsers: Vec<Box<dyn StepParser>>,
}

impl ParserChain {
    fn parse_step(&self, step: &Step, is_recursive: bool) -> Result<ParsedStep> {
        for parser in &self.parsers {
            if parser.can_parse(&step.step_type) {
                return parser.parse(step, is_recursive);
            }
        }
        Err(anyhow!("未知步骤类型: {}", step.step_type))
    }
}

// 具体 parser
struct ContainerParser;   // browser, excel, word, file, logic
struct IterationParser;   // cursor, loop, while
struct SimpleStepParser;  // http, delay, notify, script, shell
```

**优先级**：🔴 高 — 当前 parser 是新增节点的主要摩擦点

**收益**：
- 加新节点 → 只需新建一个 parser 文件 + 注册到链
- 每种节点的解析逻辑完全隔离，不会互相影响
- `can_parse()` 即文档

---

### D. 坐标归一化 — 浏览器元素定位改进

**现状问题**：
- 浏览器容器只支持 CSS selector 定位
- 模型输出的归一化坐标（VLM 常见输出格式）无法直接使用
- 同一元素在不同分辨率下 selector 可能失效

**改进方案**：

```rust
/// 元素定位目标（借鉴 UI-TARS Coordinates）
struct ElementTarget {
    /// CSS/XPath 选择器（优先使用）
    selector: Option<String>,
    /// 归一化坐标 (0-1)，降级方案
    coordinate: Option<NormalizedPoint>,
    /// 参考系统
    reference: ReferenceSystem,
}

struct NormalizedPoint { x: f64, y: f64 }

enum ReferenceSystem {
    Screen,         // 相对于整个屏幕
    Viewport,       // 相对于浏览器视口
    Element(u32),   // 相对于指定元素（用于嵌套 iframe）
}
```

**优先级**：🔵 低 — 锦上添花，当前 selector 模式够用

---

## 三、实施路线

```
A: Operator trait       ──→  架构改善，先搭骨架
B: Action 类型强化       ──→  消灭参数 bug，降低维护成本
C: Parser 责任链         ──→  解除 parser 瓶颈，新节点开发提速
D: 坐标归一化            ──→  浏览器定位增强，等前面稳定再做
```

A→B→C 按依赖顺序执行，D 独立可并行。

---

## 四、不值得借鉴的

| UI-TARS 设计 | 不借鉴原因 |
|-------------|-----------|
| `@tarko/agent` Agent 基类 | 我们的 Agent 不是运行在终端/VLM 循环里，是工作流 DAG 调度 |
| VLM 推理 + 截图循环 | 我们不依赖 VLM，用户手动编排步骤 |
| Electron IPC 层 | 我们用 Tauri，IPC 机制不同 |
| XML/Omni/O1 等多种 prompt 格式 | 我们不需要解析 LLM 输出，用户直接填参数 |
