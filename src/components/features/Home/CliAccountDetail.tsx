import { Badge } from '@/components/ui/badge'
import { InfoRow } from './detailRows'

// CLI 账号详情解析
function CliAccountDetail({ snapshot, cliPath }: { snapshot: any; cliPath: string }) {
  const entries = snapshot?.token_entries || []
  const deviceReg = snapshot?.device_registration

  // 找到主 token 条目
  const mainEntry = entries[0]
  const tokenData = mainEntry?.parsed_token

  if (!tokenData) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm flex-col gap-2">
        <span>无有效 Token</span>
        <span className="text-[10px] font-mono truncate max-w-full">{cliPath}</span>
      </div>
    )
  }

  // 判断认证类型
  // IdC 需按 start_url 细分 BuilderId / Enterprise：BuilderId 的 start_url 固定是
  // view.awsapps.com/start，其余（如企业 SSO 实例 ssoins-xxx / d-xxx）都是 Enterprise。
  // 旧逻辑把所有 OIDC token 一律标成 "IdC (BuilderId)"，导致企业账号被误显示为 BuilderId。
  const isOidc = mainEntry.key?.includes('odic')
  const isSocial = mainEntry.key?.includes('social')
  const startUrl: string = (tokenData.start_url || '').trim().replace(/\/+$/, '')
  const isBuilderId = !startUrl || startUrl === 'https://view.awsapps.com/start'
  const authMethod = isSocial
    ? 'Social'
    : isOidc
      ? `IdC (${isBuilderId ? 'BuilderId' : 'Enterprise'})`
      : 'Unknown'

  // Token 过期判断
  let expiresStr = '-'
  let isExpired = false
  if (tokenData.expires_at) {
    try {
      const expiresDate = new Date(tokenData.expires_at)
      if (!isNaN(expiresDate.getTime())) {
        expiresStr = expiresDate.toLocaleString('zh-CN', {
          year: 'numeric',
          month: '2-digit',
          day: '2-digit',
          hour: '2-digit',
          minute: '2-digit'
        })
        isExpired = expiresDate.getTime() < Date.now()
      } else {
        expiresStr = 'Invalid Date'
      }
    } catch {
      expiresStr = 'Invalid Date'
    }
  }

  // 截断显示
  const truncate = (s: string, len = 16) => s ? (s.length > len ? s.substring(0, len) + '...' : s) : '-'

  return (
    <div className="flex-1 flex flex-col gap-3">
      {/* 状态 */}
      <div className="flex items-center gap-2 bg-muted/30 border border-border rounded-xl p-3">
        <div className="w-9 h-9 rounded-lg flex items-center justify-center bg-gradient-to-br from-emerald-500 to-teal-600 text-white font-bold text-sm shrink-0">
          C
        </div>
        <div className="flex flex-col min-w-0 flex-1">
          <span className="text-sm font-semibold text-foreground">{authMethod}</span>
          <span className="text-[11px] text-muted-foreground font-mono truncate">{mainEntry.key}</span>
        </div>
        <Badge variant="default" className={`shrink-0 text-[10px] px-1.5 py-0 ${isExpired ? 'bg-red-500' : 'bg-green-500'}`}>
          {isExpired ? '已过期' : '有效'}
        </Badge>
      </div>

      {/* Token 信息 */}
      <div className="bg-muted/30 border border-border rounded-xl p-3">
        <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">Token</span>
        <div className="flex flex-col gap-1.5">
          <InfoRow label="Access Token" value={truncate(tokenData.access_token, 20)} mono />
          <InfoRow label="Refresh Token" value={truncate(tokenData.refresh_token, 20)} mono />
          <InfoRow label="过期时间" value={expiresStr} valueClass={isExpired ? 'text-red-500' : 'text-green-500'} />
          <InfoRow label="Region" value={tokenData.region || 'us-east-1'} mono />
          {tokenData.start_url && (
            <InfoRow label="Start URL" value={truncate(tokenData.start_url, 24)} mono />
          )}
          {tokenData.oauth_flow && (
            <InfoRow label="OAuth Flow" value={tokenData.oauth_flow} />
          )}
          {tokenData.scopes && tokenData.scopes.length > 0 && (
            <InfoRow label="Scopes" value={`${tokenData.scopes.length} 个`} />
          )}
        </div>
      </div>

      {/* Device Registration */}
      {deviceReg && (
        <div className="bg-muted/30 border border-border rounded-xl p-3">
          <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider mb-2 block">Device Registration</span>
          <div className="flex flex-col gap-1.5">
            <InfoRow label="Client ID" value={truncate(deviceReg.client_id, 20)} mono />
            <InfoRow label="Client Secret" value={truncate(deviceReg.client_secret, 20)} mono />
            <InfoRow label="Region" value={deviceReg.region || 'us-east-1'} mono />
          </div>
        </div>
      )}

      {/* 数据库路径 */}
      <div className="bg-muted/30 border border-border rounded-xl p-2.5">
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">DB 路径</span>
          <span className="text-[10px] font-mono text-muted-foreground truncate max-w-[180px]" title={cliPath}>
            {cliPath.split(/[/\\]/).slice(-2).join('/')}
          </span>
        </div>
      </div>
    </div>
  )
}

export default CliAccountDetail
