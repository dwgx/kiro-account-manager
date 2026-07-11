import { isGitHubProvider, normalizeProviderId } from '../../../../utils/accountProvider'

// external_idp 别名集合，与后端 KiroStudio is_external_idp_credential 口径一致
export function canonicalizeAuthMethod(v?: string | null): string | null {
  if (!v) return null
  const lv = String(v).trim().toLowerCase()
  if (lv === 'builder-id' || lv === 'iam') return 'idc'
  if (lv === 'api_key' || lv === 'apikey') return 'api_key'
  if (['external-idp', 'externalidp', 'external_idp', 'azure', 'azuread', 'azure_ad'].includes(lv)) return 'external_idp'
  return lv // idc/social 等原样(小写)
}

// 字段驼峰/下划线两吃
export function pick(item: any, camel: string, snake: string) {
  return item[camel] ?? item[snake]
}

// external_idp 兜底判定 (KiroStudio 无此逻辑, KAM 自行新增)
export function isExternalIdpItem(item: any): boolean {
  const canon = canonicalizeAuthMethod(pick(item, 'authMethod', 'auth_method'))
  if (canon === 'external_idp') return true
  const tokenEndpoint = pick(item, 'tokenEndpoint', 'token_endpoint')
  const issuerUrl = pick(item, 'issuerUrl', 'issuer_url')
  const host = String(tokenEndpoint || issuerUrl || '').toLowerCase()
  // 有微软端点即判 external_idp (尤其 login.microsoftonline.com)
  if (host.includes('login.microsoftonline.')) return true
  if (tokenEndpoint || issuerUrl) return true
  return false
}

export function validateAccount(item: any, index: number) {
  const errors = []
  const refreshToken = pick(item, 'refreshToken', 'refresh_token')
  if (!refreshToken) {
    errors.push(`第 ${index + 1} 条: 缺少 refreshToken`)
    return { valid: false, errors, type: null }
  }

  // external_idp 优先判定 (在 aor 校验和 social/idc 二分类之前)
  if (isExternalIdpItem(item)) {
    const profileArn = pick(item, 'profileArn', 'profile_arn')
    if (!profileArn) {
      errors.push(`第 ${index + 1} 条: external_idp 账号必须提供 profileArn`)
      return { valid: false, errors, type: null }
    }
    if (!pick(item, 'clientId', 'client_id')) {
      errors.push(`第 ${index + 1} 条: external_idp 账号必须提供 clientId`)
      return { valid: false, errors, type: null }
    }
    // 不校验 aor 前缀 / 不要求 Google/Github provider / profileArn 原样透传
    return { valid: true, errors: [] as string[], type: 'external_idp', inferredProvider: undefined }
  }

  const hasClientCredentials = pick(item, 'clientId', 'client_id') && pick(item, 'clientSecret', 'client_secret')
  const isIdC = hasClientCredentials
  const isSocial = !hasClientCredentials

  // aor 前缀校验仅对 social (external_idp/idc 跳过)
  if (isSocial && !refreshToken.startsWith('aor')) {
    errors.push(`第 ${index + 1} 条: refreshToken 格式无效（应以 aor 开头）`)
    return { valid: false, errors, type: null }
  }

  let provider = item.provider
  if (!provider) {
    if (isSocial) {
      // Social 账号必须明确指定 provider
      errors.push(`第 ${index + 1} 条: Social 账号必须指定 provider (Google/Github)`)
      return { valid: false, errors, type: null }
    } else {
      // IdC 账号：通过 startUrl 判断是 Enterprise 还是 BuilderId
      provider = item.startUrl ? 'Enterprise' : 'BuilderId'
    }
  }

  const normalizedProvider = normalizeProviderId(provider)
  const validProviders = ['Google', 'Github', 'BuilderId', 'Enterprise']
  if (!validProviders.includes(normalizedProvider)) {
    errors.push(`第 ${index + 1} 条: provider 必须是 ${validProviders.join('/')}`)
    return { valid: false, errors, type: null }
  }

  if (isSocial && !(normalizedProvider === 'Google' || isGitHubProvider(normalizedProvider))) {
    errors.push(`第 ${index + 1} 条: Social 账号的 provider 应为 Google/Github`)
    return { valid: false, errors, type: null }
  }

  if (isIdC && !['BuilderId', 'Enterprise'].includes(normalizedProvider)) {
    errors.push(`第 ${index + 1} 条: IdC 账号的 provider 应为 BuilderId/Enterprise`)
    return { valid: false, errors, type: null }
  }

  // Enterprise 账号必须提供 region 和 startUrl
  if (normalizedProvider === 'Enterprise') {
    if (!item.region || !item.region.trim()) {
      errors.push(`第 ${index + 1} 条: Enterprise 账号必须提供 region 字段`)
      return { valid: false, errors, type: null }
    }
    if (!item.startUrl || !item.startUrl.trim()) {
      errors.push(`第 ${index + 1} 条: Enterprise 账号必须提供 startUrl 字段`)
      return { valid: false, errors, type: null }
    }
  }

  return { valid: true, errors: [] as string[], type: isSocial ? 'social' : 'idc', inferredProvider: normalizedProvider }
}
