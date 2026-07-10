import { Badge } from '@/components/ui/badge'
import { QuotaRow, InfoRow } from './detailRows'
import { getQuota, getUsed, getSubPlan } from '../../../utils/accountStats'
import { getProviderDisplayName, isGitHubProvider } from '../../../utils/accountProvider'

// 当前账号完整解析卡片
function CurrentAccountDetail({ account, accent, maskEmail, t }: {
  account: any;
  accent: any;
  maskEmail: (s: string) => string;
  t: any;
}) {
  const quota = getQuota(account)
  const used = getUsed(account)
  const remaining = Math.max(0, quota - used)
  const percent = quota > 0 ? Math.round((used / quota) * 100) : 0
  const plan = getSubPlan(account)
  const email = account.usageData?.userInfo?.email || account.email || ''
  const provider = account.provider || ''
  const usageData = account.usageData
  const breakdown = usageData?.usageBreakdownList?.[0]
  const overageConfig = usageData?.overageConfiguration
  const subInfo = usageData?.subscriptionInfo
  const userInfo = usageData?.userInfo
  const nextReset = usageData?.nextDateReset
  const freeTrial = breakdown?.freeTrialInfo
  const bonuses = breakdown?.bonuses || []
  const mainUsed = breakdown?.currentUsage ?? 0
  const mainUsedPrecision = breakdown?.currentUsageWithPrecision ?? mainUsed
  const mainLimit = breakdown?.usageLimit ?? 0
  const mainLimitPrecision = breakdown?.usageLimitWithPrecision ?? mainLimit
  const mainPercent = mainLimit > 0 ? Math.round((mainUsed / mainLimit) * 100) : 0

  // 超额相关字段
  const currentOverages = breakdown?.currentOverages ?? 0
  const currentOveragesPrecision = breakdown?.currentOveragesWithPrecision ?? currentOverages
  const overageCap = breakdown?.overageCap ?? 0
  const overageCapPrecision = breakdown?.overageCapWithPrecision ?? overageCap
  const overageCharges = breakdown?.overageCharges ?? 0
  const overageRate = breakdown?.overageRate ?? 0

  const isOverage = currentOverages > 0
  const overageAmount = used > quota ? used - quota : 0
  const displayName = breakdown?.displayName || 'Credit'
  const displayNamePlural = breakdown?.displayNamePlural || 'Credits'
  const resourceType = breakdown?.resourceType || ''
  const currency = breakdown?.currency || ''
  const unit = breakdown?.unit || ''

  const getBarClass = (pct: number) => {
    if (pct > 100) return 'bg-purple-500'
    if (pct > 80) return 'bg-red-500'
    if (pct > 50) return 'bg-amber-500'
    return 'bg-green-500'
  }

  const getPercentClass = (pct: number) => {
    if (pct > 100) return 'text-purple-500'
    if (pct > 80) return 'text-red-500'
    if (pct > 50) return 'text-amber-500'
    return 'text-green-500'
  }

  // 重置时间
  let resetStr = ''
  let daysUntilReset: number | null = null
  let resetDateStr = ''
  if (nextReset) {
    const resetDate = new Date(typeof nextReset === 'string' ? nextReset : (nextReset < 1e12 ? nextReset * 1000 : nextReset))
    if (!isNaN(resetDate.getTime())) {
      resetDateStr = resetDate.toLocaleDateString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' })
    } else {
      resetDateStr = '-'
    }
    const now = new Date()
    daysUntilReset = Math.max(0, Math.ceil((resetDate.getTime() - now.getTime()) / (1000 * 60 * 60 * 24)))
    resetStr = daysUntilReset === 0 ? '今日重置' : `${daysUntilReset}天后重置`
  }

  // 超额使用百分比（相对于超额上限）
  const overagePercent = overageCap > 0 ? Math.round((currentOverages / overageCap) * 100) : 0

  return (
    <div className="flex-1 flex flex-col gap-3">
      {/* 头部：邮箱 + 计划 + Provider */}
      <div className="flex items-center gap-2 bg-muted/30 border border-border rounded-xl p-3">
        <div className={`w-10 h-10 rounded-lg flex items-center justify-center text-white font-bold text-sm shrink-0 ${
          provider === 'Google' ? 'bg-gradient-to-br from-red-500 to-orange-500' :
          isGitHubProvider(provider) ? 'bg-gradient-to-br from-gray-700 to-gray-900' :
          `bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo}`
        }`}>
          {provider?.[0] || 'K'}
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-sm font-semibold text-foreground truncate">
            {email ? maskEmail(email) : getProviderDisplayName(provider)}
          </span>
          <span className="text-[11px] text-muted-foreground">
            {getProviderDisplayName(provider)}
            {daysUntilReset != null && ` · ${resetStr}`}
          </span>
        </div>
        {plan && (
          <Badge variant="default" className="shrink-0 text-[10px] px-1.5 py-0"
            style={{ background: plan.includes('PRO+') ? 'linear-gradient(to right, rgb(168, 85, 247), rgb(236, 72, 153))' : plan.includes('PRO') ? 'rgb(59, 130, 246)' : undefined }}>
            {plan}
          </Badge>
        )}
      </div>

      {/* 总配额进度 */}
      <div className="bg-muted/30 border border-border rounded-xl p-3">
        <div className="flex items-center justify-between mb-1.5">
          <span className="text-xs font-medium text-foreground">
            本月用量 ({displayNamePlural})
          </span>
          <div className="flex items-center gap-2">
            <span className={`text-sm font-bold font-mono ${getPercentClass(percent)}`}>{percent}%</span>
          </div>
        </div>
        <div className="h-[5px] bg-muted rounded-full overflow-hidden mb-1.5">
          <div className={`h-full rounded-full transition-all duration-500 ${getBarClass(percent)}`} style={{ width: `${Math.min(percent, 100)}%` }} />
        </div>
        <div className="flex items-center justify-between">
          <span className="text-[11px] text-muted-foreground font-mono">
            {isOverage ? mainLimitPrecision : mainUsedPrecision} / {mainLimitPrecision} {displayName}
          </span>
          {isOverage ? (
            <span className="text-[11px] font-semibold text-purple-500">超额 {currentOveragesPrecision}</span>
          ) : (
            <span className={`text-[11px] font-semibold ${getPercentClass(percent)}`}>剩余 {remaining}</span>
          )}
        </div>
      </div>

      {/* 超额详情（仅超额时显示） */}
      {currentOverages > 0 && (
        <div className="bg-purple-500/5 border border-purple-500/20 rounded-xl p-3">
          <span className="text-[10px] font-bold uppercase text-purple-500 tracking-wider mb-2 block">超额详情</span>
          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">超额用量</span>
              <span className="text-[11px] font-mono text-purple-500 font-semibold">{currentOveragesPrecision} / {overageCapPrecision}</span>
            </div>
            {/* 超额进度条 */}
            <div className="h-[3px] bg-purple-500/10 rounded-full overflow-hidden">
              <div className="h-full rounded-full bg-purple-500 transition-all" style={{ width: `${Math.min(overagePercent, 100)}%` }} />
            </div>
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">超额费用</span>
              <span className="text-[11px] font-mono text-purple-500 font-semibold">${overageCharges.toFixed(2)} {currency}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-[11px] text-muted-foreground">费率</span>
              <span className="text-[11px] font-mono text-muted-foreground">${overageRate}/{displayName}</span>
            </div>
          </div>
        </div>
      )}

      {/* 额度明细（基础 + 试用 + 奖励） */}
      {breakdown && (
        <div className="bg-muted/30 border border-border rounded-xl p-3">
          <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">额度明细</span>
          <div className="flex flex-col gap-2">
            {/* 基础配额 */}
            <QuotaRow label="基础" used={mainUsed} limit={mainLimit} percent={mainPercent} color="blue" accent={accent} />

            {/* 试用配额 */}
            {freeTrial && freeTrial.freeTrialStatus === 'ACTIVE' && freeTrial.usageLimit > 0 && (
              <QuotaRow
                label="试用"
                used={freeTrial.currentUsage ?? 0}
                limit={freeTrial.usageLimit}
                percent={freeTrial.usageLimit > 0 ? Math.round((freeTrial.currentUsage ?? 0) / freeTrial.usageLimit * 100) : 0}
                color="purple"
                accent={accent}
              />
            )}

            {/* 奖励配额 */}
            {bonuses.filter((b: any) => {
              const now = Date.now()
              const expiry = b.expiresAt ? b.expiresAt * 1000 : Infinity
              return expiry > now && b.status === 'ACTIVE'
            }).map((bonus: any, idx: number) => (
              <QuotaRow
                key={idx}
                label={bonus.displayName?.substring(0, 4) || `奖励${idx + 1}`}
                used={Math.round(bonus.currentUsage ?? 0)}
                limit={Math.round(bonus.usageLimit ?? 0)}
                percent={bonus.usageLimit > 0 ? Math.round((bonus.currentUsage ?? 0) / bonus.usageLimit * 100) : 0}
                color="amber"
                accent={accent}
                expiry={bonus.expiresAt}
              />
            ))}
          </div>
        </div>
      )}

      {/* 订阅 & 账号信息 两列 */}
      <div className="grid grid-cols-2 gap-2">
        {/* 订阅信息 */}
        {subInfo && (
          <div className="bg-muted/30 border border-border rounded-xl p-3">
            <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">订阅</span>
            <div className="flex flex-col gap-1.5">
              <InfoRow label="类型" value={subInfo.subscriptionTitle || 'Free'} />
              <InfoRow label="计划" value={subInfo.type?.replace('Q_DEVELOPER_STANDALONE_', '') || '-'} mono />
              <InfoRow label="超额能力" value={subInfo.overageCapability === 'OVERAGE_CAPABLE' ? '✓ 支持' : '✗'} valueClass={subInfo.overageCapability === 'OVERAGE_CAPABLE' ? 'text-green-500' : ''} />
              <InfoRow label="升级能力" value={subInfo.upgradeCapability === 'UPGRADE_CAPABLE' ? '✓ 可升级' : '✗'} valueClass={subInfo.upgradeCapability === 'UPGRADE_CAPABLE' ? 'text-green-500' : ''} />
              {overageConfig && (
                <InfoRow label="超额开关" value={overageConfig.overageStatus === 'ENABLED' ? '⚡ 已开启' : '已关闭'} valueClass={overageConfig.overageStatus === 'ENABLED' ? 'text-purple-500 font-semibold' : ''} />
              )}
              {subInfo.subscriptionManagementTarget && (
                <InfoRow label="管理" value={subInfo.subscriptionManagementTarget} mono />
              )}
            </div>
          </div>
        )}

        {/* 账号 & 资源信息 */}
        <div className="bg-muted/30 border border-border rounded-xl p-3">
          <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">账号 & 资源</span>
          <div className="flex flex-col gap-1.5">
            <InfoRow label="IDP" value={getProviderDisplayName(provider) || '-'} />
            <InfoRow label="重置日" value={resetDateStr || '-'} />
            {userInfo?.userId && (
              <InfoRow label="用户ID" value={userInfo.userId.split('.').pop()?.substring(0, 12) || '-'} mono />
            )}
            {resourceType && (
              <InfoRow label="资源类型" value={resourceType} mono />
            )}
            {currency && (
              <InfoRow label="货币" value={currency} />
            )}
            {unit && (
              <InfoRow label="计量单位" value={unit === 'INVOCATIONS' ? '调用次数' : unit} />
            )}
            {overageCap > 0 && (
              <InfoRow label="超额上限" value={`${overageCapPrecision}`} />
            )}
            {overageRate > 0 && (
              <InfoRow label="超额费率" value={`$${overageRate}/${displayName}`} mono />
            )}
          </div>
        </div>
      </div>

      {/* IDE Token 路径 */}
      <div className="bg-muted/30 border border-border rounded-xl p-2.5">
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">Token 路径</span>
          <span className="text-[10px] font-mono text-muted-foreground truncate max-w-[220px]" title="~/.aws/sso/cache/">
            .aws/sso/cache/
          </span>
        </div>
      </div>
    </div>
  )
}

export default CurrentAccountDetail
