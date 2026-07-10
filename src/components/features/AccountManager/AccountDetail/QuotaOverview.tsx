import { CreditCard, RefreshCw } from 'lucide-react'
import { formatUsage, getAccountDisplayName } from '../../../../utils/accountStats'
import { Account, UsageBreakdown, FreeTrialInfo, Bonus } from '../../../../types/account'
import { QuotaCard } from './QuotaCard'

interface QuotaOverviewProps {
  account: Account;
  form: { quota: number; used: number };
  breakdown?: UsageBreakdown;
  freeTrialInfo?: FreeTrialInfo;
  bonuses: Bonus[];
  totals: {
    totalQuota: number;
    totalUsed: number;
    totalPercent: number;
    freeTrialQuota: number;
    freeTrialUsed: number;
    bonusQuota: number;
    bonusUsed: number;
  };
  colors: any;
  t: any;
  refreshing: boolean;
  onRefresh: () => void;
}

export function QuotaOverview({
  account: currentAccount,
  form,
  breakdown,
  freeTrialInfo,
  bonuses,
  totals,
  colors,
  t,
  refreshing,
  onRefresh
}: QuotaOverviewProps) {
  const {
    totalQuota,
    totalUsed,
    totalPercent,
    freeTrialQuota,
    freeTrialUsed,
    bonusQuota,
    bonusUsed
  } = totals

  return (
    <div className={`border-b border-border px-6 py-4`}>
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <div className={`p-1.5 rounded-lg bg-muted/30`}>
            <CreditCard size={18} className={"text-muted-foreground"} />
          </div>
          <span className={`text-sm font-semibold text-foreground`}>{t('detail.quotaOverview')}</span>
        </div>
        <button
          type="button"
          onClick={onRefresh}
          disabled={refreshing}
          className={`
            p-2 rounded-lg transition-colors duration-200 cursor-pointer focus:outline-none focus:ring-2 focus:ring-blue-500/30
            ${refreshing ? 'bg-blue-500/20' : 'bg-blue-500/20 hover:bg-blue-500/30'}
            disabled:opacity-50 disabled:cursor-not-allowed
          `}
          title={t('detail.syncQuota')}
        >
          <RefreshCw size={15} className={`text-blue-500 ${refreshing ? 'animate-spin' : ''}`} />
        </button>
      </div>

      <div className="mb-5">
        <div className="flex items-baseline justify-between mb-3">
          <div>
            <span className={`text-4xl font-semibold text-foreground`}>{formatUsage(totalUsed)}</span>
            <span className={`text-lg text-muted-foreground ml-2`}>/ {formatUsage(totalQuota)}</span>
          </div>
          <span className={`text-base font-medium px-3 py-1 rounded-lg ${
            totalPercent > 80 ? 'bg-red-500/20 text-red-500'
            : totalPercent > 50 ? 'bg-yellow-500/20 text-yellow-600'
            : 'bg-green-500/20 text-green-600'
          }`}>
            {totalPercent.toFixed(0)}% {t('detail.used')}
          </span>
        </div>
        <div className={`h-4 bg-muted/30 rounded-full overflow-hidden shadow-inner`}>
          <div
            className={`h-full rounded-full transition-all duration-500 shadow-lg ${
              totalPercent > 80 ? 'bg-gradient-to-r from-red-400 to-red-500'
              : totalPercent > 50 ? 'bg-gradient-to-r from-yellow-400 to-orange-500'
              : 'bg-gradient-to-r from-green-400 to-emerald-500'
            }`}
            style={{ width: `${totalPercent}%` }}
          />
        </div>
      </div>
      <div className="grid grid-cols-3 gap-3">
        {/* 主配额卡片 */}
        <QuotaCard
          title={t('detail.mainQuota')}
          used={form.used}
          quota={form.quota}
          icon="🔄"
          expiry={currentAccount.usageData?.nextDateReset ? (() => {
            try {
              const date = new Date(currentAccount.usageData.nextDateReset * 1000)
              return !isNaN(date.getTime()) ? `${date.toLocaleDateString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' })} ${t('detail.reset')}` : null
            } catch {
              return null
            }
          })() : null}
          colors={colors}
          t={t}
        />

        {/* 试用配额卡片 */}
        <QuotaCard
          title={t('detail.freeTrial')}
          used={freeTrialUsed}
          quota={freeTrialQuota}
          status={freeTrialInfo?.freeTrialStatus}
          icon="⏰"
          expiry={freeTrialInfo?.freeTrialExpiry ? (() => {
            try {
              const date = new Date(freeTrialInfo.freeTrialExpiry * 1000)
              return !isNaN(date.getTime()) ? `${date.toLocaleDateString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' })} ${t('detail.expires')}` : null
            } catch {
              return null
            }
          })() : null}
          colors={colors}
          t={t}
        />

        {/* 奖励配额卡片 */}
        <QuotaCard
          title={t('detail.bonusTotal')}
          used={bonusUsed}
          quota={bonusQuota}
          icon="🎁"
          expiry={bonuses.length > 0 ? `${bonuses.length} ${t('detail.bonusCount')}` : null}
          colors={colors}
          t={t}
        />
      </div>
      {/* Bonuses 列表 */}
      {bonuses.length > 0 && (
        <div className="mt-6 pt-5 border-t border-border">
          <div className="flex items-center gap-2 mb-4">
            <span className="text-lg">🎁</span>
            <span className={`text-sm font-medium text-foreground`}>{t('detail.bonusDetails')}</span>
            <span className={`text-xs px-2 py-0.5 rounded-full info-badge font-medium`}>{bonuses.length}</span>
          </div>
          <div className="space-y-3">
            {bonuses.map((bonus, idx) => (
              <div key={idx} className={`flex items-center justify-between p-4 rounded-xl border transition-colors duration-200 hover:shadow-md ${
                bonus.status === 'ACTIVE'
                  ? 'bg-purple-500/10 border-purple-500/30'
                  : `bg-muted/30 border-border`
              }`}>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <span className={`text-sm font-medium text-foreground`}>{bonus.displayName || bonus.bonusCode}</span>
                    <span className={`text-xs px-2 py-0.5 rounded-md font-medium ${
                      bonus.status === 'ACTIVE'
                        ? 'bg-green-500/20 text-green-500'
                        : bonus.status === 'EXHAUSTED'
                          ? `bg-muted/30 text-muted-foreground`
                          : 'bg-yellow-500/20 text-yellow-600'
                    }`}>
                      {bonus.status}
                    </span>
                  </div>
                  <div className={`text-xs text-muted-foreground leading-relaxed`}>
                    {bonus.description && <span>{bonus.description} · </span>}
                    {bonus.redeemedAt && <span>{t('detail.redeemed')}: {(() => {
                      try {
                        const date = new Date(bonus.redeemedAt * 1000)
                        return !isNaN(date.getTime()) ? date.toLocaleDateString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' }) : '-'
                      } catch {
                        return '-'
                      }
                    })()} · </span>}
                    {bonus.expiresAt && <span>{t('detail.expires')}: {(() => {
                      try {
                        const date = new Date(bonus.expiresAt * 1000)
                        return !isNaN(date.getTime()) ? date.toLocaleDateString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit' }) : '-'
                      } catch {
                        return '-'
                      }
                    })()}</span>}
                  </div>
                </div>
                <div className="text-right ml-4 flex-shrink-0">
                  <div className={`text-base font-semibold text-foreground`}>{formatUsage(bonus.currentUsage || 0)} <span className={`text-sm text-muted-foreground font-normal`}>/ {formatUsage(bonus.usageLimit || 0)}</span></div>
                  <div className={`text-xs text-muted-foreground font-mono mt-0.5`}>{bonus.bonusCode}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
      {/* 订阅信息 */}
      <div className="mt-6 pt-5 border-t border-border">
        <div className="flex items-center gap-2 mb-4">
          <span className="text-lg">📋</span>
          <span className={`text-sm font-medium text-foreground`}>订阅信息</span>
        </div>
        <div className="grid grid-cols-2 gap-3">
          <div className={`p-3 rounded-lg bg-muted/30`}>
            <div className={`text-xs text-muted-foreground mb-1`}>{t('detail.userId')}</div>
            <div className={`text-foreground font-mono text-xs truncate`} title={currentAccount.usageData?.userInfo?.userId}>
              {currentAccount.usageData?.userInfo?.userId?.slice(-12) || '-'}
            </div>
          </div>
          <div className={`p-3 rounded-lg bg-muted/30`}>
            <div className={`text-xs text-muted-foreground mb-1`}>{t('detail.email')}</div>
            <div className={`text-foreground text-xs truncate`}>
              {currentAccount.usageData?.userInfo?.email || currentAccount.email || getAccountDisplayName(currentAccount)}
            </div>
          </div>
          <div className={`p-3 rounded-lg bg-muted/30`}>
            <div className={`text-xs text-muted-foreground mb-1`}>{t('detail.subscriptionType')}</div>
            <div className={`text-foreground font-mono text-xs truncate`} title={currentAccount.usageData?.subscriptionInfo?.type}>
              {currentAccount.usageData?.subscriptionInfo?.type || '-'}
            </div>
          </div>
          <div className={`p-3 rounded-lg bg-muted/30`}>
            <div className={`text-xs text-muted-foreground mb-1`}>{t('detail.upgradeable')}</div>
            <div className={"text-foreground"}>
              {currentAccount.usageData?.subscriptionInfo?.upgradeCapability === 'UPGRADE_CAPABLE' ? (
                <span className="text-green-500 font-medium">✓ {t('common.yes')}</span>
              ) : (
                <span className={"text-muted-foreground"}>✗ {t('common.no')}</span>
              )}
            </div>
          </div>
          <div className={`p-3 rounded-lg bg-muted/30`}>
            <div className={`text-xs text-muted-foreground mb-1`}>超额能力</div>
            <div className={"text-foreground"}>
              {currentAccount.usageData?.subscriptionInfo?.overageCapability === 'OVERAGE_CAPABLE' ? (
                <span className="text-green-500 font-medium">✓ 支持</span>
              ) : (
                <span className={"text-muted-foreground"}>✗ 不支持</span>
              )}
            </div>
          </div>
          {currentAccount.usageData?.subscriptionInfo?.overageCapability === 'OVERAGE_CAPABLE' && (
            <div className={`p-3 rounded-lg bg-muted/30`}>
              <div className={`text-xs text-muted-foreground mb-1`}>超额状态</div>
              <div className={"text-foreground"}>
                {currentAccount.usageData?.overageConfiguration?.overageStatus === 'ENABLED' ? (
                  <span className="text-green-500 font-medium">✓ 已开启</span>
                ) : (
                  <span className={"text-muted-foreground"}>✗ 已关闭</span>
                )}
              </div>
            </div>
          )}
          {breakdown?.overageRate != null && (
            <>
              <div className={`p-3 rounded-lg bg-muted/30`}>
                <div className={`text-xs text-muted-foreground mb-1`}>超额费率</div>
                <div className={`text-foreground font-medium`}>
                  {breakdown.currency === 'USD' ? '$' : breakdown.currency}{breakdown.overageRate}/Credit
                </div>
              </div>
              <div className={`p-3 rounded-lg bg-muted/30`}>
                <div className={`text-xs text-muted-foreground mb-1`}>超额上限</div>
                <div className={`text-foreground font-medium`}>
                  {breakdown.currency === 'USD' ? '$' : breakdown.currency}{breakdown.overageCap}
                </div>
              </div>
              <div className={`p-3 rounded-lg bg-muted/30`}>
                <div className={`text-xs text-muted-foreground mb-1`}>当前超额</div>
                <div className={`text-foreground font-bold ${breakdown.currentOverages > 0 ? 'text-orange-500' : ''}`}>
                  {formatUsage(breakdown.currentOverages || 0)}
                </div>
              </div>
              <div className={`p-3 rounded-lg bg-muted/30`}>
                <div className={`text-xs text-muted-foreground mb-1`}>超额费用</div>
                <div className={`text-foreground font-bold ${breakdown.overageCharges > 0 ? 'text-orange-500' : ''}`}>
                  {breakdown.currency === 'USD' ? '$' : breakdown.currency}{breakdown.overageCharges?.toFixed(2) || '0.00'}
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
