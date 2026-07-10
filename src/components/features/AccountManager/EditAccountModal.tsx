import { useState, useEffect, useMemo } from 'react'
import { createPortal } from 'react-dom'
import { invoke } from '@tauri-apps/api/core'
import { Check, Folder, Plus, X, RefreshCw, Loader2, Network, PlugZap, Tag } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { setAccountTags, setAccountGroup, getGroups, addGroup } from '../../../api/groupTag'
import { getAccountDisplayName } from '../../../utils/accountStats'
import { TagSelector } from './GroupTagManager'
import { AccountStatusCard } from './AccountStatusCard'
import { AccountCredentialFields } from './AccountCredentialFields'
import {
  normalizeProxyConfig,
  normalizeProxyForSave,
  parseProxyUrl,
} from './utils/proxyConfig'
import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody,
  DialogFooter} from '../../shared/dialog'
import { Button } from '../../shared/button'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import { Account, AccountProxyConfig, AccountProxyProtocol, GroupDefinition } from '../../../types/account'

const PRESET_COLORS = [
  '#3b82f6', '#10b981', '#f59e0b', '#ef4444',
  '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'
]

interface GroupSelectorProps {
  groups: GroupDefinition[];
  value: string;
  onChange: (value: string) => void;
  onGroupsChange: (groups: GroupDefinition[]) => void;
}

function GroupSelector({ groups, value, onChange, onGroupsChange }: GroupSelectorProps) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const colors = useMemo(() => ({
    inputFocus: 'focus:ring-primary/20 focus:border-primary'
  }), [])
  const inputClass = `flex-1 px-4 py-2.5 border rounded-xl text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`
  const buttonClass = `p-2.5 ${accent.solidBg} text-white rounded-xl ${accent.solidHoverBg} disabled:opacity-50 cursor-pointer`
  const ghostButtonClass = `p-2.5 rounded-xl hover:bg-muted/50 cursor-pointer`

  const [newGroupName, setNewGroupName] = useState('')
  const [showInput, setShowInput] = useState(false)

  const handleAddGroup = async () => {
    const trimmed = newGroupName.trim().slice(0, 20)
    if (!trimmed) return
    if (groups.some(g => g.name === trimmed)) {
      setNewGroupName('')
      return
    }
    const color = PRESET_COLORS[Math.floor(Math.random() * PRESET_COLORS.length)]
    try {
      const newGroup = await addGroup(trimmed, color) as GroupDefinition
      onGroupsChange([...groups, newGroup])
      onChange(newGroup.id)
      setNewGroupName('')
      setShowInput(false)
    } catch (e) {
      console.error('创建分组失败:', e)
    }
  }

  if (showInput) {
    return (
      <div className="flex gap-2">
        <input
          type="text"
          value={newGroupName}
          onChange={(e) => setNewGroupName(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleAddGroup()}
          placeholder={t('groups.newGroupPlaceholder') || '输入新分组名...'}
          className={inputClass}
        />
        <button
          onClick={handleAddGroup}
          disabled={!newGroupName.trim()}
          className={buttonClass}
        >
          <Check size={16} />
        </button>
        <button
          onClick={() => { setShowInput(false); setNewGroupName('') }}
          className={ghostButtonClass}
        >
          <X size={16} />
        </button>
      </div>
    )
  }

  return (
    <div className="flex gap-2">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={inputClass}
      >
        <option value="">{t('groups.noGroup') || '无分组'}</option>
        {groups.map(g => (
          <option key={g.id} value={g.id}>{g.name}</option>
        ))}
      </select>
      <button
        onClick={() => setShowInput(true)}
        className={buttonClass}
      >
        <Plus size={16} />
      </button>
    </div>
  )
}

interface EditAccountModalProps {
  account: Account;
  onClose: () => void;
  onSuccess?: (account: Account) => void;
}

interface VerifyAccountResponse {
  usageData: any;
  accessToken: string;
  refreshToken: string;
}

interface AccountProxyTestResult {
  success: boolean;
  latencyMs: number;
  status?: number | null;
  message: string;
}

function EditAccountModal({ account, onClose, onSuccess }: EditAccountModalProps) {
  const { t, theme } = useApp()
  const { showError, showSuccess } = useDialog()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const colors = useMemo(() => ({
    inputFocus: 'focus:ring-primary/20 focus:border-primary'
  }), [])

  const isIdCAccount = account.provider === 'BuilderId' || account.provider === 'Enterprise'

  const [form, setForm] = useState({
    label: account.label || '',
    accessToken: account.accessToken || '',
    refreshToken: account.refreshToken || '',
    clientId: account.clientId || '',
    clientSecret: account.clientSecret || '',
    machineId: account.machineId || '',
    addedAt: account.addedAt || '',
    expiresAt: account.expiresAt || ''})

  const [selectedTagIds, setSelectedTagIds] = useState((account.tagLinks || []).map(link => link.tagId))
  const [selectedGroupId, setSelectedGroupId] = useState(account.groupId || '')
  const [groups, setGroups] = useState<GroupDefinition[]>([])
  const [saving, setSaving] = useState(false)
  const [verifying, setVerifying] = useState(false)
  const [testingProxy, setTestingProxy] = useState(false)
  const [proxyQuickInput, setProxyQuickInput] = useState('')
  const [proxyQuickInputError, setProxyQuickInputError] = useState('')
  const [proxyConfig, setProxyConfig] = useState<AccountProxyConfig>(() => normalizeProxyConfig(account.proxyConfig))
  const [copiedField, setCopiedField] = useState<string | null>(null)

  const formatResetTime = (value?: string | number | null) => {
    if (!value) return undefined
    try {
      const timestamp = typeof value === 'number' && value < 1e12 ? value * 1000 : value
      const date = new Date(timestamp)
      return !isNaN(date.getTime()) ? date.toLocaleString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit' }) : undefined
    } catch {
      return undefined
    }
  }

  // 账号信息状态（验证后更新）
  const [accountInfo, setAccountInfo] = useState<{
    email: string;
    subscriptionType: string;
    usage: { current: number; limit: number };
    resetTime?: string;
  } | null>(null)

  useEffect(() => {
    getGroups().then(setGroups).catch(() => {})

    // 初始化账号信息
    if (account.usageData) {
      const usageData = account.usageData
      const userInfo = usageData.userInfo || {}
      const subscriptionInfo = usageData.subscriptionInfo
      const breakdown = usageData.usageBreakdownList?.[0]
      const nextReset = usageData.nextDateReset

      setAccountInfo({
        email: account.email || userInfo.email || '',
        subscriptionType: subscriptionInfo?.subscriptionTitle || subscriptionInfo?.type || 'Free',
        usage: {
          current: breakdown?.currentUsage ?? 0,
          limit: breakdown?.usageLimit ?? 0
        },
        resetTime: formatResetTime(nextReset)
      })
    }
  }, [account])

  const handleCopy = async (text: string, field: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setCopiedField(field)
      setTimeout(() => setCopiedField(null), 2000)
    } catch (e) {
      console.error('复制失败:', e)
    }
  }

  const updateProxyConfig = (patch: Partial<AccountProxyConfig>) => {
    setProxyConfig(prev => ({ ...prev, ...patch }))
  }

  const handleProxyQuickInputChange = (value: string) => {
    setProxyQuickInput(value)
    if (!value.trim()) {
      setProxyQuickInputError('')
      return
    }

    try {
      setProxyConfig(parseProxyUrl(value))
      setProxyQuickInputError('')
    } catch {
      setProxyQuickInputError(t('editAccount.proxyQuickInputInvalid'))
    }
  }

  const validateProxyConfig = async (config: AccountProxyConfig) => {
    if (!config.enabled) return true
    if (!config.host.trim()) {
      await showError(t('editAccount.proxyInvalid'), t('editAccount.proxyHostRequired'))
      return false
    }
    if (!Number.isInteger(config.port) || config.port < 1 || config.port > 65535) {
      await showError(t('editAccount.proxyInvalid'), t('editAccount.proxyPortRequired'))
      return false
    }
    if (config.password && !config.username?.trim()) {
      await showError(t('editAccount.proxyInvalid'), t('editAccount.proxyUsernameRequired'))
      return false
    }
    return true
  }

  const handlePasteProxyUrl = async () => {
    if (!proxyQuickInput.trim()) {
      await showError(t('editAccount.proxyInvalid'), t('editAccount.proxyQuickInputInvalid'))
      return
    }

    try {
      setProxyConfig(parseProxyUrl(proxyQuickInput))
    } catch {
      await showError(t('editAccount.proxyInvalid'), t('editAccount.proxyQuickInputInvalid'))
    }
  }

  const handleTestProxy = async () => {
    const config = normalizeProxyForSave({ ...proxyConfig, enabled: true })
    if (!(await validateProxyConfig(config))) return

    setTestingProxy(true)
    try {
      const result = await invoke<AccountProxyTestResult>('test_account_proxy', {
        proxyConfig: config
      })
      if (result.success) {
        await showSuccess(t('editAccount.proxyTestSuccess'), result.message)
      } else {
        await showError(t('editAccount.proxyTestFailed'), result.message)
      }
    } catch (error) {
      await showError(t('editAccount.proxyTestFailed'), String(error))
    } finally {
      setTestingProxy(false)
    }
  }

  const handleVerifyAndRefresh = async () => {
    if (!form.refreshToken) {
      await showError(t('editAccount.verifyFailed'), t('editAccount.pleaseFillRefreshToken'))
      return
    }
    if (isIdCAccount && (!form.clientId || !form.clientSecret)) {
      await showError(t('editAccount.verifyFailed'), t('editAccount.pleaseFillClientIdAndSecret'))
      return
    }

    setVerifying(true)
    try {
      const result = await invoke<VerifyAccountResponse>('verify_account', {
        params: {
          accessToken: form.accessToken,
          refreshToken: form.refreshToken,
          provider: account.provider,
          clientId: isIdCAccount ? form.clientId : null,
          clientSecret: isIdCAccount ? form.clientSecret : null,
          region: null
        }
      })

      // 更新表单中的 token
      setForm(prev => ({
        ...prev,
        accessToken: result.accessToken,
        refreshToken: result.refreshToken
      }))

      // 更新账号信息显示
      const usageData = result.usageData
      const userInfo = usageData.userInfo || {}
      const subscriptionInfo = usageData.subscriptionInfo
      const verifyBreakdown = usageData.usageBreakdownList?.[0]
      const verifyNextReset = usageData.nextDateReset

      setAccountInfo({
        email: userInfo.email || '',
        subscriptionType: subscriptionInfo?.subscriptionTitle || subscriptionInfo?.type || 'Free',
        usage: {
          current: verifyBreakdown?.currentUsage ?? 0,
          limit: verifyBreakdown?.usageLimit ?? 0
        },
        resetTime: formatResetTime(verifyNextReset)
      })
    } catch (e) {
      await showError(t('editAccount.verifyFailed'), String(e))
    } finally {
      setVerifying(false)
    }
  }

  const handleSave = async () => {
    const nextProxyConfig = normalizeProxyForSave(proxyConfig)
    if (!(await validateProxyConfig(nextProxyConfig))) return

    setSaving(true)
    try {
      const params: any = {
        id: account.id,
        label: form.label || null,
        accessToken: form.accessToken || null,
        refreshToken: form.refreshToken || null,
        machineId: form.machineId || null,
        addedAt: form.addedAt || null,
        expiresAt: form.expiresAt,
        proxyConfig: nextProxyConfig}
      if (isIdCAccount) {
        params.clientId = form.clientId || null
        params.clientSecret = form.clientSecret || null
      }
      const updatedAccount = await invoke<Account>('update_account', { params })
      await setAccountGroup(account.id, selectedGroupId || null)
      await setAccountTags(account.id, selectedTagIds)
      onSuccess?.(updatedAccount)
      onClose()
    } catch (e) {
      await showError(t('editAccount.saveFailed'), String(e))
    } finally {
      setSaving(false)
    }
  }

  const dialogContent = (
    <DialogRoot open={true} onOpenChange={(open) => !open && onClose()}>
      <div className="fixed inset-0 z-50 flex items-center justify-center">
        <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />

        <div className="relative w-full max-w-4xl max-h-[90vh] overflow-hidden bg-background rounded-2xl shadow-2xl z-10 animate-in zoom-in-95 duration-200 flex flex-col">
          {/* Sticky Header */}
          <div className="sticky top-0 bg-background/95 backdrop-blur-sm z-20 border-b border-border">
            <DialogHeader icon={Folder} iconColor={accent.text} iconBg={accent.iconBadgeBg}>
              <DialogTitle>{t('editAccount.title')}</DialogTitle>
              <DialogDescription>{getAccountDisplayName(account)}</DialogDescription>
            </DialogHeader>
            <button
              onClick={onClose}
              className="absolute top-4 right-4 p-2 rounded-lg hover:bg-muted/50 transition-colors cursor-pointer"
              aria-label="Close"
            >
              <X size={20} className="text-muted-foreground" />
            </button>
          </div>

          {/* Scrollable Body */}
          <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* 当前账号状态 */}
          {accountInfo && <AccountStatusCard accountInfo={accountInfo} accent={accent} />}

          <AccountCredentialFields
            form={form}
            onFormChange={(patch) => setForm(prev => ({ ...prev, ...patch }))}
            handleCopy={handleCopy}
            copiedField={copiedField}
            isIdCAccount={isIdCAccount}
            colors={colors}
            t={t}
          />

          {/* 验证并刷新按钮 */}
          <Button
            variant="secondary"
            className="w-full h-10 rounded-xl font-medium"
            onClick={handleVerifyAndRefresh}
            disabled={verifying || !form.refreshToken || (isIdCAccount && (!form.clientId || !form.clientSecret))}
          >
            {verifying ? (
              <>
                <Loader2 size={16} className="mr-2 animate-spin" />
                验证中...
              </>
            ) : (
              <>
                <RefreshCw size={16} className="mr-2" />
                验证并刷新凭证信息
              </>
            )}
          </Button>

          {/* 账号代理 */}
          <div className="rounded-xl border border-border bg-muted/20 p-4 space-y-4">
            <div className="flex items-start justify-between gap-4">
              <div className="flex items-start gap-3">
                <div className={`mt-0.5 rounded-lg p-2 ${accent.iconBadgeBg}`}>
                  <Network size={18} className={accent.text} />
                </div>
                <div>
                  <div className="text-sm font-semibold text-foreground">{t('editAccount.proxyTitle')}</div>
                  <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                    {t('editAccount.proxyDescription')}
                  </p>
                </div>
              </div>
              <label className="flex min-h-11 cursor-pointer items-center gap-2 rounded-lg px-2 text-sm font-medium text-foreground">
                <input
                  type="checkbox"
                  checked={proxyConfig.enabled}
                  onChange={(event) => updateProxyConfig({ enabled: event.target.checked })}
                  className="h-4 w-4 accent-primary cursor-pointer"
                />
                {t('editAccount.proxyEnabled')}
              </label>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-[1fr_auto] gap-3">
              <div>
                <label className="block text-sm font-medium text-foreground mb-2">{t('editAccount.proxyQuickInput')}</label>
                <input
                  type="text"
                  value={proxyQuickInput}
                  onChange={(event) => handleProxyQuickInputChange(event.target.value)}
                  placeholder={t('editAccount.proxyQuickInputPlaceholder')}
                  className={`w-full px-4 py-3 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none font-mono`}
                />
                <div className={`mt-1 text-[11px] ${proxyQuickInputError ? 'text-destructive' : 'text-muted-foreground'}`}>
                  {proxyQuickInputError || t('editAccount.proxyQuickInputHint')}
                </div>
              </div>
              <div className="flex items-end">
                <Button
                  variant="secondary"
                  className="h-11 rounded-xl font-medium"
                  onClick={handlePasteProxyUrl}
                >
                  {t('editAccount.proxyApplyQuickInput')}
                </Button>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div>
                <label className="block text-sm font-medium text-foreground mb-2">{t('editAccount.proxyProtocol')}</label>
                <select
                  value={proxyConfig.protocol}
                  onChange={(event) => updateProxyConfig({ protocol: event.target.value as AccountProxyProtocol })}
                  className={`w-full px-4 py-3 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
                >
                  <option value="http">HTTP / HTTPS</option>
                  <option value="socks5">SOCKS5</option>
                </select>
              </div>
              <div className="md:col-span-2">
                <label className="block text-sm font-medium text-foreground mb-2">{t('editAccount.proxyHost')}</label>
                <input
                  type="text"
                  value={proxyConfig.host}
                  onChange={(event) => updateProxyConfig({ host: event.target.value })}
                  placeholder="127.0.0.1"
                  className={`w-full px-4 py-3 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-foreground mb-2">{t('editAccount.proxyPort')}</label>
                <input
                  type="number"
                  min={0}
                  max={65535}
                  value={proxyConfig.port || ''}
                  onChange={(event) => updateProxyConfig({ port: Number(event.target.value || 0) })}
                  placeholder="7890"
                  className={`w-full px-4 py-3 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
                />
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-foreground mb-2">
                  {t('editAccount.proxyUsername')} ({t('editAccount.proxyOptional')})
                </label>
                <input
                  type="text"
                  value={proxyConfig.username || ''}
                  onChange={(event) => updateProxyConfig({ username: event.target.value || null })}
                  placeholder="username"
                  className={`w-full px-4 py-3 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-foreground mb-2">
                  {t('editAccount.proxyPassword')} ({t('editAccount.proxyOptional')})
                </label>
                <input
                  type="password"
                  value={proxyConfig.password || ''}
                  onChange={(event) => updateProxyConfig({ password: event.target.value || null })}
                  placeholder="password"
                  className={`w-full px-4 py-3 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
                  autoComplete="off"
                />
              </div>
            </div>

            <div className="flex justify-end">
              <Button
                variant="secondary"
                className="h-10 rounded-xl font-medium"
                onClick={handleTestProxy}
                disabled={testingProxy || !proxyConfig.host || !proxyConfig.port}
              >
                {testingProxy ? (
                  <>
                    <Loader2 size={16} className="mr-2 animate-spin" />
                    {t('editAccount.proxyTesting')}
                  </>
                ) : (
                  <>
                    <PlugZap size={16} className="mr-2" />
                    {t('editAccount.proxyTest')}
                  </>
                )}
              </Button>
            </div>
          </div>

          {/* 分组 & 标签 */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3 rounded-xl border border-border bg-muted/20 p-3">
            <div className="min-w-0">
              <div className={`text-xs font-medium mb-1.5 flex items-center gap-1.5 text-muted-foreground`}>
                <Folder size={14} />
                {t('groups.title') || '分组'}
              </div>
              <GroupSelector
                groups={groups}
                value={selectedGroupId}
                onChange={setSelectedGroupId}
                onGroupsChange={setGroups}
              />
            </div>

            <div className="min-w-0">
              <div className={`text-xs font-medium mb-1.5 flex items-center gap-1.5 text-muted-foreground`}>
                <Tag size={14} />
                {t('tags.title') || '标签'}
              </div>
              <TagSelector
                selectedTagIds={selectedTagIds}
                onChange={setSelectedTagIds}
              />
            </div>
          </div>
        </div>

        {/* Sticky Footer */}
        <div className="sticky bottom-0 bg-background/95 backdrop-blur-sm p-4 border-t border-border flex justify-end gap-3 z-20">
          <Button variant="secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button
            variant="success"
            onClick={handleSave}
            disabled={saving}
            loading={saving}
          >
            {t('common.save')}
          </Button>
        </div>
      </div>
    </div>
  </DialogRoot>
  )

  return createPortal(dialogContent, document.body)
}

export default EditAccountModal
