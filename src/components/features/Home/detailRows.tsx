// 配额行
export function QuotaRow({ label, used, limit, percent, color, accent, expiry }: {
  label: string;
  used: number;
  limit: number;
  percent: number;
  color: 'blue' | 'purple' | 'amber';
  accent: any;
  expiry?: number;
}) {
  const colorMap = {
    blue: { dot: 'bg-blue-500', bar: 'bg-blue-500', text: 'text-blue-600' },
    purple: { dot: 'bg-purple-500', bar: 'bg-purple-500', text: 'text-purple-600' },
    amber: { dot: 'bg-amber-500', bar: 'bg-amber-500', text: 'text-amber-600' },
  }
  const c = colorMap[color]
  const expiryStr = expiry ? (() => {
    try {
      const date = new Date(expiry * 1000)
      return !isNaN(date.getTime()) ? date.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' }) : null
    } catch {
      return null
    }
  })() : null

  return (
    <div className="flex items-center gap-2">
      <div className={`w-1.5 h-1.5 rounded-full ${c.dot} shrink-0`} />
      <span className="text-[11px] text-muted-foreground w-10 shrink-0" title={expiryStr ? `${expiryStr} 到期` : ''}>{label}</span>
      <div className="flex-1 h-[3px] bg-muted rounded-full overflow-hidden">
        <div className={`h-full rounded-full ${c.bar} transition-all`} style={{ width: `${Math.min(percent, 100)}%` }} />
      </div>
      <span className={`text-[10px] font-mono ${c.text} w-20 text-right shrink-0`}>
        {used}/{limit}{expiryStr ? ` · ${expiryStr}` : ''}
      </span>
    </div>
  )
}

// 信息行
export function InfoRow({ label, value, valueClass, mono }: {
  label: string;
  value: string;
  valueClass?: string;
  mono?: boolean;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-[11px] text-muted-foreground">{label}</span>
      <span className={`text-[11px] ${valueClass || 'text-foreground'} ${mono ? 'font-mono' : ''} truncate max-w-[100px]`}>{value}</span>
    </div>
  )
}
