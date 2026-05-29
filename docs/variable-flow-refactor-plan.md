# 变量流转系统重构方案

## 一、问题诊断

### 1.1 今日实战踩坑清单

| # | 问题 | 根因 | 影响 |
|---|------|------|------|
| 1 | `{{step_2.rows}}` 在 Excel 容器中不展开 | 容器跳过全局 resolve_config | 所有容器节点的模板变量失效 |
| 2 | Excel create action 不传 data 参数 | 容器内部 resolve 实现遗漏 | create 无法带数据 |
| 3 | `[href]` 在 file:// 模式不提取属性 | scraper 分支没调 extract_attr_from_selector | 本地文件模式功能缺失 |
| 4 | `text()` 在 file:// 模式被当无效选择器 | 没处理 text() 语法 | 本地文件模式功能缺失 |
| 5 | `item.title` 在 Rhai 中返回 `()` | 疑似字段名冲突或 Map key 访问异常 | script 节点数据丢失 |
| 6 | `to_int()` 在 Rhai 中不存在 | Rhai 标准库没有 to_int 全局函数 | 脚本执行失败 |
| 7 | `now()` / `.format()` 在 Rhai 中不存在 | Rhai 标准库没有日期函数 | 脚本执行失败 |
| 8 | Excel write 覆盖 create 的文件 | create 和 write 都创建新 Workbook | 数据丢失 |

### 1.2 根因分析

**变量解析有三层，每层规则不同：**

```
┌─────────────────────────────────────────────────┐
│ Layer 1: 全局 resolve_config (executor.rs)       │
│ - 处理 {{step_xxx.field}} 模板替换               │
│ - 容器节点跳过！(避免类型破坏)                    │
├─────────────────────────────────────────────────┤
│ Layer 2: 容器内部 resolve (excel_container.rs等) │
│ - 每个容器自己实现 resolve                       │
│ - 实现不一致，容易遗漏                           │
├─────────────────────────────────────────────────┤
│ Layer 3: Rhai 引擎 (context.rs eval_expr)       │
│ - 处理脚本中的变量访问                           │
│ - 只注入 step_outputs，不注入 params             │
└─────────────────────────────────────────────────┘
```

**核心矛盾**：容器跳过全局 resolve 是为了避免类型破坏（模板变量变成数字/对象导致反序列化失败），但这导致每个容器要自己实现 resolve，实现不一致。

### 1.3 问题本质

不是某个节点的 bug，而是**变量流转系统缺乏统一规范和测试覆盖**。

---

## 二、解决方案

### 2.1 架构层：统一变量解析

**目标**：消除 Layer 2（容器内部 resolve），统一到 Layer 1。

**方案**：两阶段 resolve

```
阶段 1: 类型安全的预处理 (新增)
  - 扫描 config 中所有 {{...}} 模板
  - 替换为占位符: {{step_2.rows}} → __VAR_PLACEHOLDER_0__
  - 记录占位符映射: {0: ("step_2", "rows")}

阶段 2: 正常反序列化
  - config 用占位符反序列化为类型化 struct
  - 不会类型破坏，因为占位符是字符串

阶段 3: 后处理替换 (新增)
  - 遍历 struct 所有字段
  - 把占位符替换为实际值
  - 保持原始类型（数组、数字等）
```

**好处**：
- 容器不需要自己 resolve
- 不会类型破坏
- 所有节点统一行为

### 2.2 测试层：节点集成测试矩阵

**目标**：覆盖所有节点类型之间的变量传递链路。

**测试矩阵**：

```
数据源节点:
  web_scrape ──┐
  http ────────┤
  file_read ───┼──→ 数据处理节点 ──→ 输出节点
  json_parse ──┤      script         excel
  clipboard ───┘      array_*        word
                      convert_*      file_write
                                     notify
```

**每条链路验证**：
1. `{{step_xxx}}` 引用整个输出
2. `{{step_xxx.field}}` 引用嵌套字段
3. `{{params.xxx}}` 引用工作流参数
4. 容器内部 action 间引用
5. 循环体内 `{{__item}}` 引用

**测试用例格式**：
```json
{
  "name": "test_web_scrape_to_excel",
  "description": "web_scrape → script → excel 变量传递",
  "steps": [
    {"id": "step_1", "type": "web_scrape", "config": {"url": "file://test.html", "extract": [{"selector": "li", "fields": {"title": "", "link": "[href]"}}]}},
    {"id": "step_2", "type": "script", "config": {"script": "let items = step_1.items; #{rows: items, count: items.len}"}},
    {"id": "step_3", "type": "excel", "config": {"file_path": "/tmp/test.xlsx", "actions": [{"id": "a1", "type": "write", "config": {"value": "{{step_2.rows}}"}}]}}
  ],
  "assertions": [
    {"step": "step_3", "field": "rows_written", "expected": 10},
    {"file": "/tmp/test.xlsx", "row_count": 11, "col_count": 2}
  ]
}
```

### 2.3 调试层：变量追踪可视化

**目标**：运行时显示每步的输入输出，方便定位问题。

**实现**：在 executor 中添加 `--trace` 模式

```
[TRACE] step_1 (web_scrape) 输出:
  items[0] = {title: "比亚迪...", link: "https://..."}
  items[1] = {title: "华为...", link: "https://..."}
  total_items = 12

[TRACE] step_2 (script) 输入:
  step_1.items = Array(12)
  params.top_n = "10"
[TRACE] step_2 (script) 输出:
  rows = Array(10) [["1","比亚迪...","https://..."], ...]
  count = 10

[TRACE] step_3 (excel) 输入:
  step_2.rows = Array(10) (类型: Array)
  resolve_config 处理: {{step_2.rows}} → Array(10)
[TRACE] step_3 (excel) 输出:
  rows_written = 10
```

### 2.4 文档层：节点能力契约

**目标**：每个节点声明自己的输入输出格式和变量引用方式。

**格式**：
```yaml
# nodes/web_scrape.contract.yaml
name: web_scrape
inputs: []
outputs:
  items:
    type: array
    description: "提取的数据数组"
    schema:
      type: object
      properties:
        title: {type: string}
        link: {type: string}
  total_items:
    type: integer
config:
  extract:
    fields:
      description: "字段提取规则"
      syntax:
        - "空字符串 → 元素文本内容"
        - "[attr] → 提取属性值"
        - "text() → 元素文本内容"
        - "CSS选择器 → 子元素文本"
```

---

## 三、实施计划

### Phase 1: 紧急修复（今天）
- [x] 修复 web_scrape file:// 模式 `[href]` 属性提取
- [x] 修复 web_scrape file:// 模式 `text()` 语法
- [ ] 修复 Excel create action 传 data 参数
- [ ] 修复 Rhai 脚本 `item.title` 返回 `()` 问题
- [ ] 验证 ithome-news-excel 工作流完整运行

### Phase 2: 集成测试（本周）
- [ ] 编写测试框架（test harness）
- [ ] 覆盖 10 条核心链路
- [ ] CI 集成：每次 commit 跑测试矩阵

### Phase 3: 统一 resolve（下周）
- [ ] 实现两阶段 resolve（占位符方案）
- [ ] 移除所有容器内部的 resolve 实现
- [ ] 验证所有容器节点正常工作

### Phase 4: 调试工具（下周）
- [ ] 实现 `--trace` 模式
- [ ] 添加变量类型检查（运行时报错而不是静默失败）

### Phase 5: 文档契约（第三周）
- [ ] 为 60+ 节点编写 contract.yaml
- [ ] 生成节点能力文档
- [ ] 前端显示节点输入输出提示

---

## 四、验收标准

### 4.1 功能验收
- [ ] ithome-news-excel 工作流完整运行，Excel 包含序号+标题+链接
- [ ] 10 条核心链路的集成测试全部通过
- [ ] 新增节点必须有 contract.yaml

### 4.2 质量验收
- [ ] 变量引用错误有明确报错（不是静默返回 Null）
- [ ] `--trace` 模式能显示每步的输入输出
- [ ] CI 绿灯

### 4.3 架构验收
- [ ] 容器节点不需要自己实现 resolve
- [ ] 新增节点只需声明 contract，不需要关心变量解析
- [ ] 变量解析逻辑集中在一处（executor.rs）

---

## 五、风险评估

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 两阶段 resolve 引入新 bug | 中 | 高 | 先写测试再重构，逐步替换 |
| 现有工作流不兼容 | 低 | 高 | 保持向后兼容，渐进迁移 |
| 性能下降 | 低 | 低 | 占位符替换是 O(n)，可忽略 |

---

## 六、总结

**核心思路**：不是修 bug，而是建体系。

1. **统一入口**：消除容器内部 resolve，统一到 executor 层
2. **测试覆盖**：节点集成测试矩阵，CI 自动验证
3. **调试可视化**：`--trace` 模式，运行时看变量流转
4. **文档契约**：每个节点声明输入输出，消除歧义

**预期效果**：新增节点不再踩变量引用的坑，已有节点的问题在 CI 中暴露。
