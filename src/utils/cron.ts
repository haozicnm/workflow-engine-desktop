// ─── Cron 表达式描述 ───

export function describeCron(expr: string): string {
  const map: Record<string, string> = {
    '0 * * * *': '00:00',
    '0 9 * * *': '天 09:00',
    '0 9 * * 1-5': 'Workday 09:00',
    '0 9 * * 1': '周一 09:00',
    '0 0 * * *': '天午夜',
    '*/30 * * * *': ' 30 分钟',
    '*/5 * * * *': ' 5 分钟',
  }
  return map[expr] || expr
}
