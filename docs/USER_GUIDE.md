# Workflow Engine Desktop — 使用说明

> 版本 6.6.0 | 更新时间：2026-05-15

---

## 目录

1. [安装与启动](#1-安装与启动)
2. [界面概览](#2-界面概览)
3. [创建工作流](#3-创建工作流)
4. [节点类型详解](#4-节点类型详解)
5. [变量与表达式](#5-变量与表达式)
6. [执行与调试](#6-执行与调试)
7. [模板快速开始](#7-模板快速开始)
8. [定时调度](#8-定时调度)
9. [设置](#9-设置)
10. [常见问题](#10-常见问题)

---

## 1. 安装与启动

### 方式一：安装版（推荐）

1. 下载 `Workflow Engine_1.1.0-beta_x64-setup.exe`
2. 双击运行安装程序，按提示完成安装
3. 从开始菜单或桌面快捷方式启动

### 方式二：绿色版（免安装）

1. 解压 `Workflow Engine 1.1.0-beta Portable.zip`
2. 运行 `启动 Workflow Engine.bat` 或直接双击 `workflow-engine.exe`
3. 数据存储在同目录下的 `data/` 文件夹，随用随删

### 系统要求

- **操作系统**：Windows 10/11（64位）
- **磁盘空间**：约 100 MB（不含内置 Python）
- **Python**（可选）：如需使用浏览器节点，需要 Python 3.8+
  - 绿色版已内置 Python，无需额外安装
  - 安装版需自行安装 Python 并在设置中配置路径

---

## 2. 界面概览

应用包含 4 个主要页面：

### 🏠 首页（工作流列表）

- 查看所有已保存的工作流
- 搜索工作流名称
- 快速操作：**编辑**、**克隆**、**导出**、**删除**、**立即执行**
- 从模板创建工作流
- 导入 YAML 文件

### 📝 编辑器

- **左侧**：节点面板 + 变量面板
- **中间**：画布（拖拽编辑步骤）
- **右侧**：YAML 编辑面板（可隐藏）
- **顶部**：工具栏（保存/执行/验证/历史）

### 📊 运行历史

- 查看所有工作流的执行记录
- 按工作流筛选
- 查看每步执行详情（状态/耗时/输出/错误）

### ⚙️ 设置

- 主题/语言
- Python 路径配置
- 浏览器选择
- 开机自启

---

## 3. 创建工作流

### 3.1 基本概念

工作流由**步骤（Step）**按顺序执行。每个步骤有：

| 属性 | 说明 |
|------|------|
| ID | 唯一标识符，用于步骤间引用（如 `step_http1`） |
| 名称 | 显示名称 |
| 类型 | 节点类型（http、data、script 等） |
| 配置 | 该类型的具体参数 |

### 3.2 创建步骤

**方式一：从节点面板添加**

1. 点击左侧节点面板中的节点类型按钮
2. 新步骤自动添加到画布末尾
3. 点击步骤卡片上的 ⚙️ 图标编辑配置

**方式二：从模板创建**

1. 点击首页的 📋 模板按钮
2. 选择一个模板（如「HTTP GET」「网页抓取→Excel」）
3. 自动生成包含多个步骤的工作流

**方式三：拖拽排序**

- 在画布中拖拽步骤卡片调整执行顺序

### 3.3 保存与验证

- **Ctrl+S** 快速保存
- 点击 **✔ 验证** 检查 YAML 格式是否正确
- 保存后才能执行

### 3.4 导入导出

- **导出**：首页点击工作流的 📤 按钮，下载 YAML 文件
- **导入**：首页点击 📥 导入按钮，选择 YAML 文件
- **克隆**：首页点击 📋 按钮，复制一份相同工作流

---

## 4. 节点类型详解

### 4.1 🌐 HTTP 请求

发送 HTTP 请求，获取 API 数据或调用 Web 服务。

| 参数 | 说明 | 示例 |
|------|------|------|
| 方法 | GET / POST / PUT / DELETE / PATCH | GET |
| URL | 请求地址 | `https://api.example.com/data` |
| Headers | 请求头（JSON） | `{"Authorization": "Bearer xxx"}` |
| Body | 请求体（POST/PUT 时使用） | `{"name": "test"}` |
| 超时 | 请求超时时间（秒） | 30 |

**示例：获取天气数据**

```yaml
- id: get_weather
  name: 获取天气
  type: http
  config:
    action: GET
    url: "https://api.openweathermap.org/data/2.5/weather?q=Beijing&appid=YOUR_KEY"
```

**输出：** `{ "status": 200, "body": { "main": { "temp": 293 }, ... }, "headers": {...} }`

---

### 4.2 📊 数据处理

对数据进行赋值、过滤、转换等操作。

| 操作 | 说明 | 示例 |
|------|------|------|
| set | 设置变量值 | `key: "result", value: 42` |
| get | 获取嵌套字段 | `path: "response.data.items"` |
| length | 计算数组/字符串长度 | `source: "{{step_api.items}}"` |
| merge | 合并多个对象 | `sources: ["{{step_a}}", "{{step_b}}"]` |
| transform | 映射数组 | `source: [...], mapper: "item.name"` |
| default | 设置默认值 | `key: "count", value: "{{step_x.count \| 0}}"` |

**示例：提取 API 返回的列表**

```yaml
- id: extract_items
  name: 提取列表
  type: data
  config:
    action: get
    path: "step_api.body.data.items"
```

---

### 4.3 📝 脚本

执行 Rhai 脚本进行复杂数据处理。

| 参数 | 说明 |
|------|------|
| script | Rhai 脚本代码 |

**支持的 Rhai 功能：**
- 变量、数组、对象
- 数学运算
- 字符串操作
- 循环和条件
- 内置函数（`len()`、`type_of()`、`parse_int()` 等）

**示例：计算总价**

```yaml
- id: calc_total
  name: 计算总价
  type: script
  config:
    script: |
      let items = step_extract.items;
      let total = 0;
      for item in items {
          total += item.price;
      }
      #{ "total": total, "count": items.len() }
```

**输出：** `{ "total": 1500, "count": 5 }`

---

### 4.4 ❓ 条件

根据条件判断执行不同分支。

| 参数 | 说明 |
|------|------|
| mode | `compare`（声明式）或 `expression`（表达式） |
| op | 比较操作符（见下表） |
| left | 左值 |
| right | 右值 |
| true_next | 条件为真时跳转的步骤 ID |
| false_next | 条件为假时跳转的步骤 ID |

**支持的比较操作符：**

| 操作符 | 说明 |
|--------|------|
| `eq` / `ne` | 等于 / 不等于 |
| `gt` / `gte` | 大于 / 大于等于 |
| `lt` / `lte` | 小于 / 小于等于 |
| `contains` | 包含 |
| `starts_with` / `ends_with` | 开头 / 结尾匹配 |
| `empty` / `not_empty` | 为空 / 不为空 |
| `in` / `not_in` | 在列表中 / 不在列表中 |

**示例：判断是否有数据**

```yaml
- id: check_data
  name: 有数据吗
  type: condition
  config:
    mode: compare
    left: "{{step_api.body.total}}"
    op: gt
    right: "0"
    true_next: process_data
    false_next: no_data_notify
```

---

### 4.5 🔄 循环

遍历数组，对每个元素执行子步骤。

| 参数 | 说明 |
|------|------|
| items | 要遍历的数组（JSON 或变量引用） |
| body | 循环体（步骤数组） |
| collect | **汇集**：从每轮迭代提取字段，汇集成数组 |
| table | **表格**：自动生成 headers+rows+data，可直接喂给 Excel |

**循环内置变量：**
- `{{__item}}` — 当前元素
- `{{__index}}` — 当前索引（从 0 开始）
- `{{__index1}}` — 当前索引（从 1 开始）
- `{{__item.name}}` — 当前元素的嵌套字段

**示例：逐个调用 API**

```yaml
- id: loop_users
  name: 遍历用户
  type: loop
  config:
    items: "{{step_get_users.body}}"
    body:
      - id: send_email
        name: 发送邮件
        type: http
        config:
          action: POST
          url: "https://api.example.com/send"
          body: {"to": "{{__item.email}}", "subject": "通知"}
```

#### Collect 汇集（免代码提取）

从每轮迭代中按路径提取字段，汇集成数组。省去写 script 节点的麻烦。

```yaml
- id: loop_fetch
  name: 循环获取
  type: loop
  config:
    items: "{{step_urls.items}}"
    collect:
      title: "step_fetch.body.title"    # 从每轮 step_fetch 输出中提取 title
      status: "step_fetch.status"        # 从每轮 step_fetch 输出中提取 status
      item_name: "__item.name"           # 从原始遍历项中提取 name
    body:
      - id: step_fetch
        type: http
        config:
          action: GET
          url: "{{__item.url}}"
```

**输出：**
```json
{
  "count": 5,
  "results": [...],
  "collected": {
    "title": ["页面A", "页面B", ...],
    "status": [200, 200, ...],
    "item_name": ["网站A", "网站B", ...]
  }
}
```

下游节点直接用 `{{step_loop.collected.title}}` 引用整个数组。

#### Table 表格（零代码写 Excel）

自动生成二维数组（表头+数据行），可直接作为 Excel 写入节点的 `data` 参数。

```yaml
- id: loop_scrape
  name: 循环抓取
  type: loop
  config:
    items: "{{step_urls.items}}"
    table:
      - header: "名称"
        field: "__item.name"
      - header: "价格"
        field: "step_fetch.body.price"
      - header: "链接"
        field: "__item.url"
    body:
      - id: step_fetch
        type: http
        config:
          action: GET
          url: "{{__item.url}}"

- id: write_excel
  name: 写入Excel
  type: excel
  config:
    action: write
    path: "./output.xlsx"
    sheet: "数据"
    data: "{{step_scrape.table.data}}"   # 直接使用，无需任何代码！
```

**输出：**
```json
{
  "table": {
    "headers": ["名称", "价格", "链接"],
    "rows": [["商品A", 99, "http://..."], ...],
    "data": [["名称", "价格", "链接"], ["商品A", 99, "http://..."], ...]
  }
}
```

`data` 就是 `headers + rows` 合并后的二维数组，Excel 写入节点直接用。

---

### 4.5b 🔁 While 循环

**与 Loop 的区别：**
- **Loop**（for-each）：遍历已知数组，每项都执行
- **While**：遍历数组但**每轮检查条件**，条件不满足时停止

典型场景：读取 Excel 某列，有数据就继续处理，遇到空行就停止。

| 参数 | 说明 |
|------|------|
| items | 数据源数组（变量引用或步骤输出） |
| condition | 停止条件（JSON 对象） |
| body | 循环体（步骤数组） |
| max_iterations | 最大轮次安全上限（默认 10000） |

**循环内置变量：**
- `{{__current}}` / `{{__item}}` — 当前元素
- `{{__index}}` — 当前索引（从 0 开始）
- `{{__index1}}` — 当前索引（从 1 开始）

**condition 操作符：**

| op | 说明 | 示例 |
|----|------|------|
| `not_empty` | 当前元素不为空就继续 | 遇到空行停止 |
| `empty` | 当前元素为空就继续 | 遇到有数据的行停止 |
| `eq` | 当前元素等于 right 就继续 | `right: 0` |
| `ne` | 当前元素不等于 right 就继续 | `right: ""` |
| `gt` / `gte` | 大于 / 大于等于 | `right: 100` |
| `lt` / `lte` | 小于 / 小于等于 | `right: 50` |

**示例：Excel 逐行填写浏览器表单**

```yaml
# 1. 先读取 A 列全部数据
- id: step_read
  name: 读取A列
  type: excel
  config:
    action: extract_column
    path: "./data/input.xlsx"
    column: A

# 2. While 循环：有数据就处理，无数据就停
- id: step_loop
  name: 逐行处理
  type: while
  config:
    items: "step_read"
    condition:
      op: not_empty        # 遇到空行就停
    max_iterations: 1000
    body:
      - id: step_fill
        name: 填入浏览器
        type: browser
        config:
          action: fill
          selector: "#input"
          value: "{{__current}}"

# 3. 完成通知
- id: step_done
  name: 完成通知
  type: notify
  config:
    notify_type: system
    title: 处理完成
    body: "共处理 {{step_loop.count}} 行"
```

**执行流程：**
1. 读取 A 列 → `["张三", "李四", "王五", "", "", ...]`
2. 第 1 轮：`__current = "张三"`，不为空 → 执行 body
3. 第 2 轮：`__current = "李四"`，不为空 → 执行 body
4. 第 3 轮：`__current = "王五"`，不为空 → 执行 body
5. 第 4 轮：`__current = ""`，为空 → **停止**
6. 输出 `{ count: 3, stopped_at: 3, results: [...] }`

---

### 4.6 🌍 浏览器

通过 Playwright 控制浏览器，支持 23 种操作。

**常用操作：**

| 操作 | 说明 | 必填参数 |
|------|------|----------|
| navigate | 导航到 URL | url |
| click | 点击元素 | selector |
| fill | 填写输入框 | selector, value |
| text | 获取元素文本 | selector |
| screenshot | 截图 | path |
| evaluate | 执行 JavaScript | script |
| wait | 等待元素出现 | selector |
| select | 下拉选择 | selector, value |

**新增操作（v1.1）：**

| 操作 | 说明 | 必填参数 |
|------|------|----------|
| extract_text | 批量提取文本 | selector |
| extract_html | 批量提取 HTML | selector |
| extract_table | 提取表格数据 | selector（可选） |
| extract_links | 提取所有链接 | - |
| extract_attribute | 批量提取属性 | selector, attribute |
| scroll_to | 滚动页面 | to（bottom/top） |
| pdf | 生成 PDF | path |
| cookies | Cookie 管理 | action（get/set/clear） |
| set_headers | 设置 HTTP 头 | headers |
| new_page | 新建标签页 | url（可选） |
| switch_page | 切换标签页 | index |
| back / forward | 前进/后退 | - |
| reload | 刷新页面 | - |

**示例：抓取页面标题和链接**

```yaml
- id: open_page
  name: 打开网页
  type: browser
  config:
    action: navigate
    url: "https://example.com"

- id: get_title
  name: 获取标题
  type: browser
  config:
    action: evaluate
    script: "document.title"

- id: get_links
  name: 获取所有链接
  type: browser
  config:
    action: extract_links
```

**示例：截图整个页面**

```yaml
- id: full_screenshot
  name: 全页截图
  type: browser
  config:
    action: screenshot
    path: "./output/full_page.png"
    full_page: true
```

**示例：无限滚动加载**

```yaml
- id: scroll
  name: 滚动加载更多
  type: browser
  config:
    action: scroll_to
    to: bottom
    times: 5
    delay_ms: 2000
```

---

### 4.7 🕷️ 网页抓取（v1.1 新增）

声明式网页数据提取，无需写代码，配置 CSS 选择器即可抓取。

| 参数 | 说明 | 示例 |
|------|------|------|
| url | 目标网页 | `https://example.com/products` |
| wait_for | 等待选择器 | `.product-list` |
| extract | 提取规则（JSON 数组） | 见下方示例 |
| pagination.next | 下一页按钮选择器 | `.next-page` |
| pagination.max_pages | 最大翻页数 | 5 |
| scroll | 是否无限滚动 | true |
| scroll_times | 滚动次数 | 3 |
| delay_ms | 请求间隔（毫秒） | 1000 |
| headless | 无头模式 | true |
| proxy | 代理地址 | `http://proxy:8080` |

**示例：抓取商品列表**

```yaml
- id: scrape_products
  name: 抓取商品
  type: web_scrape
  config:
    url: "https://shop.example.com/products"
    wait_for: ".product-card"
    extract:
      - selector: ".product-card"
        fields:
          name: ".product-name"
          price: ".product-price"
          image: "img[src]"
          link: "a[href]"
    pagination:
      next: ".pagination .next"
      max_pages: 10
    delay_ms: 1500
```

**输出：**
```json
{
  "pages_scraped": 10,
  "total_items": 200,
  "items": [
    { "name": "商品A", "price": "¥99", "image": "https://...", "link": "https://..." },
    { "name": "商品B", "price": "¥199", "image": "https://...", "link": "https://..." }
  ]
}
```

**示例：抓取新闻（无限滚动）**

```yaml
- id: scrape_news
  name: 抓取新闻
  type: web_scrape
  config:
    url: "https://news.example.com"
    wait_for: ".article-list"
    extract:
      - selector: ".article-item"
        fields:
          title: "h2 a"
          summary: ".summary"
          date: ".publish-date"
          url: "h2 a[href]"
    scroll: true
    scroll_times: 10
    delay_ms: 2000
```

**extract 规则说明：**

每个 `extract` 规则包含：
- `selector`：匹配哪些元素（CSS 选择器）
- `fields`：从每个元素中提取哪些字段
  - 字段名：自定义名称
  - 字段值：元素内部的 CSS 选择器
  - 特殊语法：`a[href]`、`img[src]` 会自动提取属性值

**翻页说明：**

- `pagination.next`：点击哪个元素翻到下一页
- `pagination.max_pages`：最多翻几页
- 如果没配置 pagination，则只抓取一页

---

### 4.8 📗 Excel

读写 Excel 文件（.xlsx / .xls）。

| 操作 | 说明 |
|------|------|
| read | 读取整个工作表为二维数组 |
| write | 写入数据（覆盖） |
| append | 追加数据到末尾 |
| update | 更新指定单元格 |
| sheets | 获取所有工作表名称 |
| extract_column | 提取某一列 |

**示例：读取 Excel**

```yaml
- id: read_excel
  name: 读取Excel
  type: excel
  config:
    action: read
    path: "./data/input.xlsx"
    sheet: "Sheet1"
```

**输出：** `{ "headers": ["姓名","年龄","城市"], "rows": [["张三",25,"北京"],...], "row_count": 100 }`

**示例：写入 Excel**

```yaml
- id: write_excel
  name: 写入Excel
  type: excel
  config:
    action: write
    path: "./output/report.xlsx"
    sheet: "数据汇总"
    data:
      - ["姓名", "金额"]
      - ["张三", 1000]
      - ["李四", 2000]
```

**示例：更新特定单元格**

```yaml
- id: update_cell
  name: 更新单元格
  type: excel
  config:
    action: update
    path: "./data/report.xlsx"
    updates:
      - cell: "A1"
        value: "更新时间"
      - cell: "B1"
        value: "2026-04-26"
```

---

### 4.9 📘 Word

读写 Word 文档（.docx）。

| 操作 | 说明 |
|------|------|
| read | 读取文档文本 |
| write | 创建新文档 |
| append | 追加段落 |
| replace | 替换占位符（模板填充） |

**示例：模板填充**

```yaml
- id: fill_contract
  name: 填充合同
  type: word
  config:
    action: replace
    path: "./templates/contract.docx"
    output: "./output/contract_张三.docx"
    replacements:
      "{{甲方}}": "张三"
      "{{日期}}": "2026-04-26"
      "{{金额}}": "¥50,000"
```

**示例：创建报告**

```yaml
- id: create_report
  name: 创建报告
  type: word
  config:
    action: write
    path: "./output/report.docx"
    paragraphs:
      - "月度销售报告"
      - "本月销售额：¥100,000"
      - "环比增长：15%"
```

---

### 4.10 🔔 通知

| 类型 | 说明 |
|------|------|
| system | 系统弹窗通知 |
| webhook | 发送 HTTP POST 到指定 URL |

**示例：系统通知**

```yaml
- id: notify_done
  name: 完成通知
  type: notify
  config:
    notify_type: system
    title: "任务完成"
    body: "工作流已成功执行完毕"
```

**示例：Webhook 通知（如钉钉/飞书）**

```yaml
- id: webhook
  name: Webhook通知
  type: notify
  config:
    notify_type: webhook
    title: "数据更新"
    body: "抓取到 {{step_scrape.total_items}} 条数据"
    url: "https://hooks.example.com/xxx"
```

---

### 4.11 ✅ 审批

暂停工作流，等待用户在界面上点击「通过」或「拒绝」。

| 参数 | 说明 |
|------|------|
| message | 显示给用户的审批消息 |
| timeout | 超时时间（秒），超时自动拒绝 |

**示例：**

```yaml
- id: approval
  name: 人工确认
  type: approval
  config:
    message: "即将发送 100 封邮件，确认继续？"
    timeout: 300
```

执行时会弹出审批窗口，用户点击「通过」继续执行，点击「拒绝」终止。

---

### 4.12 ⚡ 并行

同时执行多组步骤。

```yaml
- id: parallel_tasks
  name: 并行执行
  type: parallel
  config:
    branches:
      - - id: fetch_a
          name: 抓取A站
          type: http
          config: { action: GET, url: "https://a.com/api" }
      - - id: fetch_b
          name: 抓取B站
          type: http
          config: { action: GET, url: "https://b.com/api" }
```

---

### 4.13 🔀 数据映射

将数组中的每个元素通过模板转换为新格式。

| 参数 | 说明 |
|------|------|
| source | 数据来源（变量路径） |
| template | 映射模板（JSON） |

**示例：格式化数据用于写入 Excel**

```yaml
- id: format_data
  name: 格式化数据
  type: map
  config:
    source: "step_scrape.items"
    template:
      cell: "A{{__index1}}"
      value: "{{__item.name}}"
```

---

## 5. 变量与表达式

### 5.1 变量引用语法

| 语法 | 说明 | 示例 |
|------|------|------|
| `{{key}}` | 引用工作流变量 | `{{api_key}}` |
| `{{step_xxx}}` | 引用整个步骤输出 | `{{step_get_data}}` |
| `{{step_xxx.field}}` | 引用步骤输出的嵌套字段 | `{{step_api.body.data}}` |
| `{{__item}}` | 循环体中的当前元素 | 循环节点内使用 |
| `{{__item.name}}` | 循环元素的嵌套字段 | `{{__item.email}}` |
| `{{__index}}` | 循环当前索引（从 0 开始） | 循环节点内使用 |
| `{{__index1}}` | 循环当前索引（从 1 开始） | 循环节点内使用 |

### 5.2 全局变量

在工作流编辑器的 YAML 中定义：

```yaml
name: 我的工作流
variables:
  api_key: "your-api-key-here"
  base_url: "https://api.example.com"
  max_retries: 3
steps:
  - id: fetch
    name: 获取数据
    type: http
    config:
      action: GET
      url: "{{base_url}}/data?key={{api_key}}"
```

### 5.3 变量面板

编辑器左侧面板会实时显示：
- **全局变量**：工作流中定义的 variables
- **步骤输出**：每个步骤的名称、类型、执行状态、耗时

点击任意变量即可复制引用语法到剪贴板。

### 5.4 类型保留

变量替换会保留原始数据类型，不会全部转为字符串：

```yaml
# 如果 step_api.body.count = 42（数字）
value: "{{step_api.body.count}}"   # → 42（仍然是数字，不是 "42"）

# 如果 step_data.items = ["a","b","c"]（数组）
data: "{{step_data.items}}"        # → ["a","b","c"]（仍然是数组）
```

---

## 6. 执行与调试

### 6.1 执行工作流

1. 打开工作流编辑器
2. 点击右上角 **▶ 执行** 按钮
3. 画布中每个步骤会显示实时状态：
   - ⏳ 等待中
   - ⚡ 执行中（显示已用时间）
   - ✅ 完成（显示总耗时）
   - ❌ 失败（显示错误信息）

### 6.2 查看输出

- 点击画布中已完成的步骤卡片，可查看该步骤的输出数据
- 运行历史页面可查看所有历史执行记录

### 6.3 取消执行

- 执行中点击 **⏹ 取消** 按钮立即终止

### 6.4 审批等待

如果工作流包含审批节点，执行会在该步骤暂停：
- 界面弹出审批窗口
- 点击 **通过** 继续执行
- 点击 **拒绝** 终止工作流
- 超时后自动拒绝

### 6.5 运行历史

首页点击 **📊 历史** 按钮：
- 查看所有执行记录
- 按工作流名称筛选
- 查看每步详情：状态、耗时、输入输出、错误信息
- 支持级联删除（删除工作流时同步删除历史）

---

## 7. 模板快速开始

首页点击 **📋 模板** 按钮，选择一个模板快速创建：

| 模板 | 说明 |
|------|------|
| HTTP GET | 单步 HTTP 请求 |
| Excel 读写 | 读取 + 写入 Excel |
| 浏览器操作 | 打开页面 + 等待 + 获取文本 |
| 网页抓取 | 声明式数据提取 |
| 抓取→Excel | 抓取网页数据写入 Excel |
| 循环+映射 | 遍历数组并转换格式 |
| 审批+通知 | 人工审批 + 完成通知 |

### 实战示例：抓取新闻写入 Excel

从模板创建后，修改配置：

```yaml
name: 抓取新闻到Excel
steps:
  - id: scrape
    name: 抓取新闻
    type: web_scrape
    config:
      url: "https://news.ycombinator.com"
      wait_for: ".itemlist"
      extract:
        - selector: ".athing"
          fields:
            title: ".titleline a"
            link: ".titleline a[href]"
            site: ".sitestr"

  - id: write
    name: 写入Excel
    type: excel
    config:
      action: write
      path: "./news.xlsx"
      sheet: "HN新闻"
      data:
        - ["标题", "链接", "来源"]
        # 用 map 节点转换 scrape 结果后写入
```

---

## 8. 定时调度

### 8.1 创建定时计划

1. 在首页找到目标工作流
2. 点击 ⏰ 图标打开调度设置
3. 输入 Cron 表达式
4. 点击保存

### 8.2 Cron 表达式

格式：`秒 分 时 日 月 周`

| 表达式 | 说明 |
|--------|------|
| `0 0 9 * * *` | 每天上午 9:00 |
| `0 */30 * * * *` | 每 30 分钟 |
| `0 0 9 * * 1-5` | 工作日上午 9:00 |
| `0 0 9 1 * *` | 每月 1 日上午 9:00 |
| `0 0 9,18 * * *` | 每天 9:00 和 18:00 |

### 8.3 管理调度

- 在调度设置中可启用/禁用
- 删除工作流时自动删除关联调度
- 应用关闭后调度停止，重新打开后自动恢复

---

## 9. 设置

### 9.1 常规设置

| 选项 | 说明 |
|------|------|
| Python 路径 | 浏览器节点使用的 Python 路径（留空自动检测） |
| 浏览器 | Chromium 内核浏览器选择（自动/Edge/Chrome） |
| 开机自启 | Windows 开机自动启动 |

### 9.2 浏览器节点说明

- 绿色版已内置 Python + Playwright，开箱即用
- 安装版需安装 Python 3.8+，首次使用浏览器节点时会自动安装 Playwright
- 如需使用浏览器节点，建议安装 Edge 或 Chrome 浏览器（无需下载 Playwright Chromium）

---

## 10. 常见问题

### Q: 浏览器节点报错「未找到 Python」

**A:** 
- 绿色版：检查 `embed/` 目录是否存在
- 安装版：安装 Python 3.8+ 并在设置中配置路径
- 或安装 Edge/Chrome 浏览器，应用会自动检测

### Q: 浏览器节点报错「Playwright 未安装」

**A:** 运行以下命令：
```bash
pip install playwright
playwright install chromium
```
或直接安装 Edge 浏览器，应用会优先使用系统浏览器。

### Q: Excel/Word 节点找不到文件

**A:** 使用绝对路径或相对于 exe 的路径。绿色版默认从 exe 所在目录查找。

### Q: 工作流执行很慢

**A:** 
- 浏览器节点最慢，建议减少不必要的 navigate/wait
- HTTP 请求可设置合理超时
- 使用并行节点同时执行多个独立步骤

### Q: 变量引用没有生效

**A:** 
- 检查变量名是否正确（区分大小写）
- 检查引用的步骤是否在当前步骤之前执行
- 使用 ✔ 验证 按钮检查 YAML 格式

### Q: 如何调试工作流

**A:** 
- 使用单步执行：每次只执行一个步骤，查看输出
- 查看运行历史中的步骤详情
- 使用 `evaluate` 节点执行 JS 调试浏览器状态

### Q: 网页抓取不到数据

**A:** 
- 检查 CSS 选择器是否正确（F12 开发者工具验证）
- 尝试增加 `wait_for` 等待时间
- 如果页面需要登录，先用浏览器节点 `cookies` 操作设置 Cookie
- 尝试关闭 `headless` 模式，观察浏览器行为
- 如果被反爬，尝试设置 `proxy` 和自定义 `user_agent`

### Q: 如何连接多个工作流

**A:** 目前不支持工作流间调用，但可以通过：
- Webhook 通知触发外部系统
- Excel/文件作为中间数据传递
- 后续版本会支持工作流嵌套

---

## 附录：节点配置速查表

| 节点类型 | 核心参数 |
|---------|---------|
| http | action, url, headers, body |
| data | action, key/value/path/source |
| script | script |
| condition | mode, op, left, right, true_next, false_next |
| loop | items, body, collect, table |
| while | items, condition, body, max_iterations |
| browser | action, url/selector/value/script/path |
| web_scrape | url, wait_for, extract, pagination, scroll |
| excel | action, path, sheet, data/column/updates |
| word | action, path, paragraphs/replacements |
| notify | notify_type, title, body, url |
| approval | message, timeout |
| map | source, template |
| parallel | branches |
