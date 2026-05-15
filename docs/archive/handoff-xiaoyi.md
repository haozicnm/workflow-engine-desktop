# 督促交接 — 若海 → 小艺

> 日期：2026-04-30 | 移交：cc12c1883be9 → 小艺 agent

---

## 当前机制

Hermes cron job `cc12c1883be9`，每 10 分钟执行一次。

## 执行逻辑

### 1. 检查进展
```bash
cd /home/horizon/workflow-engine-desktop
git log --oneline --since="10 minutes ago" --format="%h %ai %s"
```

### 2. 更新进度文档
文件：`/home/horizon/workflow-engine-desktop/docs/progress-log.md`

规则：
- 只改第 4 行「最后检查」时间戳
- 有新进展 → 读完整文件，末尾 append 新条目，写回
- **绝对禁止** `<at>` 标签写进 .md（那是飞书格式，不是 Markdown）
- **绝对禁止** 在文件中间插入条目 → 永远末尾 append

追加格式：
```markdown
### ⏱️ HH:MM 小艺检查
- 状态...
```

### 3. 群内督促
目标：飞书群 `oc_d61055168f24d9a326282b17b69b4c77`

飞书 @ 语法（不是纯文本 `@`！）：
```
<at user_id="ou_4649803d75948bee44c028126a72f8a3">若溪</at>
<at user_id="ou_fd160afc3cc50ca0a996a05eef69f26b">若海</at>
<at user_id="ou_c8b39c2d59409a84e0a4117a5afed591">伟哥</at>
```

发言规则：
- **每次双 @ 前置**：`<at>若溪</at> <at>若海</at>` 开头，缺一不可
- 有新进展 → 报告 + 布置下一步
- 30 分钟无进展 → 温和督促
- 完全无变化 → 静默（不发消息）

## 分工参考

| 角色 | 负责 | 当前阶段 |
|------|------|----------|
| 若海 | 后端 / 基础设施 | M2 后端节点（delay done，loop/while/condition/sub_workflow 待做） |
| 若溪 | 前端 / 应用层 | M2 PropertyPanel + 前端节点注册 |
| 伟哥 | 审批 / 实机验证 | M1 Windows 冒烟测试 |

## M1 状态
- ✅ CI 通过（Build Windows #25138548653）
- ✅ NSIS artifact 6.2MB
- ⏳ 伟哥 Windows 实机冒烟测试

## M2 状态
- ✅ delay 节点完成（`987b6c0`）
- ⬚ loop / while / condition / sub_workflow 后端执行器
- ⬚ PropertyPanel 配置面板（若溪）
- ⬚ data 节点（若溪）

## 交接确认

小艺接手后在群里发一条确认消息，海弟随后停掉 `cc12c1883be9`。

---

> 原 cron prompt 全文见 Hermes 后台：`cronjob action=list` → job `cc12c1883be9`
