---
version: alpha
name: workflow-engine-design
description: "一个兼容 shadcn/ui 的双主题设计系统，为 Vue 3 + Tauri 桌面端的工作流自动化引擎定制。深色碳灰画布 + 亮色暖白 + 翠绿色品牌强调 + Inter 字体 ss03。采用 shadcn 标准双层 oklch 结构（--background → @theme inline → --color-background）。"
colors:
  # ── shadcn/ui 标准 oklch 变量（亮色默认）──
  background: "240 10% 99%"
  foreground: "215 18% 7%"
  card: "0 0% 100%"
  card-foreground: "215 18% 7%"
  popover: "0 0% 100%"
  popover-foreground: "215 18% 7%"
  primary: "160 84% 39%"
  primary-foreground: "0 0% 0%"
  secondary: "220 14% 96%"
  secondary-foreground: "213 13% 38%"
  muted: "220 14% 96%"
  muted-foreground: "216 9% 47%"
  accent: "220 20% 97%"
  accent-foreground: "215 18% 7%"
  destructive: "347 66% 49%"
  destructive-foreground: "0 0% 100%"
  success: "140 62% 40%"
  success-foreground: "0 0% 100%"
  warning: "43 63% 50%"
  warning-foreground: "0 0% 0%"
  info: "210 100% 52%"
  info-foreground: "0 0% 100%"
  border: "212 12% 84%"
  input: "212 12% 84%"
  ring: "160 84% 39%"
  radius: "0.5rem"
  # ── 暗色覆盖（.dark）──
  dark-background: "240 5% 3%"
  dark-foreground: "240 10% 96%"
  dark-card: "240 7% 8%"
  dark-card-foreground: "240 10% 96%"
  dark-popover: "225 9% 10%"
  dark-popover-foreground: "240 10% 96%"
  dark-secondary: "225 9% 10%"
  dark-secondary-foreground: "224 15% 84%"
  dark-muted: "240 7% 8%"
  dark-muted-foreground: "222 9% 55%"
  dark-accent: "220 9% 12%"
  dark-accent-foreground: "240 10% 96%"
  dark-info: "202 100% 67%"
  dark-info-foreground: "0 0% 0%"
  dark-border: "225 8% 16%"
  dark-input: "225 8% 16%"
  # ── Workflow 专用（亮暗共用，值在 CSS 中处理）──
  node-idle: "220 13% 91%"
  node-running: "202 100% 67%"
  node-success: "140 62% 40%"
  node-error: "347 66% 49%"
  edge-color: "212 12% 84%"
  edge-active: "160 84% 39%"
  dark-node-idle: "220 6% 25%"
  dark-edge-color: "228 8% 22%"

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
    backgroundColor: "hsl(var(--background))"
    textColor: "hsl(var(--foreground))"
    typography: "{typography.body-sm}"
    height: 48px
  sidebar:
    backgroundColor: "hsl(var(--background))"
    textColor: "hsl(var(--secondary-foreground))"
    typography: "{typography.body-sm}"
    width: 240px
  sidebar-item-active:
    backgroundColor: "hsl(var(--secondary))"
    textColor: "hsl(var(--foreground))"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: 8px 12px
  button-primary:
    backgroundColor: "hsl(var(--primary))"
    textColor: "hsl(var(--primary-foreground))"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: 8px 16px
    height: 36px
  button-primary-hover:
    backgroundColor: "hsl(var(--primary) / 0.9)"
    textColor: "hsl(var(--primary-foreground))"
  button-secondary:
    backgroundColor: "hsl(var(--secondary))"
    textColor: "hsl(var(--foreground))"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: 8px 16px
    height: 36px
    border: 1px solid "hsl(var(--border))"
  button-ghost:
    backgroundColor: transparent
    textColor: "hsl(var(--secondary-foreground))"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: 6px 10px
  tab-list:
    backgroundColor: "hsl(var(--secondary))"
    rounded: "{rounded.md}"
    padding: 4px
  tab-trigger:
    backgroundColor: transparent
    textColor: "hsl(var(--muted-foreground))"
    typography: "{typography.body-sm}"
    rounded: "{rounded.sm}"
    padding: 6px 12px
  tab-trigger-active:
    backgroundColor: "hsl(var(--card))"
    textColor: "hsl(var(--foreground))"
  dropdown-content:
    backgroundColor: "hsl(var(--popover))"
    textColor: "hsl(var(--foreground))"
    rounded: "{rounded.md}"
    padding: 4px
    border: 1px solid "hsl(var(--border))"
  dropdown-item:
    backgroundColor: transparent
    textColor: "hsl(var(--foreground))"
    typography: "{typography.body-sm}"
    rounded: "{rounded.xs}"
    padding: 6px 10px
  dropdown-item-danger:
    backgroundColor: transparent
    textColor: "hsl(var(--destructive))"
    typography: "{typography.body-sm}"
  dialog-content:
    backgroundColor: "hsl(var(--popover))"
    textColor: "hsl(var(--foreground))"
    rounded: "{rounded.lg}"
    padding: 24px
    border: 1px solid "hsl(var(--border))"
  text-input:
    backgroundColor: "hsl(var(--secondary))"
    textColor: "hsl(var(--foreground))"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: 8px 12px
    height: 36px
    border: 1px solid "hsl(var(--border))"
  text-input-focus:
    border: 1px solid "hsl(var(--primary))"
  step-card:
    backgroundColor: "hsl(var(--card))"
    textColor: "hsl(var(--foreground))"
    rounded: "{rounded.lg}"
    border: 1px solid "hsl(var(--border))"
  step-card-running:
    border: 1px solid "hsl(var(--node-running))"
  step-card-success:
    border: 1px solid "hsl(var(--node-success))"
  step-card-error:
    border: 1px solid "hsl(var(--node-error))"
  canvas-node:
    backgroundColor: "hsl(var(--secondary))"
    textColor: "hsl(var(--foreground))"
    typography: "{typography.body-sm}"
    rounded: "{rounded.md}"
    padding: 12px 16px
    border: 1px solid "hsl(var(--border))"
    minWidth: 180px
  canvas-node-selected:
    border: 1px solid "hsl(var(--primary))"
  canvas-node-running:
    border: 1px solid "hsl(var(--node-running))"
  canvas-edge:
    strokeColor: "hsl(var(--edge-color))"
    strokeWidth: 1.5px
  canvas-edge-hover:
    strokeColor: "hsl(var(--edge-active))"
  port-dot:
    backgroundColor: "hsl(var(--border) / 0.8)"
    size: 8px
    rounded: "{rounded.full}"
  port-dot-hover:
    backgroundColor: "hsl(var(--primary))"
  badge:
    backgroundColor: "hsl(var(--popover))"
    textColor: "hsl(var(--muted-foreground))"
    typography: "{typography.caption}"
    rounded: "{rounded.full}"
    padding: 2px 8px
  toast:
    backgroundColor: "hsl(var(--popover))"
    textColor: "hsl(var(--foreground))"
    typography: "{typography.body-sm}"
    rounded: "{rounded.md}"
    padding: 12px 16px
    border: 1px solid "hsl(var(--border))"

---

## Overview

Workflow-Engine 是一个面向开发者的工作流自动化桌面应用（Vue 3 + Tauri + Rust）。设计系统兼容 shadcn/ui 标准——采用双层 oklch 结构（`--background` → `@theme inline` → `--color-background`），通过 `.dark` class 切换亮暗主题。深色碳灰画布 + 亮色暖白 + 翠绿色唯一强调色。

**Key Characteristics:**
- 兼容 shadcn/ui CSS 变量体系，所有组件可直接使用 `bg-background`、`text-foreground` 等标准类
- 深色画布（`var(--background)` — oklch(0.131 0.002 286)）
- 亮色画布（`var(--background)` — oklch(0.992 0.001 286)，无 `.dark` 时的默认值）
- 翠绿色 `var(--primary)` (oklch(0.69 0.148 162)) 作为唯一品牌色
- Hairline 1px 边框承载卡片层次，无投影
- Inter 字体 + ss03 stylistic set
- 统一 8px 按钮圆角（`--radius: 0.5rem`），12px 卡片圆角
- 节点执行状态色：idle/灰、running/蓝、success/绿、error/红

## Colors

### Brand & Accent
- **翠绿色** (`var(--primary)` — oklch(0.69 0.148 162) / `#10b981`)：主 CTA、选中态、focus ring。唯一品牌色，克制使用。
- **亮暗一致**：primary 色在亮暗主题下保持不变。

### Surface
- **Background** (`--background`)：页面底色。亮色 `240° 10% 99%`（暖白 `#fcfcfd`），暗色 `240° 5% 3%`（碳灰 `#090a0b`）。
- **Card** (`--card`)：卡片背景。亮色 `#ffffff`，暗色 `#111214`。
- **Popover** (`--popover`)：弹出层、dropdown、dialog。亮色 `#ffffff`，暗色 `#16171a`。
- **Secondary** (`--secondary`)：次要表面。亮色 `#f3f4f6`，暗色 `#16171a`。
- **Muted** (`--muted`)：弱化表面。亮色 `#f3f4f6`，暗色 `#111214`。
- **Accent** (`--accent`)：强调表面。亮色 `#f9fafb`，暗色 `#1a1c1f`。
- **Border** (`--border`)：1px 卡片/组件边框。亮色 `#d0d7de`，暗色 `#23252a`。

### Text
- **Foreground** (`--foreground`)：标题和强调文本。亮色 `#0d1117`，暗色 `#f4f4f6`。
- **Secondary Foreground** (`--secondary-foreground`)：次要文本（原 ink-muted）。亮色 `#57606a`，暗色 `#d0d6e0`。
- **Muted Foreground** (`--muted-foreground`)：三级文本（原 ink-subtle）。亮色 `#6e7681`，暗色 `#8a8f98`。

### Semantic（语义色，亮暗一致）
- **Success** (`--success` — `#27a644`)：节点执行成功。
- **Destructive** (`--destructive` — `#cf2d56`)：节点执行失败。
- **Warning** (`--warning` — `#d4a72c`)：警告。
- **Info** (`--info`)：信息提示。亮色 `#0969da`，暗色 `#57c1ff`。

### Workflow（节点/连线专用）
- **Node Idle** (`--node-idle`)：待执行节点。亮色 `#e5e7eb`，暗色 `#3a3d42`。
- **Node Running** (`--node-running` — `#57c1ff`)：执行中节点。
- **Node Success** (`--node-success` — `#27a644`)：成功节点。
- **Node Error** (`--node-error` — `#cf2d56`)：失败节点。
- **Edge Color** (`--edge-color`)：默认连线。亮色 `#d0d7de`，暗色 `#34343a`。
- **Edge Active** (`--edge-active` — `#10b981`)：激活/悬停连线。

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
深色画布本身就是"留白"。区域间用 `hsl(var(--secondary))` 面板和 1px `hsl(var(--border))` 分隔，间距 `{spacing.section}` 64px。

## Elevation & Depth

| Level | Treatment | Use |
|---|---|---|
| 0 — Background | `hsl(var(--background))` | 主背景、画布 |
| 1 — Card | `hsl(var(--card))` + 1px border | 步骤卡片、面板 |
| 2 — Elevated | `hsl(var(--popover))` + 1px border | Dialog、Dropdown |
| 3 — Accent | `hsl(var(--accent))` | 嵌套面板 |

系统**不使用阴影**。层次完全由背景色阶梯 + hairline 边框表达。亮暗主题通过 `.dark` class 自动切换表面层次。

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
- **`button-primary`** — 翠绿色 CTA，用 `hsl(var(--primary))`。慎用——每屏最多一个。
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
- **`canvas-node`** — secondary 背景，8px 圆角，1px border 边框，最小宽度 180px。选中时边框变为 `hsl(var(--primary))`。
- **`canvas-edge`** — 1.5px 贝塞尔曲线，默认 `hsl(var(--edge-color))`，悬停/选中 `hsl(var(--edge-active))`。
- **`port-dot`** — 8px 圆点，默认 hairline-strong，悬停变 primary。

### Dialog
- **`dialog-content`** — surface-2 背景，12px 圆角，hairline 边框，24px 内边距。

### Toast
- **`toast`** — surface-2 背景，8px 圆角，hairline 边框，12px-16px 内边距。

## Do's and Don'ts

### Do
- 用 `hsl(var(--primary))` 翠绿色**仅**用于主 CTA、选中态、focus ring。
- 暗色模式用 surface ladder（canvas → surface-1 → surface-2）表达层次，不用阴影。
- 亮色模式用 white → gray-50 → gray-100 阶梯表达层次，hairline 边框承载边界。
- 用 1px hairline 边框承载所有卡片边界。
- 按钮统一 8px 圆角，卡片 12px 圆角。
- 节点执行状态色固定在 idle/running/success/error 四色体系内。
- 启用 Inter 的 ss03 stylistic set。

### Don't
- 不要引入第二个品牌色。翠绿色是唯一强调色。
- 不要用 box-shadow 表达深度。
- 暗色模式不要用纯黑 `#000000`。Canvas 必须是 `#090a0b`。
- 亮色模式不要用纯白 `#ffffff` 作 canvas。Canvas 用 `#fcfcfd` 暖白。
- 不要在不同组件间混用圆角（按钮 6-8px，卡片 12px）。
- 不要用绿色表示成功操作同时用作品牌色——`hsl(var(--primary))` ≠ `hsl(var(--success))`。

## Iteration Guide

1. 引用 `{token.refs}`，不写 inline hex。
2. 新组件加 `components:` 块，命名用 kebab-case。
3. 组件变体（hover/pressed/active）作为独立 entry。
4. 按钮默认 `{rounded.md}` 8px，卡片 `{rounded.lg}` 12px。
5. 所有文本默认 `{typography.body}` 14px/400/1.5。
6. Canvas 节点状态色映射：idle → `hsl(var(--node-idle))`、running → `hsl(var(--node-running))`、success → `hsl(var(--node-success))`、error → `hsl(var(--node-error))`。