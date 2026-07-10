import { parseAllowedIps, parseClientApiKeys } from './parsing'
import { redactGatewayApiKey } from './format'

export const buildGatewayStatusSummary = ({ config, status, errorHistory, lastStatusSyncAt }: any) => {
  const mode = config?.accountMode || 'single'
  const strategy = config?.strategy || 'round_robin'
  const listenHost = (status?.running ? status?.host : null) || config?.host || status?.host || '127.0.0.1'
  const listenPort = (status?.running ? status?.port : null) || config?.port || status?.port || 8765
  const errorCount = Array.isArray(errorHistory) ? errorHistory.length : 0
  const errorHits = Array.isArray(errorHistory) ? errorHistory.reduce((sum, item) => sum + Number(item.count || 0), 0) : 0
  return {
    listen: `http://${listenHost}:${listenPort}`,
    requests: String(status?.requestCount || 0),
    region: config?.region || 'us-east-1',
    logLevel: config?.logLevel || 'debug',
    sync: lastStatusSyncAt || '-',
    routing: `${mode} / ${strategy}`,
    exposure: config?.localOnly ? '仅本机' : '允许远程',
    errorCount: errorCount
  }
}
export const buildGatewayRoutingSummary = ({ config, counts, selectedLabels = {} }: any) => {
  const mode = config?.accountMode || 'single'
  const inventorySummary = `账号 ${counts?.accounts || 0} 个 / 分组 ${counts?.groups || 0} 个`

  if (mode === 'single') {
    return {
      modeLabel: '指定单账号',
      modeDescription: '2API会固定使用一个账号，适合调试或绑定到单一租户场景。',
      selectionLabel: '当前账号',
      selectionValue: selectedLabels.single || '未选择账号',
      inventorySummary,
      strategySummary: '固定账号，不参与轮换'
    }
  }

  if (mode === 'group') {
    return {
      modeLabel: '按分组账号池',
      modeDescription: '先锁定账号分组，再按策略和阈值从该分组中挑选可用账号。',
      selectionLabel: '当前分组',
      selectionValue: selectedLabels.group || '未选择分组',
      inventorySummary,
      strategySummary: `策略 ${config?.strategy || 'round_robin'} / 阈值 ${Number(config?.threshold) || 90}%`
    }
  }

  if (mode === 'pool') {
    return {
      modeLabel: '账号管理池',
      modeDescription: '使用所有可用账号，按策略和阈值自动选择，不限制分组。',
      selectionLabel: '账号范围',
      selectionValue: '所有可用账号',
      inventorySummary,
      strategySummary: `策略 ${config?.strategy || 'round_robin'} / 阈值 ${Number(config?.threshold) || 90}%`
    }
  }

  return {
    modeLabel: '按分组账号池',
    modeDescription: '先锁定账号分组，再按策略和阈值从该分组中挑选可用账号。',
    selectionLabel: '当前分组',
    selectionValue: selectedLabels.group || '未选择分组',
    inventorySummary,
    strategySummary: `策略 ${config?.strategy || 'round_robin'} / 阈值 ${Number(config?.threshold) || 90}%`
  }
}

export const buildGatewayActionSummary = ({
  running,
  isDirty,
  hasUnsavedChanges,
  hasRuntimeChanges,
  hasFieldErrors }: any) => {
  const unsavedChanges = hasUnsavedChanges ?? isDirty ?? false
  const runtimeChanges = hasRuntimeChanges ?? (running && unsavedChanges)

  if (hasFieldErrors) {
    return {
      tone: 'red',
      title: '先修正配置错误',
      description: '当前表单存在无效配置，保存、启动和重启都会被拦截，先修正标红字段。'
    }
  }

  if (running && unsavedChanges && runtimeChanges) {
    return {
      tone: 'yellow',
      title: '配置已变更，重启后生效',
      description: '2API仍按已启动时的配置运行。先保存，再执行重启2API，才能让新配置生效。'
    }
  }

  if (running && unsavedChanges) {
    return {
      tone: 'blue',
      title: '当前运行配置尚未保存',
      description: '当前页面配置已经用于运行2API，但还没有写回配置文件；如需保留下次启动沿用，请保存配置。'
    }
  }

  if (running) {
    return {
      tone: 'teal',
      title: '2API运行中',
      description: '当前配置与已保存状态一致；如需中断流量可直接停止2API。'
    }
  }

  if (unsavedChanges) {
    return {
      tone: 'blue',
      title: '可按当前配置直接启动',
      description: '启动2API会使用当前表单里的配置；如果希望下次应用启动也沿用这些设置，先点保存配置。'
    }
  }

  return {
    tone: 'blue',
    title: '2API当前未启动',
    description: '可以直接启动现有配置，或先调整表单后再启动。'
  }
}

export const buildGatewaySecuritySummary = ({ config }: any) => {
  const allowedIpsCount = parseAllowedIps(config?.allowedIpsText).length
  const clientApiKeys = parseClientApiKeys(config?.clientApiKeysText || config?.apiKey)

  return {
    exposureLabel: config?.localOnly ? '仅本机访问' : '允许远程访问',
    allowedIpsCount,
    apiKeyState: clientApiKeys.length
      ? `已配置 ${clientApiKeys.length} 个客户端 Key`
      : '未配置客户端 Key',
    logLevel: config?.logLevel || 'debug'
  }
}

export const buildGatewayIntegrationSummary = ({ baseUrl, apiKey, clientApiKeysText, logDir, errorHistory }: any) => {
  const clientApiKeys = parseClientApiKeys(clientApiKeysText || apiKey)
  const safeKey = redactGatewayApiKey(clientApiKeys[0] || '')
  const errorCount = Array.isArray(errorHistory) ? errorHistory.length : 0
  const errorHits = Array.isArray(errorHistory) ? errorHistory.reduce((sum, item) => sum + Number(item.count || 0), 0) : 0

  return {
    endpointLabel: baseUrl,
    authLabel: clientApiKeys.length > 1 ? `Bearer ${safeKey}（共 ${clientApiKeys.length} 个 Key）` : `Bearer ${safeKey}`,
    logDirState: String(logDir || '').trim() ? '日志目录已定位' : '日志目录未获取',
    errorDigest: `${errorCount} 条错误 / ${errorHits} 次命中`
  }
}
