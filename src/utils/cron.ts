// ─── Cron 表达式描述 ───

export function describeCron(expr: string): string {
  const map: Record<string, string> = {
    '0 * * * *': '每小时整点',
    '0 9 * * *': '每天 09:00',
    '0 9 * * 1-5': '工作日 09:00',
    '0 9 * * 1': '每周一 09:00',
    '0 0 * * *': '每天午夜',
    '*/30 * * * *': '每 30 分钟',
    '*/5 * * * *': '每 5 分钟',
  }
  return map[expr] || expr
}
