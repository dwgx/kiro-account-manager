import { useState, useRef, useEffect, useMemo } from 'react'
import { createPortal } from 'react-dom'
import { invoke } from '@tauri-apps/api/core'
import { Shield } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { getAccountDisplayName, calcTotalUsageWithExtras } from '../../../utils/accountStats'
import { getAccountStatusMeta, isBannedStatus } from '../../../utils/accountStatus'
import {
  DialogRoot,
  DialogContent,
  DialogBody} from '../../shared/dialog'
import { Account, AvailableModel, ListAvailableModelsResponse } from '../../../types/account'
import { getPrimaryUsage } from './utils/accountModelFormat'
import { DetailHeader } from './AccountDetail/DetailHeader'
import { QuotaOverview } from './AccountDetail/QuotaOverview'
import { BasicInfoSection } from './AccountDetail/BasicInfoSection'
import { ModelsSection } from './AccountDetail/ModelsSection'

interface AccountDetailModalProps {
  account: Account;
  onClose: () => void;
  onRefresh?: () => void;
}

function AccountDetailModal({ account, onClose, onRefresh }: AccountDetailModalProps) {
  const { t } = useApp()
  const { showError } = useDialog()
  const [currentAccount, setCurrentAccount] = useState<Account>(account)

  // 样式定义
  const colors = useMemo(() => ({
    inputFocus: 'focus:ring-primary/20 focus:border-primary'
  }), [])

  const initialUsage = getPrimaryUsage(currentAccount)

  const [form, setForm] = useState({
    email: currentAccount.email || getAccountDisplayName(currentAccount),
    label: currentAccount.label || '',
    quota: initialUsage.quota,
    used: initialUsage.used,
    status: currentAccount.status,
    accessToken: currentAccount.accessToken || '',
    refreshToken: currentAccount.refreshToken || ''})

  const [refreshing, setRefreshing] = useState(false)
  const [copied, setCopied] = useState<string | null>(null)
  const copiedTimerRef = useRef<NodeJS.Timeout | null>(null)

  // Models 相关 state
  const [models, setModels] = useState<AvailableModel[]>([])
  const [modelsLoading, setModelsLoading] = useState(false)
  const [modelsError, setModelsError] = useState<string | null>(null)
  const [modelsExpanded, setModelsExpanded] = useState(false)

  // 获取可用模型
  const fetchModels = async (forceRefresh = false) => {
    setModelsLoading(true)
    setModelsError(null)
    try {
      console.log('[AccountDetailModal] Fetching models for account:', account.id, 'forceRefresh:', forceRefresh)
      const response = await invoke<ListAvailableModelsResponse>('list_available_models', {
        id: account.id,
        forceRefresh
      })
      console.log('[AccountDetailModal] Models response:', response)
      const modelsList = response.availableModels
      console.log('[AccountDetailModal] Models list:', modelsList.length, 'models')
      setModels(modelsList)
    } catch (e) {
      console.error('[AccountDetailModal] Failed to fetch models:', e)
      setModelsError(String(e))
    } finally {
      setModelsLoading(false)
    }
  }

  // 清理timer
  useEffect(() => {
    return () => {
      if (copiedTimerRef.current) {
        clearTimeout(copiedTimerRef.current)
      }
    }
  }, [])

  const handleToggleModelsExpanded = () => {
    const nextExpanded = !modelsExpanded
    setModelsExpanded(nextExpanded)
    if (nextExpanded && models.length === 0 && !modelsLoading) {
      fetchModels()
    }
  }

  useEffect(() => {
    setCurrentAccount(account)
    const usage = getPrimaryUsage(account)
    setForm({
      email: account.email || getAccountDisplayName(account),
      label: account.label || '',
      quota: usage.quota,
      used: usage.used,
      status: account.status,
      accessToken: account.accessToken || '',
      refreshToken: account.refreshToken || ''})
  }, [account])

  const handleRefresh = async () => {
    setRefreshing(true)
    try {
      const result = await invoke<{ account: Account, warning?: string }>('sync_account', { id: account.id })
      const updated = result.account
      setCurrentAccount(updated)

      // 如果有警告，显示提示
      if (result.warning) {
        await showError('同步警告', result.warning)
      }

      // 封禁账号额度为 0
      const isBanned = isBannedStatus(updated)
      const quota = isBanned ? 0 : (updated.usageData?.usageBreakdownList?.[0]?.usageLimit ?? 0)
      const used = updated.usageData?.usageBreakdownList?.[0]?.currentUsage ?? 0
      setForm(prev => ({ ...prev, quota, used, status: updated.status }))
      void onRefresh?.()
    } catch (e) {
      const errorMsg = String(e)
      // sync_account 后端已处理状态更新，前端同步状态用于表单显示
      let status = account.status
      if (errorMsg.includes('BANNED')) {
        status = 'banned'
      } else if (errorMsg.includes('AUTH_ERROR') || errorMsg.includes('401') || errorMsg.includes('invalid') || errorMsg.includes('失效')) {
        status = 'invalid'
      }
      setForm(prev => ({ ...prev, status }))
      await showError(t('detail.refreshFailed'), errorMsg)
    } finally {
      setRefreshing(false)
    }
  }

  const handleCopy = (text: string, field: string) => {
    navigator.clipboard.writeText(text).catch(e => console.error('Copy failed:', e))
    setCopied(field)
    if (copiedTimerRef.current) {
      clearTimeout(copiedTimerRef.current)
    }
    copiedTimerRef.current = setTimeout(() => setCopied(null), 1500)
  }

  // 计算总配额和使用量（基于表单值 + usageData 中的额外配额）
  const {
    totalQuota,
    totalUsed,
    totalPercent,
    freeTrialQuota,
    freeTrialUsed,
    bonusQuota,
    bonusUsed
  } = calcTotalUsageWithExtras(
    form.quota,
    form.used,
    currentAccount.usageData
  )

  // 从 usageData 读取额外信息（用于显示详情）
  const breakdown = currentAccount.usageData?.usageBreakdownList?.[0]
  const freeTrialInfo = breakdown?.freeTrialInfo
  const bonuses = breakdown?.bonuses || []

  const statusMeta = getAccountStatusMeta({ status: form.status, usageData: currentAccount.usageData }, t)

  return createPortal(
    <DialogRoot open={true} onOpenChange={(open) => !open && onClose()}>
      <DialogContent maxWidth="800px" showClose={false}>
        {/* 顶部渐变背景 */}
        <div className="absolute top-0 left-0 right-0 h-40 bg-gradient-to-br from-blue-500/5 via-purple-500/3 to-transparent pointer-events-none rounded-t-2xl" />

        <DetailHeader
          account={currentAccount}
          t={t}
          copied={copied}
          onCopy={handleCopy}
          onClose={onClose}
        />

        {/* Body - 使用 DialogBody 的 noPadding，自己控制每个区域的 padding */}
        <DialogBody noPadding>
          <QuotaOverview
            account={currentAccount}
            form={form}
            breakdown={breakdown}
            freeTrialInfo={freeTrialInfo}
            bonuses={bonuses}
            totals={{
              totalQuota,
              totalUsed,
              totalPercent,
              freeTrialQuota,
              freeTrialUsed,
              bonusQuota,
              bonusUsed
            }}
            colors={colors}
            t={t}
            refreshing={refreshing}
            onRefresh={handleRefresh}
          />

          <BasicInfoSection
            account={currentAccount}
            breakdown={breakdown}
            t={t}
          />

          <ModelsSection
            models={models}
            modelsLoading={modelsLoading}
            modelsError={modelsError}
            modelsExpanded={modelsExpanded}
            t={t}
            onToggle={handleToggleModelsExpanded}
            onForceRefresh={() => fetchModels(true)}
          />
        </DialogBody>

        {/* 状态栏（底部简洁显示） */}
        <div className="px-6 py-3 border-t border-border flex items-center gap-2">
          {statusMeta.tone === 'success'
            ? <><Shield size={14} className="text-green-500" /><span className="text-xs text-green-500 font-medium">{statusMeta.label}</span></>
            : statusMeta.tone === 'danger'
              ? <><Shield size={14} className="text-red-500" /><span className="text-xs text-red-500 font-medium">{statusMeta.label}</span></>
              : <><Shield size={14} className="text-orange-500" /><span className="text-xs text-orange-500 font-medium">{statusMeta.label}</span></>}
        </div>
      </DialogContent>
    </DialogRoot>,
    document.body
  )
}

export default AccountDetailModal
