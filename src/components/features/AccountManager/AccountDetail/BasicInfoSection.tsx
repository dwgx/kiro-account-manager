import { User, Shield } from 'lucide-react'
import { getAccountDisplayName } from '../../../../utils/accountStats'
import { getProviderDisplayName } from '../../../../utils/accountProvider'
import { Account, UsageBreakdown } from '../../../../types/account'

interface BasicInfoSectionProps {
  account: Account;
  breakdown?: UsageBreakdown;
  t: any;
}

export function BasicInfoSection({ account: currentAccount, breakdown, t }: BasicInfoSectionProps) {
  return (
    <div className="px-6 py-4">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* 基本信息 */}
        <section className="space-y-3">
          <h3 className="flex items-center gap-2 font-bold text-sm text-foreground">
            <User size={16} className="text-primary" />
            {t('detail.basicInfo')}
          </h3>
          <div className="bg-muted/30 border rounded-xl p-4 space-y-4">
            <div className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground">{t('detail.emailAddress')}</label>
              <div className="text-sm font-mono break-all select-all">{currentAccount.email || getAccountDisplayName(currentAccount)}</div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1 min-w-0">
                <label className="text-xs font-medium text-muted-foreground">{t('detail.remarkLabel')}</label>
                <div className="text-sm font-medium truncate">{currentAccount.label || '-'}</div>
              </div>
              <div className="space-y-1">
                <label className="text-xs font-medium text-muted-foreground">Provider</label>
                <div className="text-sm font-medium">{getProviderDisplayName(currentAccount.provider) || '-'}</div>
              </div>
            </div>
            <div className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground">User ID</label>
              <div className="text-xs font-mono break-all bg-background p-2 rounded border select-all">
                {currentAccount.usageData?.userInfo?.userId || '-'}
              </div>
            </div>
          </div>
        </section>
        {/* 订阅详情 */}
        <section className="space-y-3">
          <h3 className="flex items-center gap-2 font-bold text-sm text-foreground">
            <Shield size={16} className="text-primary" />
            订阅详情
          </h3>
          <div className="bg-muted/30 border rounded-xl p-4 text-sm space-y-3">
            <div className="flex justify-between items-center py-1 border-b border-border/50">
              <span className="text-muted-foreground text-xs">Region</span>
              <span className="font-mono text-xs px-1.5 py-0.5 bg-muted rounded-md">us-east-1</span>
            </div>
            <div className="flex justify-between items-center py-1 border-b border-border/50">
              <span className="text-muted-foreground text-xs">Token 到期</span>
              <span className="font-medium text-xs">{currentAccount.expiresAt || '-'}</span>
            </div>
            <div className="flex justify-between items-center py-1 border-b border-border/50">
              <span className="text-muted-foreground text-xs">订阅类型</span>
              <span className="font-mono text-xs">{currentAccount.usageData?.subscriptionInfo?.type || '-'}</span>
            </div>
            <div className="flex justify-between items-center py-1 border-b border-border/50">
              <span className="text-muted-foreground text-xs">超额费率</span>
              <span className="font-mono text-xs">
                {breakdown?.overageRate ? `$${breakdown.overageRate}/${breakdown.unit === 'INVOCATIONS' ? 'Credit' : breakdown.unit}` : '-'}
              </span>
            </div>
            <div className="flex justify-between items-center py-1 border-b border-border/50">
              <span className="text-muted-foreground text-xs">资源类型</span>
              <span className="font-mono text-xs">{breakdown?.resourceType || '-'}</span>
            </div>
            <div className="flex justify-between items-center py-1">
              <span className="text-muted-foreground text-xs">可升级</span>
              <span className={`text-xs font-bold ${currentAccount.usageData?.subscriptionInfo?.upgradeCapability === 'UPGRADE_CAPABLE' ? 'text-green-600' : 'text-muted-foreground'}`}>
                {currentAccount.usageData?.subscriptionInfo?.upgradeCapability === 'UPGRADE_CAPABLE' ? 'YES' : 'NO'}
              </span>
            </div>
          </div>
        </section>
      </div>
    </div>
  )
}
