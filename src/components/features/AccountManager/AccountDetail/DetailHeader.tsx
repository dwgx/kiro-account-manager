import { Copy, Check, User, X } from 'lucide-react'
import { getAccountDisplayName } from '../../../../utils/accountStats'
import { getProviderDisplayName, isGitHubProvider } from '../../../../utils/accountProvider'
import { Account } from '../../../../types/account'

interface DetailHeaderProps {
  account: Account;
  t: any;
  copied: string | null;
  onCopy: (text: string, field: string) => void;
  onClose: () => void;
}

export function DetailHeader({ account: currentAccount, t, copied, onCopy, onClose }: DetailHeaderProps) {
  return (
    <div className={`sticky top-0 z-20 bg-background/95 backdrop-blur border-b border-border px-6 py-4 rounded-t-2xl`}>
      <div className="text-xs font-medium text-muted-foreground mb-3 uppercase tracking-wider">账号详情</div>
      <div className="flex items-start gap-3">
        {/* 头像图标 */}
        <div className={`
          w-12 h-12 rounded-lg flex items-center justify-center flex-shrink-0 shadow-md
          ${currentAccount.provider === 'Google'
            ? 'bg-gradient-to-br from-red-500 to-orange-500'
            : isGitHubProvider(currentAccount.provider)
              ? 'bg-gradient-to-br from-gray-700 to-gray-900'
              : 'bg-gradient-to-br from-blue-500 to-indigo-600'
          }`}
        >
          <User size={22} className="text-white" strokeWidth={2} />
        </div>

        {/* 账号信息 */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <h2 className={`text-base font-semibold text-foreground truncate`}>
              {currentAccount.email ? currentAccount.email : getAccountDisplayName(currentAccount)}
            </h2>
            <span className={`px-2 py-0.5 rounded-md text-xs font-medium whitespace-nowrap shadow-sm ${
              (currentAccount.usageData?.subscriptionInfo?.subscriptionTitle?.toUpperCase()?.includes('ENTERPRISE'))
                ? 'bg-gradient-to-r from-amber-500 to-orange-500 text-white shadow-amber-500/30'
                : (currentAccount.usageData?.subscriptionInfo?.subscriptionTitle?.includes('PRO+'))
                  ? 'bg-gradient-to-r from-purple-500 to-pink-500 text-white shadow-purple-500/30'
                  : (currentAccount.usageData?.subscriptionInfo?.subscriptionTitle?.includes('PRO'))
                    ? 'bg-gradient-to-r from-blue-500 to-indigo-500 text-white shadow-blue-500/30'
                    : (currentAccount.usageData?.subscriptionInfo?.subscriptionTitle?.toUpperCase()?.includes('KIRO'))
                      ? 'bg-gradient-to-r from-teal-500 to-cyan-500 text-white shadow-teal-500/30'
                      : `bg-muted/30 text-muted-foreground`
            }`}>
              {currentAccount.usageData?.subscriptionInfo?.subscriptionTitle || 'Free'}
            </span>
          </div>

          <div className={`flex items-center gap-2 text-xs text-muted-foreground mb-2`}>
            <span className={`flex items-center gap-1 font-medium ${
              currentAccount.provider === 'Google' ? 'text-red-500'
                : isGitHubProvider(currentAccount.provider) ? "text-foreground"
                : currentAccount.provider === 'BuilderId' ? 'text-orange-500'
                : "text-muted-foreground"
            }`}>
              <div className="w-1 h-1 rounded-full bg-current"></div>
              {getProviderDisplayName(currentAccount.provider) || t('common.unknown')}
            </span>
            <span>·</span>
            <span>{t('detail.addedAt')} {currentAccount.addedAt?.split(' ')[0]}</span>
          </div>

          {/* 机器码 */}
          {currentAccount.machineId && (
            <div className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-muted/30`}>
              <span className={`text-[10px] font-medium text-muted-foreground`}>Machine ID:</span>
              <code className="text-[10px] font-mono text-red-400">
                {currentAccount.machineId}
              </code>
              <button
                type="button"
                onClick={() => onCopy(currentAccount.machineId || '', 'machineId')}
                className={`p-0.5 rounded hover:bg-muted/50 cursor-pointer transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-blue-500/30`}
              >
                {copied === 'machineId' ? <Check size={10} className="text-green-500" /> : <Copy size={10} className={"text-muted-foreground"} />}
              </button>
            </div>
          )}
        </div>
        {/* 关闭按钮 */}
        <button
          onClick={onClose}
          className="p-2 rounded-full hover:bg-muted transition-colors flex-shrink-0"
        >
          <X size={18} className="text-muted-foreground" />
        </button>
      </div>
    </div>
  )
}
