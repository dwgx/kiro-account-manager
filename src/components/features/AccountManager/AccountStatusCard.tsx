import { CheckCircle } from 'lucide-react'
import { getThemeAccent } from '../KiroConfig/themeAccent'

interface AccountStatusCardProps {
  accountInfo: {
    email: string;
    subscriptionType: string;
    usage: { current: number; limit: number };
    resetTime?: string;
  };
  accent: ReturnType<typeof getThemeAccent>;
}

export function AccountStatusCard({ accountInfo, accent }: AccountStatusCardProps) {
  return (
    <div className={`p-4 rounded-xl border space-y-3 ${accent.subtleBg} border-primary/10`}>
      <div className="flex items-center justify-between border-b border-primary/10 pb-2">
        <span className="text-sm font-semibold text-foreground/80">当前账号状态</span>
        <div className="px-2.5 py-0.5 rounded-full bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 text-xs font-medium flex items-center gap-1.5">
          <CheckCircle size={14} />
          已验证
        </div>
      </div>
      <div className="grid grid-cols-2 gap-4 text-sm">
        <div>
          <span className="text-muted-foreground text-xs block mb-1">邮箱</span>
          <span className="font-medium font-mono text-xs truncate block" title={accountInfo.email}>
            {accountInfo.email}
          </span>
        </div>
        <div>
          <span className="text-muted-foreground text-xs block mb-1">订阅计划</span>
          <span className="font-medium">{accountInfo.subscriptionType}</span>
        </div>
        <div>
          <span className="text-muted-foreground text-xs block mb-1">使用额度</span>
          <span className="font-medium">
            {accountInfo.usage.current.toLocaleString()} / {accountInfo.usage.limit.toLocaleString()}
          </span>
        </div>
        <div>
          <span className="text-muted-foreground text-xs block mb-1">重置时间</span>
          <span className="font-medium">{accountInfo.resetTime ?? '-'}</span>
        </div>
      </div>
    </div>
  )
}
