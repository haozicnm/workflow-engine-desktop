# Hermes Agent 架构升级方案

> 基于 Claude Code 源码分析，提炼可落地的改进点

**目标：** 将 Claude Code 中经过工程验证的 7 个核心模式应用到 Hermes Agent，提升工具执行效率、长对话稳定性和错误恢复能力。

**当前架构现状：**
- Agent 循环：同步 `while` 循环（`run_agent.py:11152`），14,666 行单文件
- 工具执行：已有并发支持（`ThreadPoolExecutor`），但分批策略简单
- 上下文压缩：单级压缩（`context_compressor.py`），75% 阈值触发
- 错误处理：有重试+退避+故障转移，但缺少分级恢复
- 流式：始终流式，但工具执行不流式（等 API 完成才开始）
- 记忆：双层（内置文件 + 外部 provider），结构简单

---

## Phase 1：工具执行效率提升（P0）

### Task 1.1：智能工具分批编排

**现状：** `_should_parallelize_tool_batch()` 只做简单的"全并行或全串行"判断，基于路径重叠和工具白名单。

**Claude Code 做法：** `[Read, Read, Write, Read]` → `[Read,Read] 并行 → [Write] 串行 → [Read] 并行`，自动分区。

**改动：**
- 文件：`run_agent.py`，替换 `_execute_tool_calls()` 的分批逻辑（~line 9778）
- 新增：`agent/tool_batching.py`

```python
# agent/tool_batching.py
from dataclasses import dataclass
from typing import List

@dataclass
class ToolBatch:
    """一组可并行执行的工具调用"""
    calls: list          # tool_call 对象列表
    parallel: bool       # 是否并行
    needs_context_flush: bool  # 执行完是否需要刷新上下文（如写文件后）

def partition_tool_batches(tool_calls: list, registry) -> List[ToolBatch]:
    """
    将工具调用序列分为可并行批次。
    规则：
    1. 只读工具（read_file, search_files, terminal 只读命令）→ 可并行
    2. 写工具（write_file, patch, terminal 写命令）→ 独占串行
    3. 同路径操作 → 严格顺序
    4. 上下文修改器（todo, memory）→ 延迟到批次末尾
    """
    batches: List[ToolBatch] = []
    current_parallel = []
    current_serial = []

    for call in tool_calls:
        tool_entry = registry.get(call.function.name)
        is_read_only = tool_entry and getattr(tool_entry, 'is_read_only', False)
        is_path_scoped = _is_path_scoped_tool(call.function.name)

        if is_read_only and not _has_path_conflict(call, current_parallel):
            # 可并行：加入当前并行批次
            if current_serial:
                batches.append(ToolBatch(current_serial, parallel=False, needs_context_flush=False))
                current_serial = []
            current_parallel.append(call)
        else:
            # 需要串行：先提交当前并行批次
            if current_parallel:
                batches.append(ToolBatch(current_parallel, parallel=True, needs_context_flush=False))
                current_parallel = []
            current_serial.append(call)

    # 提交剩余
    if current_parallel:
        batches.append(ToolBatch(current_parallel, parallel=True, needs_context_flush=False))
    if current_serial:
        batches.append(ToolBatch(current_serial, parallel=False, needs_context_flush=False))

    return batches
```

**验证：** 多文件读取操作应自动并行；写文件后读同一文件应自动串行。

---

### Task 1.2：工具结果流式执行

**现状：** 等 API 响应完全结束后才开始执行工具。Claude Code 在 API 流式返回 tool_use 时就开始验证 schema 并排队执行。

**改动：**
- 文件：`run_agent.py`，修改 `_interruptible_streaming_api_call()`（~line 6973）
- 新增：`agent/streaming_tool_executor.py`

```python
# agent/streaming_tool_executor.py
class StreamingToolExecutor:
    """
    状态机：在 API 流式返回过程中提前开始工具执行。
    状态：queued → validating → executing → completed → yielded
    """
    def __init__(self, registry, max_concurrent=5):
        self.registry = registry
        self.max_concurrent = max_concurrent
        self.pending = {}    # tool_call_id → future
        self.results = {}    # tool_call_id → result

    def on_tool_use_chunk(self, tool_call_id, name, partial_args):
        """API 流返回 tool_use 块时调用"""
        if name in self._read_only_tools:
            # 只读工具：args 完整后立即开始执行
            self._validate_and_enqueue(tool_call_id, name, partial_args)

    def on_stream_complete(self, all_tool_calls):
        """流结束时，确保所有工具都已入队"""
        for call in all_tool_calls:
            if call.id not in self.pending:
                self._validate_and_enqueue(call.id, call.name, call.arguments)

    def get_results(self):
        """等待所有工具完成，按原始顺序返回结果"""
        ...
```

**收益：** 多文件读取场景下，工具执行与 API 流式重叠，总延迟减少 30-50%。

---

### Task 1.3：单次迭代工具结果预算

**现状：** 每个工具有 `max_result_size_chars`，但没有批次总预算。一次返回 10 个大文件会直接撑爆上下文。

**改动：**
- 文件：`run_agent.py`，在 `_execute_tool_calls()` 中添加批次预算检查
- 配置：新增 `tool_batch_max_output_tokens`（默认 30K tokens）

```python
def _check_batch_budget(self, results: list, max_tokens: int = 30000):
    """检查单次迭代工具输出总 token 是否超预算"""
    total = sum(estimate_tokens(r.content) for r in results)
    if total > max_tokens:
        # 超出部分截断，标记为 truncated
        self._truncate_batch_results(results, max_tokens)
        logger.warning(f"Tool batch output truncated: {total} > {max_tokens} tokens")
```

**收益：** 防止单次迭代工具输出撑爆上下文窗口。

---

## Phase 2：分级上下文压缩（P1）

### Task 2.1：5 级压缩流水线

**现状：** 单级压缩——超 75% 阈值时用辅助模型摘要中间消息。没有断路器，没有分级策略。

**Claude Code 做法：** 5 级递进压缩，每级有不同成本和效果。

**改动：**
- 文件：`agent/context_compressor.py`，重写 `compress()` 方法
- 新增配置：`context_compression_levels`

```python
# agent/context_compressor.py — 新增分级压缩

class CompressionLevel(Enum):
    SNIP = 1           # 删除旧工具结果（无 LLM 调用，零成本）
    MICRO = 2          # 截断大工具结果（无 LLM 调用，零成本）
    SUMMARIZE = 3      # 用辅助模型摘要中间消息（当前方案）
    REACTIVE = 4       # prompt-too-long 时的紧急压缩（LLM 调用）
    FULL = 5           # 完整对话摘要（最后手段）

class ContextCompressor:
    def compress(self, messages, target_tokens):
        """按级别递进压缩，直到满足目标"""
        for level in CompressionLevel:
            if self._estimate_tokens(messages) <= target_tokens:
                return messages  # 已满足

            if level == CompressionLevel.SNIP:
                messages = self._snip_old_tool_results(messages)
            elif level == CompressionLevel.MICRO:
                messages = self._microcompact_large_results(messages, target_tokens)
            elif level == CompressionLevel.SUMMARIZE:
                messages = self._summarize_middle(messages, target_tokens)
            elif level == CompressionLevel.REACTIVE:
                # 仅在 API 返回 prompt-too-long 时触发
                break
            elif level == CompressionLevel.FULL:
                messages = self._full_compact(messages, target_tokens)

        return messages

    def _snip_old_tool_results(self, messages):
        """删除超过 N 轮前的工具结果，替换为 [结果已省略]"""
        ...

    def _microcompact_large_results(self, messages, target):
        """截断单条超过 2000 字符的工具结果"""
        ...
```

### Task 2.2：压缩断路器

**现状：** 压缩失败会无限重试。

**改动：**
- 文件：`agent/context_compressor.py`

```python
class CompressionCircuitBreaker:
    """连续失败 N 次后停止压缩，避免浪费 API 调用"""
    def __init__(self, max_failures=3, reset_after=300):
        self.failures = 0
        self.max_failures = max_failures
        self.reset_after = reset_after
        self.last_failure = 0

    def should_try(self):
        if self.failures >= self.max_failures:
            if time.time() - self.last_failure > self.reset_after:
                self.failures = 0  # 重置
            else:
                return False
        return True

    def record_success(self):
        self.failures = 0

    def record_failure(self):
        self.failures += 1
        self.last_failure = time.time()
```

### Task 2.3：压缩后状态重注入

**现状：** 压缩后丢失活跃文件引用、计划状态、待执行工具等上下文。

**改动：**
- 文件：`agent/context_compressor.py`，在压缩后重注入关键状态

```python
def _reinject_state(self, messages, compressed_summary):
    """压缩后重注入关键状态到用户消息"""
    reinject_parts = []

    # 活跃文件引用
    if self.active_files:
        reinject_parts.append(f"活跃文件: {', '.join(self.active_files)}")

    # 待办状态
    if self.todo_state:
        reinject_parts.append(f"当前任务: {self.todo_state}")

    # 压缩摘要
    reinject_parts.append(f"之前的对话摘要:\n{compressed_summary}")

    # 注入到最后一条用户消息
    if reinject_parts:
        messages[-1]['content'] += '\n\n' + '\n'.join(reinject_parts)

    return messages
```

---

## Phase 3：输出 token 自动恢复（P1）

### Task 3.1：最大输出 token 升级

**现状：** API 返回截断时直接报错给用户。

**Claude Code 做法：** 默认 8k → 触顶自动升级到 64k，注入 "resume directly" 继续，最多 3 次。

**改动：**
- 文件：`run_agent.py`，修改 API 响应处理（~line 13326）

```python
# run_agent.py — 输出 token 恢复

MAX_OUTPUT_ESCALATION = 3
DEFAULT_MAX_OUTPUT = 8192
ESCALATED_MAX_OUTPUT = 65536

def _handle_max_output_tokens(self, response, messages, escalation_count):
    """处理输出截断：自动升级 token 限制并继续"""
    if escalation_count >= MAX_OUTPUT_ESCALATION:
        return None  # 放弃，返回已生成内容

    # 注入继续指令
    messages.append({"role": "assistant", "content": response.content})
    messages.append({
        "role": "user",
        "content": "Your response was truncated. Continue exactly where you left off. Do not repeat anything."
    })

    # 升级 token 限制
    self._current_max_tokens = ESCALATED_MAX_OUTPUT

    return messages  # 继续循环
```

**收益：** 消除"回答到一半被截断"的问题，用户不再需要手动说"继续"。

---

## Phase 4：错误分级恢复（P2）

### Task 4.1：Withheld Error 模式

**现状：** API 错误直接展示给用户，即使是可以内部恢复的。

**Claude Code 做法：** 可恢复错误（prompt-too-long、max-output-tokens）先不给用户看，内部尝试恢复，全部失败才展示。

**改动：**
- 文件：`run_agent.py`，修改错误处理循环（~line 11484）

```python
# 可恢复错误类型
RECOVERABLE_ERRORS = {
    'context_overflow': 'compress_and_retry',
    'max_output_tokens': 'escalate_and_resume',
    'rate_limit': 'backoff_and_retry',
}

def _classify_and_handle_error(self, error, messages):
    """分级错误处理：可恢复的先不展示"""
    classified = self.error_classifier.classify(error)

    if classified.reason.value in RECOVERABLE_ERRORS:
        handler = RECOVERABLE_ERRORS[classified.reason.value]
        result = getattr(self, f'_recover_{handler}')(error, messages)
        if result:
            return {'recovered': True, 'messages': result}
        # 恢复失败，才展示给用户
        return {'recovered': False, 'error': classified}

    # 不可恢复，直接展示
    return {'recovered': False, 'error': classified}
```

### Task 4.2：工具错误级联取消

**现状：** Bash 工具失败后，同批次其他工具继续执行。

**Claude Code 做法：** Bash 错误通过 AbortController 级联取消同批次并行工具。

**改动：**
- 文件：`agent/streaming_tool_executor.py`

```python
class ToolBatchController:
    """管理一个批次内工具的级联取消"""
    def __init__(self):
        self.abort_controller = AbortController()
        self.critical_tools = {'terminal', 'execute_code'}  # 这些工具失败会级联

    def on_tool_error(self, tool_name, error):
        if tool_name in self.critical_tools:
            # 级联取消同批次其他工具
            self.abort_controller.abort()
            logger.warning(f"Batch cascade cancel due to {tool_name} failure")
```

---

## Phase 5：Hook 系统增强（P2）

### Task 5.1：PreToolUse / PostToolUse Hook

**现状：** 有 `pre_tool_call` 和 `post_tool_call` 插件钩子，但功能有限（只能 block/observe，不能修改参数或结果）。

**Claude Code 做法：** 6 阶段流水线，Hook 可以 allow/deny/modify。

**改动：**
- 文件：`tools/registry.py`，增强 `dispatch()` 方法

```python
@dataclass
class ToolHookResult:
    action: str  # 'allow' | 'deny' | 'modify'
    modified_args: dict = None   # 如果 modify，新参数
    modified_result: str = None  # 如果 modify，新结果
    reason: str = ''

class ToolRegistry:
    def dispatch(self, name, args, context):
        # 1. Schema 验证
        # 2. PreToolUse Hook（可修改 args）
        hook_result = self._run_pre_hooks(name, args, context)
        if hook_result.action == 'deny':
            return {"error": f"Denied: {hook_result.reason}"}
        if hook_result.action == 'modify':
            args = hook_result.modified_args

        # 3. 执行
        result = self._invoke(name, args, context)

        # 4. PostToolUse Hook（可修改 result）
        post_result = self._run_post_hooks(name, args, result, context)
        if post_result.action == 'modify':
            result = post_result.modified_result

        return result
```

**收益：** 用户可以自定义工具行为（如：执行前自动备份、执行后自动格式化代码）。

---

## Phase 6：记忆系统增强（P3）

### Task 6.1：Session Memory

**现状：** 长对话压缩后历史细节丢失。MEMORY.md 和 USER.md 是手动维护的。

**Claude Code 做法：** Session Memory 自动摘要，每 5k token 更新一次，存储在本地文件。

**改动：**
- 新增：`agent/session_memory.py`
- 修改：`run_agent.py`，在循环中定期触发

```python
class SessionMemory:
    """自动会话记忆：长对话摘要"""
    UPDATE_INTERVAL_TOKENS = 5000
    UPDATE_INTERVAL_TOOL_CALLS = 3
    MIN_TOKENS_TO_START = 10000

    def __init__(self, session_id, auxiliary_client):
        self.session_id = session_id
        self.client = auxiliary_client
        self.path = Path(f"~/.hermes/sessions/{session_id}/session_memory.md")
        self.token_count = 0
        self.tool_call_count = 0

    def maybe_update(self, messages):
        """检查是否需要更新 session memory"""
        self.token_count += self._estimate_last_turn_tokens(messages)

        if self.token_count < self.MIN_TOKENS_TO_START:
            return

        self.tool_call_count += 1
        if (self.token_count % self.UPDATE_INTERVAL_TOKENS < 500 or
            self.tool_call_count % self.UPDATE_INTERVAL_TOOL_CALLS == 0):
            self._update(messages)

    def _update(self, messages):
        """用辅助模型生成会话摘要"""
        summary = self.client.summarize(
            messages,
            prompt="Generate a structured session summary with: Active Task, Key Decisions, File Changes, Pending Items"
        )
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.path.write_text(summary)
        self.path.chmod(0o600)
```

### Task 6.2：记忆相关性召回

**现状：** MEMORY.md 全量注入 system prompt，不管当前任务是否相关。

**Claude Code 做法：** 用轻量模型从 MEMORY.md 中选择最多 5 个相关文件。

**改动：**
- 文件：`tools/memory_tool.py`，新增召回逻辑

```python
def recall_relevant_memory(self, current_query, max_files=5):
    """根据当前查询召回相关记忆片段"""
    memory_dir = Path("~/.hermes/memories/")
    all_files = list(memory_dir.glob("*.md"))

    if len(all_files) <= max_files:
        return [f.read_text() for f in all_files]

    # 用轻量模型评分
    scored = []
    for f in all_files:
        content = f.read_text()[:500]  # 只看前 500 字
        score = self._relevance_score(current_query, content)
        scored.append((score, f))

    scored.sort(reverse=True)
    return [f.read_text() for _, f in scored[:max_files]]
```

---

## 优先级总览

| 阶段 | 任务 | 文件 | 改动量 | 预期收益 |
|---|---|---|---|---|
| **P0** | 1.1 智能工具分批 | `run_agent.py` + 新建 `tool_batching.py` | ~150 行 | 多文件操作速度 2-3x |
| **P0** | 1.2 流式工具执行 | `run_agent.py` + 新建 `streaming_tool_executor.py` | ~200 行 | 工具延迟减少 30-50% |
| **P0** | 1.3 批次结果预算 | `run_agent.py` | ~30 行 | 防止上下文溢出 |
| **P1** | 2.1 分级压缩 | `context_compressor.py` | ~200 行 | 长对话稳定性 |
| **P1** | 2.2 压缩断路器 | `context_compressor.py` | ~40 行 | 防止压缩死循环 |
| **P1** | 2.3 状态重注入 | `context_compressor.py` | ~50 行 | 压缩后不丢上下文 |
| **P1** | 3.1 输出 token 恢复 | `run_agent.py` | ~40 行 | 消除回答截断 |
| **P2** | 4.1 错误分级恢复 | `run_agent.py` | ~60 行 | 减少用户被打断 |
| **P2** | 4.2 工具级联取消 | `streaming_tool_executor.py` | ~30 行 | 避免无用工具执行 |
| **P2** | 5.1 Hook 增强 | `registry.py` | ~80 行 | 用户可自定义工具行为 |
| **P3** | 6.1 Session Memory | 新建 `session_memory.py` | ~100 行 | 长对话记忆不丢失 |
| **P3** | 6.2 记忆召回 | `memory_tool.py` | ~50 行 | 减少无关记忆注入 |

**总计改动：** ~1,030 行（新建 3 文件 + 修改 4 文件）

---

## 不做的事（YAGNI）

| Claude Code 有 | 不做原因 |
|---|---|
| AsyncGenerator agent loop | Hermes 已有 ThreadPoolExecutor 并发，重构成本太高收益低 |
| Swarm/多 Agent 协作 | delegate_task 已够用，当前不需要团队模式 |
| MCP Server 模式 | Hermes 是消费者不是提供者，不需要暴露工具给外部 |
| Coordinator 模式 | 现有 subagent 已覆盖此场景 |
| 4 层记忆系统 | 双层 + Session Memory 已足够，4 层过度工程 |

---

## 验证方案

每个 Phase 完成后：

```bash
# Phase 1 验证
# 测试：同时读 5 个文件，确认并行执行
hermes chat "读取 src/a.py src/b.py src/c.py src/d.py src/e.py 并总结"
# 预期：日志显示 ThreadPoolExecutor 并行，总时间 < 串行的 60%

# Phase 2 验证
# 测试：长对话触发分级压缩
hermes chat --session test-compact "写 100 行代码然后继续对话 20 轮"
# 预期：日志显示 snip → micro → summarize 逐级触发

# Phase 3 验证
# 测试：大输出自动恢复
hermes chat "详细解释 Linux 内核的进程调度机制，至少 5000 字"
# 预期：输出不截断，日志显示 escalation
```
