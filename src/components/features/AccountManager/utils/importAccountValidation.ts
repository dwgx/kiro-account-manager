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

  // 企业号的 start_url 藏在 clientSecret 的 JWT payload（base64 编码，明文搜不到
  // "initiateLoginUri"），且 region 常在 authRegion —— 前端无法可靠区分 BuilderId/Enterprise。
  // 这里只做初判（有顶层 startUrl 才敢定 Enterprise），真正的判定交给后端：add_account_by_idc
  // 会用 extract_start_url_from_client_secret 解码 JWT，提取到企业 start_url 就自动按企业号处理。
  const clientSecret = pick(item, 'clientSecret', 'client_secret')

  let provider = item.provider
  if (!provider) {
    if (isSocial) {
      // Social 账号必须明确指定 provider
      errors.push(`第 ${index + 1} 条: Social 账号必须指定 provider (Google/Github)`)
      return { valid: false, errors, type: null }
    } else {
      // 有顶层 startUrl → Enterprise；否则先按 BuilderId 传给后端，由后端解 JWT 纠正
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

  // Enterprise 账号需要 region 和 startUrl，但真实 Kiro 企业号这两项常不在 JSON 顶层：
  // region 在 authRegion、startUrl 在 clientSecret 的 JWT 里（后端会解码提取）。
  // 因此放宽：region 接受 authRegion 兜底；只要有 clientSecret（后端能从中提 startUrl）
  // 就不强制顶层 startUrl。
  if (normalizedProvider === 'Enterprise') {
    const hasRegion = (item.region && item.region.trim()) || (item.authRegion && String(item.authRegion).trim()) || (item.auth_region && String(item.auth_region).trim())
    if (!hasRegion) {
      errors.push(`第 ${index + 1} 条: Enterprise 账号必须提供 region 或 authRegion 字段`)
      return { valid: false, errors, type: null }
    }
    const hasStartUrl = (item.startUrl && item.startUrl.trim()) || (typeof clientSecret === 'string' && clientSecret.length > 0)
    if (!hasStartUrl) {
      errors.push(`第 ${index + 1} 条: Enterprise 账号必须提供 startUrl 字段`)
      return { valid: false, errors, type: null }
    }
  }

  return { valid: true, errors: [] as string[], type: isSocial ? 'social' : 'idc', inferredProvider: normalizedProvider }
}
