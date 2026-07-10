import { formatGatewayRequestDuration } from './format'

export const buildGatewayRequestLogSummary = (entries: any) => {
  const logs = Array.isArray(entries) ? entries : []
  const errors = logs.filter(item => item?.outcome === 'error').length
  const streaming = logs.filter(item => item?.outcome === 'stream').length
  const success = logs.filter(item => item?.outcome === 'success').length
  const maxDuration = logs.reduce((max, item) => Math.max(max, Number(item?.durationMs || 0)), 0)
  const latestOccurredAt = logs[0]?.occurredAt || '-'

  // Prompt Caching 统计
  let totalInputTokens = 0
  let totalOutputTokens = 0
  let totalCacheReadTokens = 0
  let totalCacheCreationTokens = 0
  let requestsWithCache = 0

  logs.forEach(item => {
    const inputTokens = Number(item?.inputTokens || 0)
    const outputTokens = Number(item?.outputTokens || 0)
    const cacheReadTokens = Number(item?.cacheReadInputTokens || 0)
    const cacheCreationTokens = Number(item?.cacheCreationInputTokens || 0)

    totalInputTokens += inputTokens
    totalOutputTokens += outputTokens
    totalCacheReadTokens += cacheReadTokens
    totalCacheCreationTokens += cacheCreationTokens

    if (cacheReadTokens > 0 || cacheCreationTokens > 0) {
      requestsWithCache++
    }
  })

  const total = logs.length
  const successRateLabel = total > 0 ? `${((success / total) * 100).toFixed(1)}%` : '0%'
  const errorRateLabel = total > 0 ? `${((errors / total) * 100).toFixed(1)}%` : '0%'

  // 计算缓存命中率
  const cacheHitRate = total > 0
    ? Math.round((requestsWithCache / total) * 100)
    : 0

  // 计算节省成本百分比（缓存读取成本是输入成本的 10%）
  const totalCacheableTokens = totalCacheReadTokens + totalCacheCreationTokens
  const costSavings = totalCacheableTokens > 0
    ? Math.round((totalCacheReadTokens / totalCacheableTokens) * 90)
    : 0

  return {
    total,
    errors,
    streaming,
    success,
    successRateLabel,
    errorRateLabel,
    maxDurationLabel: formatGatewayRequestDuration(maxDuration),
    latestOccurredAt,
    // Prompt Caching 统计
    totalInputTokens,
    totalOutputTokens,
    totalCacheReadTokens,
    totalCacheCreationTokens,
    requestsWithCache,
    cacheHitRate: `${cacheHitRate.toFixed(1)}%`,
    costSavings: `${costSavings.toFixed(2)}`
  }
}
interface MetricEntry {
  label: string
  count: number
  percent: string
}

const buildTopEntries = (values: Record<string, number>, total: number, limit = 5): MetricEntry[] =>
  Object.entries(values)
    .sort((left, right) => {
      if (right[1] !== left[1]) {
        return right[1] - left[1]
      }
      return left[0].localeCompare(right[0], 'zh-CN')
    })
    .slice(0, limit)
    .map(([label, count]) => ({
      label,
      count,
      percent: total > 0 ? `${Math.round((count / total) * 100)}%` : '0%'
    }))

export const buildGatewayMetricsSummary = (entries: any) => {
  const logs = Array.isArray(entries) ? entries : []
  const total = logs.length

  if (total === 0) {
    return {
      total: 0,
      avgDurationLabel: '0 ms',
      successRateLabel: '0%',
      errorRateLabel: '0%',
      uniqueModels: 0,
      uniqueUpstreams: 0,
      topModels: [],
      topUpstreams: [],
      topStatuses: [],
      topEndpoints: [],
      topRegions: []
    }
  }

  const outcomeCounts = { success: 0, stream: 0, error: 0, other: 0 }
  const modelCounts: Record<string, number> = {}
  const upstreamCounts: Record<string, number> = {}
  const statusCounts: Record<string, number> = {}
  const endpointCounts: Record<string, number> = {}
  const regionCounts: Record<string, number> = {}
  let totalDuration = 0

  logs.forEach(item => {
    totalDuration += Number(item?.durationMs || 0)

    const outcome = String(item?.outcome || 'other')
    if (outcomeCounts[outcome] !== undefined) {
      outcomeCounts[outcome] += 1
    } else {
      outcomeCounts.other += 1
    }

    const model = String(item?.model || '未记录模型').trim() || '未记录模型'
    modelCounts[model] = (modelCounts[model] || 0) + 1

    const upstream = String(item?.upstreamSource || '未解析上游来源').trim() || '未解析上游来源'
    upstreamCounts[upstream] = (upstreamCounts[upstream] || 0) + 1

    const status = String(item?.statusCode || 0)
    statusCounts[status] = (statusCounts[status] || 0) + 1

    const endpoint = String(item?.endpoint || '-')
    endpointCounts[endpoint] = (endpointCounts[endpoint] || 0) + 1

    const region = String(item?.region || '-')
    regionCounts[region] = (regionCounts[region] || 0) + 1
  })

  const avgDuration = Math.round(totalDuration / total)

  return {
    total,
    avgDurationLabel: formatGatewayRequestDuration(avgDuration),
    successRateLabel: `${Math.round((outcomeCounts.success / total) * 100)}%`,
    errorRateLabel: `${Math.round((outcomeCounts.error / total) * 100)}%`,
    uniqueModels: Object.keys(modelCounts).length,
    uniqueUpstreams: Object.keys(upstreamCounts).length,
    topModels: buildTopEntries(modelCounts, total),
    topUpstreams: buildTopEntries(upstreamCounts, total),
    topStatuses: buildTopEntries(statusCounts, total),
    topEndpoints: buildTopEntries(endpointCounts, total),
    topRegions: buildTopEntries(regionCounts, total)
  }
}

const stringifyGatewayRequestLog = (entry: any): string => {
  if (!entry || typeof entry !== 'object') {
    return ''
  }

  return [
    entry.endpoint,
    entry.outcome,
    entry.model,
    entry.region,
    entry.clientIp,
    entry.upstreamSource,
    entry.error,
    entry.requestBody,
    entry.responseBody,
    entry.statusCode,
    entry.requestIndex,
    entry.occurredAt,
  ]
    .filter(value => value !== undefined && value !== null)
    .join(' ')
    .toLowerCase()
}

interface FilterOptions {
  outcome?: string
  query?: string
}

export const filterGatewayRequestLogs = (entries: any[], options: FilterOptions = {}): any[] => {
  const { outcome = 'all', query = '' } = options
  const logs = Array.isArray(entries) ? entries : []
  const normalizedOutcome = String(outcome || 'all').trim()
  const normalizedQuery = String(query || '').trim().toLowerCase()

  return logs.filter(entry => {
    if (normalizedOutcome !== 'all' && entry?.outcome !== normalizedOutcome) {
      return false
    }

    if (!normalizedQuery) {
      return true
    }

    return stringifyGatewayRequestLog(entry).includes(normalizedQuery)
  })
}
