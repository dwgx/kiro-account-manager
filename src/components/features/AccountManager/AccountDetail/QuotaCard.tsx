import { memo } from 'react'
import React from 'react'
import { formatUsage } from '../../../../utils/accountStats'

interface QuotaCardProps {
  title: string;
  used: number;
  quota: number;
  icon: string | React.ReactNode;
  status?: string;
  expiry?: string | null;
  colors: any;
  t: any;
}

// 配额卡片组件（优化性能）
const QuotaCard = memo(({ title, used, quota, icon, status, expiry, colors, t }: QuotaCardProps) => {
  const isActive = status === 'ACTIVE'
  const hasQuota = quota > 0

  return (
    <div className={`rounded-lg p-3 border transition-colors duration-200 hover:shadow-md ${
      hasQuota && isActive
        ? 'border-blue-500/30 bg-blue-500/5 shadow-blue-500/10'
        : `border-border bg-muted/30`
    }`}>
      <div className="flex items-center gap-2 mb-3">
        <div className={`w-2.5 h-2.5 rounded-full ${
          hasQuota && isActive
            ? title.includes('试用')
              ? 'bg-cyan-500 shadow-lg shadow-cyan-500/50'
              : title.includes('奖励')
                ? 'bg-purple-500 shadow-lg shadow-purple-500/50'
                : 'bg-blue-500 shadow-lg shadow-blue-500/50'
            : 'bg-gray-400'
        }`}></div>
        <span className={`text-xs font-medium uppercase tracking-wide ${
          hasQuota && isActive
            ? title.includes('试用')
              ? 'text-cyan-500'
              : title.includes('奖励')
                ? 'text-purple-500'
                : "text-muted-foreground"
            : "text-muted-foreground"
        }`}>{title}</span>
        {status && status !== 'ACTIVE' && (
          <span className={`text-xs px-2 py-0.5 rounded-md font-medium bg-muted/30 text-muted-foreground`}>
            {status}
          </span>
        )}
      </div>
      <div className={`text-2xl font-semibold text-foreground mb-1`}>
        {hasQuota ? (
          <>{formatUsage(used)} <span className={`text-base text-muted-foreground font-normal`}>/ {formatUsage(quota)}</span></>
        ) : (
          <span className={"text-muted-foreground"}>-</span>
        )}
      </div>
      {expiry && (
        <div className={`text-xs text-muted-foreground mt-2 flex items-center gap-1`}>
          <span>{icon}</span>
          <span>{expiry}</span>
        </div>
      )}
    </div>
  )
})

QuotaCard.displayName = 'QuotaCard'

export { QuotaCard }
export type { QuotaCardProps }
