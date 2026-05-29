# Phase 2: 节点集成测试框架 — 详细方案

## 一、现状分析

### 1.1 已有测试（7个文件，约100+测试）

| 文件 | 覆盖范围 | 质量 |
|------|----------|------|
| `integration_test.rs` | 16种节点执行 + 2条链路 | ★★★★ |
| `context_tests.rs` | resolve_var/resolve_config/eval_expr | ★★★★★ |
| `scheduler_tests.rs` | 路由逻辑（条件/游标/审批/容器） | ★★★★ |
| `parser_tests.rs` | JSON→Step 转换 | ★★★★ |
| `library_template_tests.rs` | 5个模板加载验证 | ★★★ |
| `template_tests.rs` | 模板结构验证 | ★★ |
| `template_exec_tests.rs` | 模板端到端执行 | ★★ |

### 1.2 关键缺口

**缺口 A：变量传递链路（最严重）**

已测：
- shell → notify（字符串传递）
- script → condition → approval（多步传递）
- params.xxx 注入（Windows-only）

未测：
- web_scrape → script → excel（数组传递）← 今天踩坑
- http → json_parse → script（嵌套对象传递）
- loop collect → 后续步骤（聚合结果传递）
- 并行分支输出聚合
- 容器内部 action 间变量引用

**缺口 B：30+ 节点零测试**

文件操作：file_read/write/list/delete/exists/append/mkdir/copy/move/glob/checksum
数据操作：data_set/get/length/default/merge
数组操作：array_filter/sort/dedup/paginate/map/join/reduce
正则：regex_extract/replace/match
转换：convert_to_text/number/json/csv/html/base64
其他：web_scrape, clipboard, text_template

**缺口 C：错误处理路径**

- retry 机制（Step.retry 字段存在，从未测试）
- step timeout（Step.timeout 字段存在，从未测试）
- 循环中的错误传播
- 并行分支的错误隔离

---

## 二、测试框架设计

### 2.1 核心理念

**不测节点内部逻辑（那是单元测试），测变量在节点间的流转。**

一个测试用例 = 一条变量流转链路 + 断言。

### 2.2 测试用例格式

```rust
// tests/variable_flow_tests.rs

#[tokio::test]
async fn test_web_scrape_to_script_to_excel() {
    let chain = TestChain::new()
        .step("step_1", "web_scrape", json!({
            "url": "file://tests/fixtures/ithome-sample.html",
            "extract": [{"selector": "li a", "fields": {"title": "", "link": "[href]"}}]
        }))
        .step("step_2", "script", json!({
            "script": "let items = step_1.items; #{rows: items, count: items.len}"
        }))
        .step("step_3", "excel", json!({
            "file_path": "/tmp/test_chain.xlsx",
            "actions": [{"id": "a1", "type": "write", "config": {"value": "{{step_2.rows}}"}}]
        }));

    let result = chain.run().await;

    // 断言变量传递
    assert_step_output(&result, "step_1", |out| {
        assert_eq!(out["total_items"].as_i64().unwrap(), 12);
        assert!(out["items"][0]["title"].as_str().unwrap().len() > 0);
        assert!(out["items"][0]["link"].as_str().unwrap().starts_with("https://"));
    });

    assert_step_output(&result, "step_2", |out| {
        assert_eq!(out["count"].as_i64().unwrap(), 12);
        assert_eq!(out["rows"].as_array().unwrap().len(), 12);
        assert_eq!(out["rows"][0][0].as_str().unwrap(), "中国首款"); // 标题
    });

    assert_step_output(&result, "step_3", |out| {
        assert_eq!(out["rows_written"].as_i64().unwrap(), 12);
    });

    // 断言文件存在且内容正确
    assert_excel_row_count("/tmp/test_chain.xlsx", 12);
}
```

### 2.3 TestChain 辅助结构

```rust
struct TestChain {
    steps: Vec<(String, String, serde_json::Value)>, // (id, type, config)
    variables: HashMap<String, serde_json::Value>,
}

impl TestChain {
    fn new() -> Self { ... }

    fn step(mut self, id: &str, step_type: &str, config: serde_json::Value) -> Self {
        self.steps.push((id.to_string(), step_type.to_string(), config));
        self
    }

    fn var(mut self, key: &str, value: serde_json::Value) -> Self {
        self.variables.insert(key.to_string(), value);
        self
    }

    async fn run(self) -> TestResult {
        let executor = StepExecutor::new(
            Arc::new(ApprovalStore::new()),
            Arc::new(Database::new(":memory:").unwrap()),
        );
        let workflow = Workflow {
            variables: Some(self.variables),
            ..Default::default()
        };
        let mut ctx = ExecutionContext::new("test-run", &workflow);

        let mut outputs = HashMap::new();
        for (id, step_type, config) in &self.steps {
            let step = make_step(id, "test", step_type, config);
            let result = executor.execute(&step, &mut ctx).await;
            ctx.set_output(id, result.clone().unwrap_or(serde_json::Value::Null));
            outputs.insert(id.clone(), result);
        }

        TestResult { outputs, ctx }
    }
}

struct TestResult {
    outputs: HashMap<String, Result<serde_json::Value>>,
    ctx: ExecutionContext,
}

fn assert_step_output(result: &TestResult, step_id: &str, check: impl Fn(&serde_json::Value)) {
    let output = result.outputs.get(step_id)
        .expect(&format!("step {} not found", step_id))
        .as_ref()
        .expect(&format!("step {} failed", step_id));
    check(output);
}
```

### 2.4 测试夹具（Fixtures）

```
tests/
  fixtures/
    html/
      ithome-sample.html          # IT之家热榜 HTML 片段
      simple-list.html            # 简单列表页
      table.html                  # 表格页
    json/
      api-response.json           # 模拟 API 响应
      nested-data.json            # 嵌套数据结构
    excel/
      sample.xlsx                 # 预制 Excel 文件
    files/
      test.txt                    # 文本文件
      test.csv                    # CSV 文件
```

---

## 三、测试用例清单

### 3.1 变量传递链路（10条核心链路）

| # | 链路 | 验证点 | 优先级 |
|---|------|--------|--------|
| 1 | web_scrape → script → excel | 数组传递、字段提取、Excel写入 | P0 |
| 2 | http → json_parse → script | 嵌套对象、$.data.items 语法 | P0 |
| 3 | script → loop → collect → script | 循环变量注入、聚合结果 | P0 |
| 4 | script → condition → notify (true分支) | 条件判断、分支路由 | P0 |
| 5 | script → condition → notify (false分支) | 条件判断、另一分支 | P0 |
| 6 | file_read → script → file_write | 文件读写链路 | P1 |
| 7 | script → array_filter → array_sort → script | 数组操作链 | P1 |
| 8 | script → parallel(2 branches) → script | 并行分支聚合 | P1 |
| 9 | script → while → script | 循环条件变量 | P1 |
| 10 | params注入 → script → excel | params.xxx 引用 | P1 |

### 3.2 节点基础测试（30个未覆盖节点）

**P0（核心数据流节点）**：
- web_scrape: file:// 模式、[href] 属性、text() 语法
- data_set / data_get: 跨步骤变量存取
- array_filter / array_sort / array_paginate: 数组操作
- convert_to_json / convert_to_csv: 格式转换

**P1（文件操作节点）**：
- file_read / file_write / file_append
- file_list / file_glob
- file_exists / file_mkdir

**P2（其他节点）**：
- regex_extract / regex_replace
- clipboard_read / clipboard_write
- text_template

### 3.3 错误处理测试（5条）

| # | 场景 | 预期行为 |
|---|------|----------|
| 1 | script 语法错误 | 返回错误，不崩溃 |
| 2 | step.retry=2 + 第一次失败 | 自动重试2次 |
| 3 | step.timeout=1000 + 超时 | 超时报错 |
| 4 | loop 空数组 | 0次迭代，正常完成 |
| 5 | excel 容器 write 不存在的文件 | 自动创建或报错 |

---

## 四、实施步骤

### Step 1: 创建测试基础设施（1小时）
- 创建 `tests/variable_flow_tests.rs`
- 实现 `TestChain` 和 `TestResult` 辅助结构
- 创建 `tests/fixtures/` 目录和夹具文件

### Step 2: 实现 P0 链路测试（2小时）
- 链路 1: web_scrape → script → excel（修复 today's bug）
- 链路 2: http → json_parse → script
- 链路 3: script → loop → collect → script
- 链路 4-5: condition 分支

### Step 3: 实现 P0 节点测试（2小时）
- web_scrape file:// 模式测试
- data_set / data_get 测试
- array_* 系列测试

### Step 4: CI 集成（30分钟）
- 添加到 GitHub Actions
- 确保 `cargo test` 包含新测试

### Step 5: 实现 P1 链路和节点测试（3小时）
- 链路 6-10
- 文件操作节点
- 数组操作节点

### Step 6: 错误处理测试（1小时）
- retry/timeout/空数组/语法错误

---

## 五、预期产出

### 5.1 数量指标
- 新增测试文件：1 个（`variable_flow_tests.rs`）
- 新增测试用例：约 40 个
- 测试夹具文件：约 10 个
- 代码行数：约 1500 行

### 5.2 质量指标
- 节点覆盖率：从 50% → 85%（30个未测节点覆盖 25 个）
- 变量传递链路覆盖：从 2 条 → 12 条
- 每次 commit 自动验证变量流转正确性

### 5.3 验收标准
- [ ] 链路 1（web_scrape → script → excel）测试通过
- [ ] 链路 3（script → loop → collect → script）测试通过
- [ ] 所有 P0 节点有独立测试
- [ ] `cargo test` 在 CI 中全绿
- [ ] ithome-news-excel 工作流实际运行成功

---

## 六、与后续阶段的关系

Phase 2（本方案）建立测试基础后：

- **Phase 3（统一 resolve）**：重构 resolve 逻辑时，跑测试矩阵确保不破坏现有行为
- **Phase 4（调试工具）**：`--trace` 模式的输出可以用测试验证
- **Phase 5（文档契约）**：contract.yaml 可以从测试用例自动生成

测试是基础设施，先建好再重构。
