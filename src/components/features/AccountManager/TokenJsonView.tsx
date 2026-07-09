// Token 凭证 JSON 视图组件
import { useState, useRef, useEffect, useMemo } from 'react'
import { Copy, Check, ChevronDown, Key, Eye, EyeOff } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { getThemeAccent } from '../KiroConfig/themeAccent'

// H3：敏感字段名(大小写不敏感,兼容前端 camelCase 与后端 snake_case)。
// 默认对这些字段脱敏,避免完整凭据被动明文进入 DOM / 被一键复制。
const SENSITIVE_KEYS = new Set([
  'accesstoken', 'access_token',
  'refreshtoken', 'refresh_token',
  'idtoken', 'id_token',
  'clientsecret', 'client_secret',
  'password',
])

function isSensitiveKey(key: string): boolean {
  return SENSITIVE_KEYS.has(key.toLowerCase())
}

// 把一个敏感字符串脱敏成"前 6 位 + ••••"（空值返回原值）
function maskSecret(value: string): string {
  if (!value) return value
  if (value.length <= 6) return '••••••'
  return `${value.slice(0, 6)}••••••`
}

// 构建凭证 JSON 对象（直接使用整个账号对象）
function buildCredentialsJson(account) {
  // 直接返回整个账号对象，让后端的序列化逻辑处理
  return account
}

// 生成用于展示/复制的对象:reveal=false 时把顶层敏感字段脱敏。
// 只处理顶层字符串字段(账号凭据都在顶层),嵌套对象原样保留。
function applyMask(account, reveal: boolean) {
  if (reveal || !account || typeof account !== 'object') return account
  const out: any = Array.isArray(account) ? [...account] : { ...account }
  for (const [key, value] of Object.entries(out)) {
    if (isSensitiveKey(key) && typeof value === 'string') {
      out[key] = maskSecret(value)
    }
  }
  return out
}

// 可折叠的字符串值
function CollapsibleValue({ value, colors, threshold = 50 }) {
  const [expanded, setExpanded] = useState(false)
  const isLong = value.length > threshold
  
  if (!isLong) {
    return <span className="text-emerald-500 font-medium">"{value}"</span>
  }
  
  const displayValue = expanded ? value : `${value.slice(0, threshold)}...`
  
  return (
    <span className="inline">
      <span className="text-emerald-500 font-medium">"{displayValue}"</span>
      <button
        type="button"
        onClick={(e) => { e.stopPropagation(); setExpanded(!expanded) }}
        className={`
          ml-2 text-xs px-2 py-0.5 rounded-md 
          bg-muted/30 text-muted-foreground hover:bg-muted/50
          transition-all duration-200 font-medium
        `}
      >
        {expanded ? '收起' : `展开 +${value.length - threshold}`}
      </button>
    </span>
  )
}

// JSON 渲染（带折叠，支持嵌套对象和数组）
function JsonRenderer({ json, colors, accent, indent = 0 }) {
  const entries = Object.entries(json).filter(([_, value]) => value !== undefined)
  const pad = '  '.repeat(indent)
  const padInner = '  '.repeat(indent + 1)

  return (
    <div className="text-sm font-mono leading-relaxed">
      <span className={"text-muted-foreground"}>{'{'}</span>
      {entries.map(([key, value], i) => (
        <div key={key} className="py-0.5">
          <span className={"text-muted-foreground"}>{padInner}</span>
          <span className={`${accent.text} font-semibold`}>"{key}"</span>
          <span className={"text-muted-foreground"}>: </span>
          {typeof value === 'string' ? (
            <CollapsibleValue value={value} colors={colors} />
          ) : value === null || value === undefined ? (
            <span className="text-orange-500 font-medium">null</span>
          ) : typeof value === 'boolean' ? (
            <span className={`${accent.text} font-medium`}>{String(value)}</span>
          ) : typeof value === 'number' ? (
            <span className="text-amber-500 font-medium">{value}</span>
          ) : Array.isArray(value) ? (
            <span className="text-emerald-500">[{value.length > 0 ? '...' : ''}]</span>
          ) : typeof value === 'object' ? (
            <span className="text-emerald-500">{'{...}'}</span>
          ) : (
            <span className="text-emerald-500">{JSON.stringify(value)}</span>
          )}
          {i < entries.length - 1 && <span className={"text-muted-foreground"}>,</span>}
        </div>
      ))}
      <span className={"text-muted-foreground"}>{pad}{'}'}</span>
    </div>
  )
}

// Token JSON 视图（只读）
export function TokenJsonView({ account, defaultExpanded = false }) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const colors = useMemo(() => ({
    inputFocus: 'focus:ring-primary/20 focus:border-primary'
  }), [])
  const [expanded, setExpanded] = useState(defaultExpanded)
  const [copied, setCopied] = useState(false)
  // H3：默认脱敏,用户显式点击"显示"才展示/复制完整凭据。
  const [reveal, setReveal] = useState(false)
  const copiedTimerRef = useRef(null)

  const credentialsJson = useMemo(() => buildCredentialsJson(account), [account])
  // 展示与复制都基于脱敏后的视图:reveal=false 时敏感字段被掩码,复制出去的也是掩码值。
  const maskedJson = useMemo(() => applyMask(credentialsJson, reveal), [credentialsJson, reveal])
  const jsonStr = useMemo(() => JSON.stringify(maskedJson, null, 2), [maskedJson])

  useEffect(() => () => copiedTimerRef.current && clearTimeout(copiedTimerRef.current), [])
  
  const handleCopy = () => {
    navigator.clipboard.writeText(jsonStr).catch(e => console.error('Copy failed:', e))
    setCopied(true)
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current)
    copiedTimerRef.current = setTimeout(() => setCopied(false), 1500)
  }
  
  return (
    <div className={`border-b border-border`} style={{ margin: 0 }}>
      <div 
        className={`flex items-center justify-between cursor-pointer hover:bg-muted/30 transition-all duration-200 px-6 py-3`}
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center gap-2">
          <Key size={16} className={"text-muted-foreground"} />
          <span className={`text-sm font-medium text-foreground`}>{t('detail.tokenCredentials') || 'Token 凭证'}</span>
          <span className={`text-xs px-1.5 py-0.5 rounded bg-muted/50 text-muted-foreground font-mono`}>
            {Object.keys(credentialsJson).length} 字段
          </span>
        </div>
        <div className="flex items-center gap-2">
          {/* H3：显示/隐藏敏感凭据切换,默认隐藏 */}
          <button
            type="button"
            onClick={(e) => { e.stopPropagation(); setReveal(r => !r) }}
            title={reveal ? '隐藏敏感凭据' : '显示敏感凭据'}
            className="text-xs text-muted-foreground hover:text-foreground px-2 py-1 rounded hover:bg-muted/50 transition-colors"
          >
            {reveal ? <EyeOff size={13} /> : <Eye size={13} />}
          </button>
          <button
            type="button"
            onClick={(e) => { e.stopPropagation(); handleCopy() }}
            title={reveal ? '复制(含完整凭据)' : '复制(敏感字段已脱敏)'}
            className="text-xs text-muted-foreground hover:text-foreground px-2 py-1 rounded hover:bg-muted/50 transition-colors"
          >
            {copied ? <Check size={13} className="text-green-500" /> : <Copy size={13} />}
          </button>
          <ChevronDown size={14} className={`text-muted-foreground transition-transform duration-200 ${expanded ? '' : '-rotate-90'}`} />
        </div>
      </div>

      {expanded && (
        <div className="px-6 pb-4">
          <div className="p-3 rounded-lg bg-muted/20 border border-border max-h-64 overflow-auto font-mono text-xs leading-relaxed">
            <JsonRenderer json={maskedJson} colors={colors} accent={accent} />
          </div>
        </div>
      )}
    </div>
  )
}

export default TokenJsonView
