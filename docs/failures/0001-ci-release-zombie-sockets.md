# 0001-CI Release 工作流僵尸 socket 问题

> 日期：2026-05-31
> 严重程度：high
> 状态：resolved

## What Failed

在 WSL 环境中尝试本地编译 Windows 二进制时，出现僵尸 socket 问题，导致编译失败。

用户反馈："我不知道你要在这个版本问题上纠结多久。检查主仓库...修复好，推送 github 构建打包，再下载打包好的 Windows 版本运行测试"。

## Why It Failed

WSL 环境与 Windows 环境的网络栈不完全兼容，导致：
1. 本地编译产生的 socket 连接不稳定
2. 僵尸 socket 占用端口
3. 编译过程卡死

## Current Replacement

改为推 GitHub 让 CI 构建 Windows 二进制：
1. 本地修改 + 测试
2. 只推 commit（不触发 CI）
3. 手动触发 CI 构建
4. 下载 CI 构建的二进制测试

## Agent Guidance

当遇到本地编译问题时：
1. **不要在 WSL 中折腾本地编译**
2. 直接推 GitHub 让 CI 构建
3. 下载 CI 构建的产物测试
4. 如果需要修复，重复上述流程

## Regression Check

- [x] 流程变更：改为手动触发 CI
- [x] 文档更新：AGENTS.md 中记录发布流程
- [ ] 自动化检查：无（流程层面）

## References

- `ci.yml` — CI 工作流配置
- `AGENTS.md` — 发布流程说明
- 用户反馈："找到 bug 要先查找实测，不是猜测糊弄"
