import { parseAllowedIps, parseClientApiKeys } from './parsing'
import { isValidGatewayHost, isValidAllowlistEntry, ALLOWED_REGIONS } from './validation'

export const createGatewayFieldErrors = (config: any) => {
  const errors: any = {}
  const host = String(config?.host || '').trim()
  const port = Number(config?.port)
  const region = String(config?.region || '').trim()
  const accountMode = String(config?.accountMode || 'single').trim()
  const localOnly = config?.localOnly ?? true
  const allowedIps = parseAllowedIps(config?.allowedIpsText)
  const clientApiKeys = parseClientApiKeys(config?.clientApiKeysText || config?.apiKey)

  if (!host) {
    errors.host = '监听地址不能为空'
  } else if (!isValidGatewayHost(host)) {
    errors.host = '监听地址必须是 localhost、IPv4 或 IPv6 地址'
  }

  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    errors.port = '端口必须在 1-65535 之间'
  }

  if (!region) {
    errors.region = 'region 不能为空'
  } else if (!(ALLOWED_REGIONS as readonly string[]).includes(region)) {
    errors.region = `region 不受支持: ${region}`
  }

  if (!['single', 'group', 'pool'].includes(accountMode)) {
    errors.accountMode = 'accountMode 必须是 single/group/pool'
  } else if (accountMode === 'single' && !String(config?.accountId || '').trim()) {
    errors.accountId = 'single 模式必须选择账号'
  } else if (accountMode === 'group' && !String(config?.groupId || '').trim()) {
    errors.groupId = 'group 模式必须选择分组'
  }

  if (!clientApiKeys.length) {
    errors.clientApiKeysText = '必须至少填写一个客户端 API Key'
  }

  if (!localOnly && !allowedIps.length) {
    errors.allowedIpsText = '允许远程访问时必须至少配置一个白名单来源 IP'
  }

  const invalidAllowlistEntry = allowedIps.find(entry => !isValidAllowlistEntry(entry))
  if (invalidAllowlistEntry) {
    errors.allowedIpsText = `白名单条目无效: ${invalidAllowlistEntry}`
  }

  return errors
}
