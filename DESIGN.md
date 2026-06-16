---
version: alpha
name: workflow-engine-design
description: "一个深色画布的开发者工具设计系统，为 Vue 3 + Tauri 桌面端的工作流自动化引擎定制。暖炭灰画布 + 翠绿色品牌强调 + Inter 字体 ss03 + surface ladder 层级。借鉴 Linear 的深色克制、Raycast 的桌面原生感、Cursor 的单一强调色哲学。"
colors:
  primary: "#10b981"
  primary-hover: "#34d399"
  primary-pressed: "#059669"
  ink: "#f4f4f6"
  ink-muted: "#d0d6e0"
  ink-subtle: "#8a8f98"
  ink-tertiary: "#62666d"
  on-primary: "#000000"
  canvas: "#090a0b"
  surface-1: "#111214"
  surface-2: "#16171a"
  surface-3: "#1a1c1f"
  surface-card: "#141518"
  hairline: "#23252a"
  hairline-strong: "#34343a"
  semantic-success: "#27a644"
  semantic-error: "#cf2d56"
  semantic-warning: "#d4a72c"
  semantic-info: "#57c1ff"
  node-idle: "#3a3d42"
  node-running: "#57c1ff"
  node-success: "#27a644"
  node-error: "#cf2d56"
  edge-color: "#34343a"
  edge-active: "#10b981"

typography:
  display-lg:
    fontFamily: Inter
    fontSize: 32px
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: -0.5px
    fontFeature: '"calt", "kern", "liga", "ss03"'
  display-md:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: -0.3px
    fontFeature: '"calt", "kern", "liga", "ss03"'
  heading:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: 0
    fontFeature: '"calt", "kern", "liga", "ss03"'
  subhead:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: 0
    fontFeature: '"calt", "kern", "liga", "ss03"'
  body:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: 0
    fontFeature: '"calt", "kern", "liga", "ss03"'
  body-sm:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: 0
    fontFeature: '"calt", "kern", "liga", "ss03"'
  caption:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: 0
    fontFeature: '"calt", "kern", "liga", "ss03"'
  button:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: 500
    lineHeight: 1.2
    letterSpacing: 0
    fontFeature: '"calt", "kern", "liga", "ss03"'
  code:
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace"
    fontSize: 13px
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: 0
  eyebrow:
    fontFamily: Inter
    fontSize: 11px
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: 0.8px
    textTransform: uppercase
    fontFeature: '"calt", "kern", "liga", "ss03"'

rounded:
  none: 0px
  xs: 4px
  sm: 6px
  md: 8px
  lg: 12px
  xl: 16px
  full: 9999px

spacing:
  xxs: 4px
  xs: 8px
  sm: 12px
  md: 16px
  lg: 24px
  xl: 32px
  xxl: 48px
  section: 64px

components:
  top-bar:
    backgroundColor: "{colors.canvas}"
    textColor: "{colors.ink}"
    typography: "{typography.body-sm}"
    height: 48px
  sidebar:
    backgroundColor: "{colors.canvas}"
    textColor: "{colors.ink-muted}"
    typography: "{typography.body-sm}"
    width: 240px
  sidebar-item-active:
    backgroundColor: "{colors.surface-1}"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: 8px 12px
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: 8px 16px
    height: 36px
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
    textColor: "{colors.on-primary}"
  button-secondary:
    backgroundColor: "{colors.surface-1}"
    textColor: "{colors.ink}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: 8px 16px
    height: 36px
    border: 1px solid "{colors.hairline}"
  button-ghost:
    backgroundColor: transparent
    textColor: "{colors.ink-muted}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: 6px 10px
  tab-list:
    backgroundColor: "{colors.surface-1}"
    rounded: "{rounded.md}"
    padding: 4px
  tab-trigger:
    backgroundColor: transparent
    textColor: "{colors.ink-subtle}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.sm}"
    padding: 6px 12px
  tab-trigger-active:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.ink}"
  dropdown-content:
    backgroundColor: "{colors.surface-2}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: 4px
    border: 1px solid "{colors.hairline}"
  dropdown-item:
    backgroundColor: transparent
    textColor: "{colors.ink}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.xs}"
    padding: 6px 10px
  dropdown-item-danger:
    backgroundColor: transparent
    textColor: "{colors.semantic-error}"
    typography: "{typography.body-sm}"
  dialog-content:
    backgroundColor: "{colors.surface-2}"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    padding: 24px
    border: 1px solid "{colors.hairline}"
  text-input:
    backgroundColor: "{colors.surface-1}"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: 8px 12px
    height: 36px
    border: 1px solid "{colors.hairline}"
  text-input-focus:
    border: 1px solid "{colors.primary}"
  step-card:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    border: 1px solid "{colors.hairline}"
  step-card-running:
    border: 1px solid "{colors.node-running}"
  step-card-success:
    border: 1px solid "{colors.node-success}"
  step-card-error:
    border: 1px solid "{colors.node-error}"
  canvas-node:
    backgroundColor: "{colors.surface-1}"
    textColor: "{colors.ink}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.md}"
    padding: 12px 16px
    border: 1px solid "{colors.hairline}"
    minWidth: 180px
  canvas-node-selected:
    border: 1px solid "{colors.primary}"
  canvas-node-running:
    border: 1px solid "{colors.node-running}"
  canvas-edge:
    strokeColor: "{colors.edge-color}"
    strokeWidth: 1.5px
  canvas-edge-hover:
    strokeColor: "{colors.edge-active}"
  port-dot:
    backgroundColor: "{colors.hairline-strong}"
    size: 8px
    rounded: "{rounded.full}"
  port-dot-hover:
    backgroundColor: "{colors.primary}"
  badge:
    backgroundColor: "{colors.surface-2}"
    textColor: "{colors.ink-subtle}"
    typography: "{typography.caption}"
    rounded: "{rounded.full}"
    padding: 2px 8px
  toast:
    backgroundColor: "{colors.surface-2}"
    textColor: "{colors.ink}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.md}"
    padding: 12px 16px
    border: 1px solid "{colors.hairline}"

---

## Overview

Workflow-Engine 是一个面向开发者的工作流自动化桌面应用（Vue 3 + Tauri + Rust）。设计系统走**深色克制路线**——借鉴 Linear 的最深黑色画布哲学和 Raycast 的"产品即品牌"理念，用碳灰画布承载翠绿色作为唯一强调色，通过 surface ladder 表达层次而非阴影。

**Key Characteristics:**
- 深色画布（`{colors.canvas}` — #090a0b），暖色调黑，不是纯黑
- 翠绿色 `{colors.primary}` (#10b981) 作为唯一品牌色，仅用于主 CTA、focus ring、选中状态
- 4 级 surface ladder：canvas → surface-1 → surface-2 → surface-3
- Hairline 1px 边框承载卡片层次，无投影
- Inter 字体 + ss03 stylistic set（同 Raycast 的签名细节）
- 统一 8px 按钮圆角，12px 卡片圆角
- 节点执行状态色：idle/灰、running/蓝、success/绿、error/红

## Colors

### Brand & Accent
- **翠绿色** (`{colors.primary}` — #10b981)：主 CTA、选中态、focus ring。唯一品牌色，克制使用。
- **Hover** (`{colors.primary-hover}` — #34d399)：按钮悬停态，稍亮。
- **Pressed** (`{colors.primary-pressed}` — #059669)：按下态，加深。

### Surface
- **Canvas** (`{colors.canvas}` — #090a0b)：页面底色，暖色调极深灰。
- **Surface-1** (`{colors.surface-1}` — #111214)：卡片、面板第一层级。
- **Surface-2** (`{colors.surface-2}` — #16171a)：弹出层、dropdown、dialog。
- **Surface-3** (`{colors.surface-3}` — #1a1c1f)：嵌套面板背景。
- **Surface Card** (`{colors.surface-card}` — #141518)：步骤卡片专用背景。
- **Hairline** (`{colors.hairline}` — #23252a)：1px 卡片边框。
- **Hairline Strong** (`{colors.hairline-strong}` — #34343a)：强化边框，focus ring 备选。

### Text
- **Ink** (`{colors.ink}` — #f4f4f6)：标题和强调文本。近白但不刺眼。
- **Ink Muted** (`{colors.ink-muted}` — #d0d6e0)：二级文本。
- **Ink Subtle** (`{colors.ink-subtle}` — #8a8f98)：三级文本，侧边栏项目。
- **Ink Tertiary** (`{colors.ink-tertiary}` — #62666d)：禁用态文本。

### Semantic（执行状态）
- **Success** (`{colors.semantic-success}` — #27a644)：节点执行成功。
- **Error** (`{colors.semantic-error}` — #cf2d56)：节点执行失败。
- **Warning** (`{colors.semantic-warning}` — #d4a72c)：警告。
- **Info** (`{colors.semantic-info}` — #57c1ff)：信息提示。
- **Node Idle** (`{colors.node-idle}` — #3a3d42)：待执行节点。
- **Node Running** (`{colors.node-running}` — #57c1ff)：执行中节点。
- **Node Success** (`{colors.node-success}` — #27a644)：成功节点。
- **Node Error** (`{colors.node-error}` — #cf2d56)：失败节点。

### Canvas（连线）
- **Edge Color** (`{colors.edge-color}` — #34343a)：默认连线颜色。
- **Edge Active** (`{colors.edge-active}` — #10b981)：激活/悬停连线。

## Typography

### Font Family
**Inter** 为主字体，启用 `font-feature-settings: "calt", "kern", "liga", "ss03"`（同 Raycast，ss03 开启 Inter 的替代 g 字形）。代码区使用 **JetBrains Mono**。

### Hierarchy

| Token | Size | Weight | Line Height | Letter Spacing | Use |
|---|---|---|---|---|---|
| `{typography.display-lg}` | 32px | 600 | 1.2 | -0.5px | Canvas 视图标题 |
| `{typography.display-md}` | 24px | 600 | 1.25 | -0.3px | Section 标题 |
| `{typography.heading}` | 18px | 600 | 1.4 | 0 | 卡片标题 |
| `{typography.subhead}` | 16px | 500 | 1.4 | 0 | 子标题 |
| `{typography.body}` | 14px | 400 | 1.5 | 0 | 正文 |
| `{typography.body-sm}` | 13px | 400 | 1.4 | 0 | 辅助文本 |
| `{typography.caption}` | 12px | 400 | 1.4 | 0 | 标签/注释 |
| `{typography.button}` | 14px | 500 | 1.2 | 0 | 按钮标签 |
| `{typography.code}` | 13px | 400 | 1.5 | 0 | YAML/代码块 |
| `{typography.eyebrow}` | 11px | 600 | 1.4 | 0.8px uppercase | 分区标签 |

## Layout

### Spacing System
- **Base unit:** 4px
- **Tokens:** `{spacing.xxs}` 4px · `{spacing.xs}` 8px · `{spacing.sm}` 12px · `{spacing.md}` 16px · `{spacing.lg}` 24px · `{spacing.xl}` 32px · `{spacing.xxl}` 48px · `{spacing.section}` 64px
- Canvas 节点间距：`{spacing.lg}` 24px horizontal，`{spacing.md}` 16px vertical
-卡片内边距：`{spacing.md}` 16px（标准）、`{spacing.lg}` 24px（大卡片）

### Grid & Container
- 侧边栏：240px 固定宽度，左侧
- 主内容区：`flex-1` 自适应剩余宽度
- Canvas 画布：无边距，节点自由定位
- 对话框：max-w-md（~448px）居中

### Whitespace
深色画布本身就是"留白"。区域间用 `{colors.surface-1}` 面板和 1px `{colors.hairline}` 分隔，间距 `{spacing.section}` 64px。

## Elevation & Depth

| Level | Treatment | Use |
|---|---|---|
| 0 — Canvas | `{colors.canvas}` | 主背景、画布 |
| 1 — Card | `{colors.surface-card}` + 1px hairline | 步骤卡片、面板 |
| 2 — Elevated | `{colors.surface-2}` + 1px hairline | Dialog、Dropdown |
| 3 — Overlay | `{colors.surface-3}` | 嵌套面板 |

系统**不使用阴影**。层次完全由背景色阶梯 + hairline 边框表达。

## Shapes

### Border Radius

| Token | Value | Use |
|---|---|---|
| `{rounded.none}` | 0px | Canvas 画布 |
| `{rounded.xs}` | 4px | Badge、Port 圆点 |
| `{rounded.sm}` | 6px | Tab trigger |
| `{rounded.md}` | 8px | 按钮、输入框、Dropdown |
| `{rounded.lg}` | 12px | 步骤卡片、Dialog |
| `{rounded.xl}` | 16px | 大容器（罕用） |
| `{rounded.full}` | 9999px | Badge pill、Port 圆点 |

## Components

### Top Bar
**`top-bar`** — 48px 高度，canvas 背景。左侧 workflow 标题，右侧运行/保存/设置按钮。

### Sidebar
**`sidebar`** — 240px 宽，canvas 背景。工作流列表，选中项用 `surface-1` 背景反色。

### Buttons
- **`button-primary`** — 翠绿色 CTA，用 `{colors.primary}`。慎用——每屏最多一个。
- **`button-secondary`** — surface-1 背景 + hairline 边框。
- **`button-ghost`** — 透明背景，悬停时显示底色。

### Tabs
- **`tab-list`** — surface-1 背景，4px 内边距，6px 圆角。
- **`tab-trigger`** — 默认透明，激活时 surface-card 背景。

### Dropdown
- **`dropdown-content`** — surface-2 背景，4px 内边距，8px 圆角，hairline 边框。

### Step Card
- **`step-card`** — surface-card + 1px hairline，12px 圆角。
- 执行状态：idle（默认边框）、running（蓝色边框）、success（绿色边框）、error（红色边框）。

### Canvas
- **`canvas-node`** — surface-1 背景，8px 圆角，1px hairline 边框，最小宽度 180px。选中时边框变为 `{colors.primary}`。
- **`canvas-edge`** — 1.5px 贝塞尔曲线，默认 `{colors.edge-color}`，悬停/选中 `{colors.edge-active}`。
- **`port-dot`** — 8px 圆点，默认 hairline-strong，悬停变 primary。

### Dialog
- **`dialog-content`** — surface-2 背景，12px 圆角，hairline 边框，24px 内边距。

### Toast
- **`toast`** — surface-2 背景，8px 圆角，hairline 边框，12px-16px 内边距。

## Do's and Don'ts

### Do
- 用 `{colors.primary}` 翠绿色**仅**用于主 CTA、选中态、focus ring。
- 用 surface ladder（canvas → surface-1 → surface-2）表达层次，不用阴影。
- 用 1px hairline 边框承载所有卡片边界。
- 按钮统一 8px 圆角，卡片 12px 圆角。
- 节点执行状态色固定在 idle/running/success/error 四色体系内。
- 启用 Inter 的 ss03 stylistic set。

### Don't
- 不要引入第二个品牌色。翠绿色是唯一强调色。
- 不要用 box-shadow 表达深度。
- 不要用纯黑 `#000000`。Canvas 必须是 `#090a0b`。
- 不要在不同组件间混用圆角（按钮 6-8px，卡片 12px）。
- 不要用绿色表示成功操作同时用作品牌色——`{colors.primary}` ≠ `{colors.semantic-success}`。

## Iteration Guide

1. 引用 `{token.refs}`，不写 inline hex。
2. 新组件加 `components:` 块，命名用 kebab-case。
3. 组件变体（hover/pressed/active）作为独立 entry。
4. 按钮默认 `{rounded.md}` 8px，卡片 `{rounded.lg}` 12px。
5. 所有文本默认 `{typography.body}` 14px/400/1.5。
6. Canvas 节点状态色映射：idle → `{colors.node-idle}`、running → `{colors.node-running}`、success → `{colors.node-success}`、error → `{colors.node-error}`。