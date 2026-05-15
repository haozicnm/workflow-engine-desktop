# 执行逻辑梳理方案

## 问题清单

| # | 问题 | 严重度 | 影响 |
|---|------|--------|------|
| 1 | cursor/loop body 存储链路不清晰 | 🔴 高 | 可能导致迭代节点 body 步骤丢失 |
| 2 | thenSteps/elseSteps 死字段残留 | 🟡 中 | 旧数据加载可能带入无效数据 |
| 3 | 变量引用无校验 | 🟡 中 | 手写错误引用静默 undefined |
| 4 | 简单/容器步骤输出格式无提示 | 🟢 低 | 用户不知道怎么引用 |

---

## 方案

### P0: cursor/loop body 存储链路验证

**目标**：确认前端 actions → parser → 后端 body_steps 的完整链路

**步骤**：
1. 读 `parser.rs` 的 `convert_step()`，确认 cursor/loop 类型是否走特殊分支
2. 如果 parser 没处理 → 加分支：cursor/loop 的 `actions` 映射为 `body_steps`
3. 写一个端到端测试：构造含 cursor 的 workflow JSON → parser → 断言 `body_steps` 非空
4. 确认后端 cursor.rs 的 `parse_body_steps()` 能正确读取

**验收**：cursor 节点添加 body 步骤后，保存→加载→执行，body 步骤正常执行

### P1: thenSteps/elseSteps 清理

**目标**：移除逻辑容器的嵌套步骤残留

**步骤**：
1. `types.ts` 的 Step 接口删除 `thenSteps?: Step[]` 和 `elseSteps?: Step[]`
2. `normalizeSteps()` 增加清理逻辑：删除 step 上的 thenSteps/elseSteps 字段
3. `parser.rs` 确认不再递归处理 then_steps/else_steps（如果还在处理就删掉）
4. grep 所有引用这两个字段的代码，确认无遗漏

**验收**：加载含 thenSteps 的旧数据不会报错，字段被静默剥离

### P2: 变量引用校验

**目标**：保存时检查引用有效性，给出警告

**步骤**：
1. 在 `workflowStore.ts` 的 `saveWorkflow()` 前加校验函数 `validateRefs()`
2. 遍历所有步骤的所有参数，正则提取 `{{...}}` 引用
3. 检查引用的 stepId 是否存在于 workflow.steps 中
4. 对于 actionId 级引用，检查对应 step 的 actions 是否包含该 actionId
5. 无效引用收集为 warning 列表，toast 提示但不阻止保存
6. （可选）在变量选择器的引用标签上加红色标记表示无效

**伪代码**：
```typescript
function validateRefs(workflow: Workflow): string[] {
  const warnings: string[] = []
  const stepIds = new Set(workflow.steps.map(s => s.id))
  const actionIds = new Map<string, Set<string>>()
  for (const step of workflow.steps) {
    actionIds.set(step.id, new Set((step.actions || []).map(a => a.id)))
  }
  
  for (const step of workflow.steps) {
    const params = step.type === 'logic' 
      ? JSON.stringify(step.conditionGroup) 
      : JSON.stringify(step.config)
    // 也检查 actions 的 params
    for (const action of (step.actions || [])) {
      const text = JSON.stringify(action.params)
      for (const m of text.matchAll(/\{\{([^}]+)\}\}/g)) {
        const ref = m[1]
        const [refStep, refAction] = ref.split('.')
        if (!stepIds.has(refStep)) {
          warnings.push(`${step.label}: 引用了不存在的步骤 ${refStep}`)
        } else if (refAction && !actionIds.get(refStep)?.has(refAction)) {
          warnings.push(`${step.label}: 引用了不存在的动作 ${ref}`)
        }
      }
    }
  }
  return warnings
}
```

**验收**：引用不存在的步骤/动作时，保存时 toast 显示警告列表

### P3: 输出格式提示

**目标**：在变量选择器中提示每个步骤的输出格式

**步骤**：
1. 在 `node-registry.ts` 的 `ContainerDef` 中加 `outputHint?: string` 字段
2. 各容器定义填写 outputHint：
   - browser/excel/word: `{ actionId: value, ... }`
   - logic: `{ branch: "true"/"false", value, result }`
   - http: `{ status, body, headers }`
   - script: 脚本返回值
   - cursor: `{ done, item, index, total }`
   - approval: `{ approved, message }`
3. 在变量选择器的步骤分组 header 上显示 outputHint（灰色小字）

**验收**：变量选择器中每个步骤组下方显示输出格式说明

---

## 执行顺序

```
P0 (验证链路) → P1 (清理死字段) → P2 (引用校验) → P3 (输出提示)
```

P0 优先，因为如果 cursor/loop 的 body 链路断了，迭代节点根本不能用。
P1 是纯清理，风险低。P2/P3 是体验优化。

---

## 不动的部分

以下已验证正常，不需要改：
- 步骤/动作 ID 生成 ✅
- 变量引用格式 `{{stepId.actionId}}` ✅
- 容器输出 key 用 action.id ✅
- resolve_var 三步查找 ✅
- 条件执行 serde alias + scheduler ✅
- 审批/游标暂停续跑机制 ✅
