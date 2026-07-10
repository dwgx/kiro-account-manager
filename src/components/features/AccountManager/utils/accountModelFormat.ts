import { AvailableModel, Account } from '../../../../types/account'

export const formatTokenLimit = (value?: number | null) => {
  if (!value) return '-'
  if (value >= 1000000) return `${(value / 1000000).toFixed(value % 1000000 === 0 ? 0 : 1)}M`
  if (value >= 1000) return `${(value / 1000).toFixed(value % 1000 === 0 ? 0 : 1)}K`
  return String(value)
}

export const formatModelList = (values?: string[] | null) => values?.filter(Boolean).join(', ') || '-'

export const formatEffortLabel = (model: AvailableModel) => {
  const levels = formatModelList(model.effortLevels)
  return model.effortSchemaPath ? `${model.effortSchemaPath}: ${levels}` : levels
}

export const getPrimaryUsage = (account: Account) => {
  const breakdown = account.usageData?.usageBreakdownList?.[0]
  return {
    quota: breakdown?.usageLimit ?? 0,
    used: breakdown?.currentUsage ?? 0,
  }
}
