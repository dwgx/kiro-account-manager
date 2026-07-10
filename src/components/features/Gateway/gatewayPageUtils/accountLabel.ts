import { getQuota, getUsed, formatUsage } from '../../../../utils/accountStats'

export const formatGatewayAccountOptionLabel = (account: any): string => {
  const email = String(account?.email || '').trim()
  const userId = String(account?.userId || '').trim()
  const baseLabel = email || userId || '未知账号'

  const quota = getQuota(account)
  const used = getUsed(account)
  const remaining = quota - used
  const breakdown = account?.usageData?.usageBreakdownList?.[0]
  const currentOverages = breakdown?.currentOverages ?? 0
  const overageCap = breakdown?.overageCap ?? 0
  const quotaInfo = currentOverages > 0
    ? `超额 ${formatUsage(currentOverages)}${overageCap > 0 ? '/' + formatUsage(overageCap) : ''}`
    : `剩余 ${formatUsage(Math.max(0, remaining))}/${formatUsage(quota)}`

  const status = String(account?.status || '').trim()
  const isActive = status === 'active' || status === ''
  const statusInfo = !isActive ? ` [${status}]` : ''

  return `${baseLabel} ${quotaInfo}${statusInfo}`
}
