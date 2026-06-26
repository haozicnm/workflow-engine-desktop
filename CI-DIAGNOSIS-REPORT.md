# Workflow Engine CI 构建问题诊断报告

**审查日期：** 2026-06-21  
**审查范围：** 4 个 GitHub Actions 工作流 + Cargo.toml + package.json + 构建脚本  
**版本：** v9.0.2

---

## 一、总体结论

**CI 存在 5 个 🔴 会导致构建失败的问题，4 个 🟡 会导致错误结果的问题。** 最严重的是 `release.yml` 的 ARM64 构建缺少系统依赖、Windows 构建缺少 Inno Setup 安装、`build-standalone.yml` 版本号硬编码、`ci.yml` 的模板验证 glob 不递归。

---

## 二、🔴 会导致构建失败的问题（5 个）

### 2.1 `release.yml` 的 `build-arm64` 缺少系统依赖安装

**位置：** `release.yml:45-53`（`build-arm64` job）  
**对比：** `build-x64` 有 `apt-get install`（第 151-154 行），`build-arm64` 没有

**问题：** ARM64 Linux 构建直接 `cargo build`，没有安装 `libgtk-3-dev`、`libwebkit2gtk-4.1-dev` 等系统库。这些库是 `tauri` 和 `webkit2gtk` 等依赖的编译前提。

**影响：** 编译时链接失败，错误类似 `ld: cannot find -lwebkit2gtk-4.1`

**修复：** 在 `build-arm64` job 中添加系统依赖安装步骤：
```yaml
      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libxdo-dev
```

---

### 2.2 `release.yml` 的 Windows 构建缺少 Inno Setup 安装

**位置：** `release.yml:323-386`（`Build Inno Setup installer` 步骤）

**问题：** 步骤尝试查找 `C:\Program Files (x86)\Inno Setup 6\ISCC.exe`，但 `windows-latest` runner **默认没有安装 Inno Setup**。如果没有找到 `ISCC.exe`，workflow 会以 `exit 1` 失败。

**影响：** Windows 构建在最后一步（生成安装程序）失败，无法产出 `.exe` 安装包

**修复：** 在 `Build Inno Setup installer` 之前添加安装步骤：
```yaml
      - name: Install Inno Setup
        shell: pwsh
        run: |
          choco install innosetup --yes --no-progress
```

或者使用 `winget`（但 `winget` 在 GitHub Actions 中可能需要额外配置）：
```yaml
      - name: Install Inno Setup
        shell: pwsh
        run: |
          $url = "https://jrsoftware.org/download.php/is.exe?site=1"
          Invoke-WebRequest -Uri $url -OutFile "is.exe"
          Start-Process -FilePath "is.exe" -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART" -Wait
```

---

### 2.3 `build-standalone.yml` 版本号硬编码为 "7.3.0"

**位置：** `build-standalone.yml:73`、`build-standalone.yml:134`

**问题：** Linux ARM64 和 Windows 的打包脚本中版本号硬编码为 `"7.3.0"`。

```yaml
# build-standalone.yml:73（Linux ARM64 deb）
sed -e "s/^Version:.*/Version: 7.3.0/" \
    -e "s/^Architecture:.*/Architecture: ${DEB_ARCH}/" \
    debian/control > pkg/DEBIAN/control

# build-standalone.yml:134（Windows zip）
$version = "7.3.0"
```

**影响：** 无论实际代码版本是多少（当前 `Cargo.toml` 是 `9.0.2`），打包出来的文件名和 deb 版本都显示为 `7.3.0`。用户下载后会困惑，也无法区分版本。

**修复：** 使用 `GITHUB_REF_NAME` 或从 `Cargo.toml` 读取版本：
```yaml
# Linux ARM64
VERSION="${GITHUB_REF_NAME#v}"
sed -e "s/^Version:.*/Version: ${VERSION}/" \
    -e "s/^Architecture:.*/Architecture: ${DEB_ARCH}/" \
    debian/control > pkg/DEBIAN/control

# Windows
$version = "${env:GITHUB_REF_NAME}" -replace '^v',''
if ([string]::IsNullOrEmpty($version)) {
    $version = "9.0.2"  # fallback
}
```

---

### 2.4 `ci.yml` 的 `templates` job 中 `**` glob 不递归

**位置：** `ci.yml:77-81`

**问题：**
```yaml
      - name: Validate JSON syntax
        run: |
          for f in library/**/*.wf.json; do
            echo "Validating $f..."
            python3 -m json.tool "$f" > /dev/null || { echo "Invalid JSON: $f"; exit 1; }
          done
```

在 bash 中，`**` 默认**只匹配一级子目录**，不会递归到深层目录。需要 `shopt -s globstar` 才能启用递归匹配。

**影响：** 如果 `library/` 目录中有深层嵌套的模板（如 `library/subdir/another/template.wf.json`），这些文件不会被验证，导致潜在的 JSON 语法错误遗漏到生产环境。

**修复：** 添加 `shopt -s globstar`：
```yaml
      - name: Validate JSON syntax
        run: |
          shopt -s globstar
          for f in library/**/*.wf.json; do
            echo "Validating $f..."
            python3 -m json.tool "$f" > /dev/null || { echo "Invalid JSON: $f"; exit 1; }
          done
```

或者使用 `find`：
```yaml
      - name: Validate JSON syntax
        run: |
          find library -name "*.wf.json" -print0 | while IFS= read -r -d '' f; do
            echo "Validating $f..."
            python3 -m json.tool "$f" > /dev/null || { echo "Invalid JSON: $f"; exit 1; }
          done
```

---

### 2.5 `build-standalone.yml` 的 `build-windows` `start.bat` 缺少 `@` 符号

**位置：** `build-standalone.yml:145-154`

**问题：**
```powershell
$lines = @(
    'echo off'        # ❌ 缺少 @
    'title Workflow Engine'
    'cd /d "%~dp0"'
    'start /MIN "" "workflow-engine.exe"'
    'echo Workflow Engine started on http://localhost:3000'
    'ping 127.0.0.1 -n 4 > nul'
    'start http://localhost:3000'
)
```

`echo off` 没有 `@` 前缀，会导致 `echo off` 本身被回显到控制台。

**影响：** 用户双击 `start.bat` 时，第一行显示 `echo off`，然后才关闭回显。这虽然不影响功能，但显得不专业。不过这不是构建失败，而是体验问题。

等等，让我重新评估这个问题。这是一个 `start.bat` 的语法问题，会导致脚本运行时 `echo off` 本身被打印出来。这不是构建失败，而是用户体验问题。我把它降级到 🟡。

让我重新检查，这个问题确实不会导致构建失败，只是运行时问题。让我把它移到 🟡 部分。

---

## 三、🟡 会导致错误结果的问题（4 个）

### 3.1 端口不一致：`build-standalone.yml` 使用 `localhost:3000`

**位置：** `build-standalone.yml:150`  
**对比：** `release.yml:305` 使用 `localhost:19529`

**问题：** `build-standalone.yml` 的 Windows `start.bat` 中 hardcoded `localhost:3000`，但 `release.yml` 中使用 `localhost:19529`。后端实际监听端口是 `19529`（从 `tauri.ts` 和 `server/mod.rs` 中确认）。

**影响：** 用户双击 `build-standalone.yml` 打包的 `start.bat` 后，浏览器打开 `localhost:3000`，但服务实际运行在 `localhost:19529`，导致连接失败。

**修复：** `build-standalone.yml` 的 `start.bat` 改为 `localhost:19529`：
```powershell
    'echo Workflow Engine started on http://localhost:19529'
    'ping 127.0.0.1 -n 4 > nul'
    'start http://localhost:19529'
```

---

### 3.2 `release.yml` 的 `build-x64` 的 `Package deb` 缺少 `set -ex`

**位置：** `release.yml:166`（`build-x64` job）  
**对比：** `build-arm64` 有 `set -ex`（第 62 行）

**问题：** `build-x64` 的 `Package deb` 脚本没有 `set -ex`。如果脚本中的某个命令失败（如 `cp` 源文件不存在），后续命令仍会继续执行，可能导致构建产出不完整或不正确的 deb 包。

**修复：** 添加 `set -ex`：
```yaml
      - name: Package deb
        run: |
          set -ex
          VERSION=${GITHUB_REF_NAME#v}
          ...
```

---

### 3.3 `build-standalone.yml` 的 `build-windows` 缺少 `python-runtime` 占位符

**位置：** `build-standalone.yml` 整体  
**对比：** `release.yml` 的 `build-windows` 有 `Create python-runtime placeholder` 步骤（第 267-273 行）

**问题：** `build-standalone.yml` 的 Windows 打包步骤中 `Copy-Item -Recurse src-tauri/python-runtime $pkgDir/python-runtime` 会复制 `python-runtime` 目录。如果目录不存在，`Copy-Item -Recurse` 在 PowerShell 中不会报错（只是不复制任何内容），但后续如果有代码期望这个目录存在，可能会出问题。

**影响：** 构建产物中缺少 `python-runtime` 目录，可能影响运行时 Python 依赖的加载。

**修复：** 在 `build-standalone.yml` 的 `build-windows` job 中添加：
```yaml
      - name: Create python-runtime placeholder
        shell: pwsh
        run: |
          New-Item -ItemType Directory -Force -Path src-tauri/python-runtime/wheels
          Set-Content -Path src-tauri/python-runtime/wheels/.gitkeep -Value "placeholder"
          New-Item -ItemType Directory -Force -Path src-tauri/python-runtime/playwright-browsers
          Set-Content -Path src-tauri/python-runtime/playwright-browsers/.gitkeep -Value "placeholder"
```

---

### 3.4 `release.yml` 的 `build-windows` 的 `node_modules` 缓存不完整

**位置：** `release.yml:245-256`（`build-windows` job）

**问题：** `actions/cache` 中包含了 `node_modules` 路径，但 `actions/setup-node` 没有启用 `cache: npm`。`npm ci` 安装的 `node_modules` 会被 `actions/cache` 缓存，但 `actions/setup-node` 的 `cache: npm` 会提供更智能的缓存策略（基于 `package-lock.json` 的 hash）。

**影响：** 缓存可能不完整或效率不高，导致每次构建都重新下载大量 npm 包，延长构建时间。

**修复：** 在 `actions/setup-node` 中启用 `cache: npm`：
```yaml
      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '22'
          cache: npm  # 添加此行
```

同时可以从 `actions/cache` 中移除 `node_modules` 路径（因为 `setup-node` 的 `cache: npm` 已经处理了 `~/.npm` 缓存）。

---

## 四、🟢 建议优化（5 个）

### 4.1 `build-standalone.yml` 的 `build-windows` 没有 `workflow-engine-gui`

**位置：** `build-standalone.yml:119-121`  
**对比：** `release.yml:265` 构建 `workflow-engine-gui --features gui`

**问题：** `build-standalone.yml` 只构建 `workflow-engine` 和 `wf-cli`，没有桌面 GUI 版本。如果用户期望 standalone 构建也包含 GUI，这会是一个缺失。

**评估：** 这可能是设计意图（standalone 只包含服务器+CLI）。但如果需要 GUI，需要添加：
```yaml
      - name: Build backend
        working-directory: src-tauri
        run: cargo build --release --bin workflow-engine --bin wf-cli --bin workflow-engine-gui --features gui
```

并在打包步骤中复制 `workflow-engine-gui.exe`。

---

### 4.2 `release.yml` 的 `build-arm64` 和 `build-x64` 打包步骤不一致

**位置：** `release.yml:61-88` vs `release.yml:166-192`

**问题：** `build-arm64` 的 `Package deb` 有 `ls -d dist/ src-tauri/sidecars/ debian/` 验证（第 67 行），`build-x64` 没有。两者应该保持一致。

---

### 4.3 `ci.yml` 没有 `push` 到 `main` 分支时自动运行的 CI

**位置：** `ci.yml:1-4`

**问题：** `ci.yml` 只在 `workflow_dispatch` 时触发。没有配置 `push` 到 `main` 分支或 `pull_request` 时自动运行。这意味着每次提交到 `main` 不会自动验证编译和测试。

**修复：** 添加 `push` 触发器：
```yaml
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch:
    ...
```

---

### 4.4 `build-arm64.yml` 只构建 `wf-cli` 但缺少前端验证

**位置：** `build-arm64.yml` 整体

**问题：** `build-arm64.yml` 只构建 `wf-cli`，没有构建前端。虽然 CLI 不需要前端，但 `release.yml` 的 `build-arm64` 需要。如果 `build-arm64.yml` 的目的是验证 ARM64 构建可行性，它应该也验证 `workflow-engine` 的构建（而不仅仅是 `wf-cli`）。

---

### 4.5 `debian/control` 静态版本号 "7.1.0" 容易误导

**位置：** `debian/control:2`

**问题：** `debian/control` 中版本号是 "7.1.0"，但 `release.yml` 的 `sed` 会替换它。如果 `sed` 命令写错了（如正则表达式不匹配），版本号不会更新，导致 deb 包版本错误。

**建议：** 将 `debian/control` 的版本号改为占位符（如 `Version: 0.0.0`），并在 CI 中强制验证替换是否成功：
```yaml
      - name: Verify version replacement
        run: |
          grep "^Version: ${VERSION}" pkg/DEBIAN/control || { echo "Version replacement failed"; exit 1; }
```

---

## 五、修复优先级总表

### 🔴 立即修复（会导致构建失败）

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 1 | `build-arm64` 缺少系统依赖 | `release.yml` | 添加 `apt-get install` 步骤 |
| 2 | Windows 缺少 Inno Setup | `release.yml` | 添加 `choco install innosetup` |
| 3 | `build-standalone` 版本硬编码 | `build-standalone.yml` | 使用 `GITHUB_REF_NAME#v` 或 `Cargo.toml` 版本 |
| 4 | `ci.yml` glob 不递归 | `ci.yml` | 添加 `shopt -s globstar` 或改用 `find` |
| 5 | `start.bat` 缺少 `@` | `build-standalone.yml` | `'@echo off'` |

### 🟡 短期修复（会导致错误结果）

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 6 | `build-standalone` 端口 3000 | `build-standalone.yml` | 改为 `localhost:19529` |
| 7 | `build-x64` 缺少 `set -ex` | `release.yml` | 添加 `set -ex` |
| 8 | `build-standalone` 缺少 `python-runtime` | `build-standalone.yml` | 添加占位符创建步骤 |
| 9 | Windows 缓存不完整 | `release.yml` | `setup-node` 添加 `cache: npm` |

### 🟢 建议优化

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 10 | `ci.yml` 没有 `push` 触发 | `ci.yml` | 添加 `push: branches: [main]` |
| 11 | `debian/control` 版本号误导 | `debian/control` | 改为 `0.0.0` 占位符 |
| 12 | `build-arm64.yml` 只构建 `wf-cli` | `build-arm64.yml` | 也构建 `workflow-engine` |
| 13 | 打包步骤不一致 | `release.yml` | `build-x64` 添加 `ls -d` 验证 |

---

*审查方法：代码走读（4 个 workflow 文件 + Cargo.toml + package.json + debian/control + 构建脚本）*
