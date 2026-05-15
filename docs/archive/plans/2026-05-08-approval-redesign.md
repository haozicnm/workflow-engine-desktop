# 人工审批系统重构 Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** 将审批系统从"文件写入+重跑"改为"暂停/恢复+推荐选项+全局审批队列"

**Architecture:**
- 后端：全局 ApprovalStore（Arc + tokio mpsc channel），scheduler 暂停等待决策信号
- 前端：全局 ApprovalCenter 组件，支持自定义选项/推荐/超时倒计时
- 去掉文件存储，统一用 SQLite + 内存 channel

**Tech Stack:** Rust (tokio, serde_json, rusqlite), Vue 3 + TypeScript + shadcn/ui

---

## Phase 1: 后端 — ApprovalStore + 新数据结构

### Task 1: 扩展 approvals SQLite 表

**Objective:** 支持存储自定义选项、推荐项、超时配置

**Files:**
- Modify: `src-tauri/src/data/db.rs:113-122`

**Step 1:** 修改建表 SQL，新增字段：

```sql
CREATE TABLE IF NOT EXISTS approvals (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    step_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    title TEXT,
    message TEXT,
    item TEXT,
    options TEXT,
    recommended TEXT,
    timeout_secs INTEGER DEFAULT 300,
    timeout_action TEXT DEFAULT 'reject',
    created_at TEXT NOT NULL,
    decided_at TEXT,
    decision TEXT,
    comment TEXT
);
```

**Step 2:** 给 db.rs 添加 approval CRUD 方法：

```rust
pub fn insert_approval(&self, approval: &ApprovalRecord) -> Result<()>;
pub fn update_approval_decision(&self, id: &str, decision: &str, comment: Option<&str>) -> Result<()>;
pub fn get_pending_approvals(&self) -> Result<Vec<ApprovalRecord>>;
pub fn get_approval(&self, id: &str) -> Result<Option<ApprovalRecord>>;
pub fn delete_approval(&self, id: &str) -> Result<()>;
```

**Step 3:** 运行 `cargo test` 验证不影响现有测试

---

### Task 2: 创建全局 ApprovalStore

**Objective:** 内存中存储等待中的审批请求，用 channel 实现暂停/恢复信号

**Files:**
- Create: `src-tauri/src/engine/approval_store.rs`
- Modify: `src-tauri/src/engine/mod.rs` (添加 mod approval_store)

**Step 1:** 创建 approval_store.rs：

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalEntry {
    pub id: String,
    pub run_id: String,
    pub step_id: String,
    pub title: String,
    pub message: String,
    pub item: Option<serde_json::Value>,
    pub options: Vec<String>,
    pub recommended: String,
    pub timeout_secs: u64,
    pub timeout_action: String,
    pub created_at: String,
}

#[derive(Debug)]
pub struct ApprovalDecision {
    pub approved: bool,
    pub option: String,
    pub comment: Option<String>,
}

pub struct ApprovalStore {
    entries: RwLock<HashMap<String, ApprovalEntry>>,
    channels: RwLock<HashMap<String, mpsc::Sender<ApprovalDecision>>>,
}

impl ApprovalStore {
    pub fn new() -> Self { ... }

    /// 注册新审批请求，返回接收端（调用方 await 等待决策）
    pub async fn register(
        &self,
        entry: ApprovalEntry,
    ) -> mpsc::Receiver<ApprovalDecision> {
        let (tx, rx) = mpsc::channel(1);
        self.channels.write().await.insert(entry.id.clone(), tx);
        self.entries.write().await.insert(entry.id.clone(), entry);
        rx
    }

    /// 提交决策
    pub async fn decide(
        &self,
        id: &str,
        decision: ApprovalDecision,
    ) -> Result<(), String> {
        let tx = self.channels.write().await.remove(id)
            .ok_or("审批请求不存在")?;
        self.entries.write().await.remove(id);
        tx.send(decision).await.map_err(|_| "发送决策失败".into())
    }

    /// 获取所有待审批
    pub async fn pending(&self) -> Vec<ApprovalEntry> {
        self.entries.read().await.values().cloned().collect()
    }

    /// 超时清理
    pub async fn timeout(&self, id: &str) -> Option<ApprovalDecision> {
        let entry = self.entries.read().await.get(id)?.clone();
        // 根据 timeout_action 生成自动决策
        ...
    }
}
```

**Step 2:** 在 main.rs 注册为 Tauri managed state：

```rust
let approval_store = Arc::new(ApprovalStore::new());
app.manage(approval_store.clone());
```

**Step 3:** `cargo build` 验证编译通过

---

### Task 3: 重写 ApprovalNode

**Objective:** 支持自定义选项、推荐项、require_review 跳过审批

**Files:**
- Rewrite: `src-tauri/src/nodes/approval.rs`

**Step 1:** 新的 execute 逻辑：

```rust
async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, executor: &Arc<StepExecutor>) -> Result<Value> {
    let config = &step.config;

    let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("人工审批").to_string();
    let message = resolve_template(config.get("message"), ctx);
    let options = config.get("options").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_else(|| vec!["同意".into(), "拒绝".into()]);
    let recommended = config.get("recommended").and_then(|v| v.as_str()).unwrap_or("同意").to_string();
    let require_review = config.get("require_review").and_then(|v| v.as_bool()).unwrap_or(true);
    let timeout_secs = config.get("timeout").and_then(|v| v.as_u64()).unwrap_or(300);
    let timeout_action = config.get("timeout_action").and_then(|v| v.as_str()).unwrap_or("recommended").to_string();

    // 收集上游数据（只取上一步输出，不取全上下文）
    let item = collect_upstream_item(ctx, config);

    // 不需要人工审核 → 直接用推荐选项
    if !require_review {
        return Ok(json!({
            "decision": recommended,
            "comment": "自动决策（无需审核）",
            "auto": true,
        }));
    }

    // 注册到 ApprovalStore，等待决策
    let store = executor.approval_store(); // 从 executor 获取引用
    let entry = ApprovalEntry { id: format!("approval:{}:{}", run_id, step.id), ... };
    let mut rx = store.register(entry).await;

    // 等待决策或超时
    let decision = tokio::select! {
        Some(d) = rx.recv() => d,
        _ = tokio::time::sleep(Duration::from_secs(timeout_secs)) => {
            // 超时：根据 timeout_action 生成决策
            match timeout_action.as_str() {
                "recommended" => ApprovalDecision { option: recommended.clone(), ... },
                "reject" => ApprovalDecision { option: "拒绝".into(), ... },
                "approve" => ApprovalDecision { option: "同意".into(), ... },
                "fail" => return Err(anyhow!("审批超时")),
                _ => ApprovalDecision { option: timeout_action, ... },
            }
        }
    };

    Ok(json!({
        "decision": decision.option,
        "comment": decision.comment,
        "item": item,
    }))
}
```

**Step 2:** 删除所有文件读写相关代码（approval_path, read_approval, save_approval, record_decision）

**Step 3:** `cargo test` — 更新 approval 相关测试

---

### Task 4: 修改 Scheduler 支持暂停/恢复

**Objective:** scheduler 遇到审批节点不退出，暂停等待信号

**Files:**
- Modify: `src-tauri/src/engine/scheduler.rs`

**Step 1:** 去掉 scheduler L159-162 的 `emit_approval_required`（废代码）

**Step 2:** 修改 execute 阶段，approval 输出 `awaiting_approval` 时暂停：

```rust
// 在执行结果 match 分支中
Ok(output) => {
    // approval 节点暂停等待（输出中已包含决策结果）
    // 因为 ApprovalNode.execute 内部 await 了决策，返回的已经是决策结果
    // 所以 scheduler 无需特殊处理，直接走正常流程
    ctx.set_output(&current_id, output.clone());
    ...
}
```

**重点：** 因为 `ApprovalNode.execute` 内部用 `tokio::select!` 等待决策，所以返回值已经是最终决策。scheduler 不需要额外处理暂停/恢复——暂停发生在 node executor 内部。

**Step 3:** 删除 `determine_next_step` 中 approval 的特殊分支（L354-361），因为不再有 `awaiting_approval` 状态

**Step 4:** `cargo test` — 验证所有 scheduler 测试通过

---

### Task 5: 更新 approval_response 命令

**Objective:** 通过 ApprovalStore 提交决策，不再写文件

**Files:**
- Modify: `src-tauri/src/commands/run.rs:252-260`

**Step 1:** 重写 approval_response：

```rust
#[tauri::command]
pub async fn approval_response(
    app: tauri::AppHandle,
    approval_id: String,
    approved: bool,
    comment: Option<String>,
    option: Option<String>,  // 新增：用户选择的选项
) -> Result<(), String> {
    let store = app.state::<Arc<ApprovalStore>>();
    let decision = ApprovalDecision {
        approved,
        option: option.unwrap_or_else(|| if approved { "同意".into() } else { "拒绝".into() }),
        comment,
    };
    store.decide(&approval_id, decision).await
}
```

**Step 2:** 更新前端 safeInvoke 的参数，传入 option

---

## Phase 2: 前端 — ApprovalCenter

### Task 6: 前端类型定义更新

**Objective:** 更新 approval 相关类型

**Files:**
- Modify: `src/types/types.ts`
- Modify: `src/types/node-registry.ts:66-75`

**Step 1:** node-registry.ts 更新 approval 参数定义：

```typescript
{ type: 'approval', label: '人工审批', icon: '✋', color: '#f778ba',
  description: '暂停流程等待人工审核：支持自定义选项和推荐',
  outputHint: '{ decision: "选项名", comment, item, auto? }',
  params: [
    { key: 'title', label: '审批标题', type: 'text', placeholder: '请确认订单信息' },
    { key: 'message', label: '审批内容', type: 'textarea', placeholder: '订单号：{{step_1.action_1_1.订单号}}' },
    { key: 'options', label: '审批选项', type: 'text', placeholder: '同意,拒绝,需要更多信息（逗号分隔）', default: '同意,拒绝' },
    { key: 'recommended', label: '推荐选项', type: 'text', placeholder: '同意', default: '同意' },
    { key: 'require_review', label: '需要人工审核', type: 'select', options: [
      { label: '是', value: 'true' }, { label: '否（自动决策）', value: 'false' },
    ], default: 'true' },
    { key: 'timeout', label: '超时(秒)', type: 'number', default: 300 },
    { key: 'timeout_action', label: '超时策略', type: 'select', options: [
      { label: '执行推荐选项', value: 'recommended' },
      { label: '自动拒绝', value: 'reject' },
      { label: '自动通过', value: 'approve' },
      { label: '标记失败', value: 'fail' },
    ], default: 'recommended' },
  ]
},
```

---

### Task 7: 创建 ApprovalCenter 全局组件

**Objective:** 全局审批队列，显示所有待审批，支持推荐高亮

**Files:**
- Create: `src/components/ApprovalCenter.vue`
- Modify: `src/App.vue` (挂载 ApprovalCenter)

**Step 1:** ApprovalCenter.vue 核心结构：

```vue
<script setup lang="ts">
// 监听 approval-required 事件，收集待审批列表
// 支持推荐选项高亮、批量操作、超时倒计时
</script>

<template>
  <!-- 侧边栏 Badge 计数 -->
  <!-- 点击展开审批面板 -->
  <!-- 审批卡片列表 -->
  <!-- 每张卡片：标题 + 消息 + 选项按钮（推荐高亮）+ 超时倒计时 -->
</template>
```

**Step 2:** 审计卡片 UI：

```
┌─────────────────────────────────┐
│  ✋ 价格审核                     │
│  采购订单 #2048，金额 ¥12,800    │
│                                  │
│  ⭐ 同意  ← 推荐（高亮蓝框）     │
│     拒绝                         │
│     需要更多信息                  │
│                                  │
│  💬 审批意见（可选）              │
│  ⏰ 超时：04:32 后自动执行推荐   │
└─────────────────────────────────┘
```

**Step 3:** 挂到 App.vue，确保所有页面共享

---

### Task 8: 删除旧 ApprovalDialog，清理引用

**Objective:** 移除旧的文件+重跑模式

**Files:**
- Delete: `src/components/ApprovalDialog.vue`
- Modify: `src/pages/Editor.vue` (删除 ApprovalDialog 引用和 onApprovalDecided)
- Modify: `src/pages/Dashboard.vue` (删除 ApprovalDialog 引用)

**Step 1:** 删除文件

**Step 2:** Editor.vue 删除：
- `import ApprovalDialog`
- `<ApprovalDialog @approval-decided="onApprovalDecided" />`
- `onApprovalDecided` 函数

**Step 3:** Dashboard.vue 删除：
- `import ApprovalDialog`
- `<ApprovalDialog @approval-decided="() => {}" />`

**Step 4:** `vite build` 验证前端编译通过

---

### Task 9: 端到端验证

**Objective:** 创建测试工作流，验证完整审批流程

**Step 1:** 创建工作流：浏览器步骤 → 审批节点 → 日志步骤
**Step 2:** 运行，验证暂停 → 弹出审批面板 → 点击同意 → 自动继续
**Step 3:** 测试超时：设置 10 秒超时，不操作，验证自动执行推荐选项
**Step 4:** 测试 require_review: false，验证跳过审批直接继续
**Step 5:** `cargo test` + `vite build` 全部通过

---

## 文件改动总览

| 文件 | 操作 | 说明 |
|------|------|------|
| `src-tauri/src/data/db.rs` | 修改 | 扩展 approvals 表 + CRUD |
| `src-tauri/src/engine/approval_store.rs` | 新建 | 全局审批 store |
| `src-tauri/src/engine/mod.rs` | 修改 | 添加 mod |
| `src-tauri/src/nodes/approval.rs` | 重写 | channel 等待模式 |
| `src-tauri/src/engine/scheduler.rs` | 修改 | 删除 approval 特殊分支 |
| `src-tauri/src/commands/run.rs` | 修改 | approval_response 走 store |
| `src-tauri/src/main.rs` | 修改 | 注册 ApprovalStore |
| `src/types/node-registry.ts` | 修改 | 新参数定义 |
| `src/components/ApprovalCenter.vue` | 新建 | 全局审批面板 |
| `src/components/ApprovalDialog.vue` | 删除 | 旧弹窗 |
| `src/pages/Editor.vue` | 修改 | 删旧引用 |
| `src/pages/Dashboard.vue` | 修改 | 删旧引用 |
| `src/App.vue` | 修改 | 挂 ApprovalCenter |
