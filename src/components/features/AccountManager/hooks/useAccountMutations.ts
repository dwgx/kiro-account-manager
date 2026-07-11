import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { showSuccess, showError } from '../../../../utils/toast'
import { normalizeAccountForUi } from '../utils/accountRuntime'
import { Account } from '../../../../types/account'

interface UseAccountMutationsParams {
  accounts: any[];
  localToken: any;
  selectedIds: string[];
  setAccounts: any;
  setSelectedIds: any;
  clearAvailableModelsState: (id: string) => void;
  handleRefreshStatus: (id: string) => Promise<any>;
  t: any;
  showConfirm: any;
}

export function useAccountMutations({
  accounts,
  localToken,
  selectedIds,
  setAccounts,
  setSelectedIds,
  clearAvailableModelsState,
  handleRefreshStatus,
  t,
  showConfirm}: UseAccountMutationsParams) {
  const [refreshingTokenId, setRefreshingTokenId] = useState<string | null>(null)
  const [refreshingQuotaId, setRefreshingQuotaId] = useState<string | null>(null)
  const [togglingOverageId, setTogglingOverageId] = useState<string | null>(null)

  const removeAccountsLocally = useCallback((ids: string[]) => {
    const idSet = new Set(ids)
    setAccounts(prev => prev.filter(account => !idSet.has(account.id)))
    setSelectedIds(prev => prev.filter(id => !idSet.has(id)))
    ids.forEach(clearAvailableModelsState)
  }, [clearAvailableModelsState, setAccounts])

  const updateAccountLocally = useCallback((updatedAccount: any) => {
    if (!updatedAccount?.id) return
    const normalizedAccount = normalizeAccountForUi(updatedAccount)
    setAccounts(prev => prev.map(account => account.id === normalizedAccount.id ? normalizedAccount : account))
  }, [setAccounts])

  // 刷新配额（智能同步：如果 token 失效会自动刷新）
  const handleRefreshQuota = useCallback(async (id: string) => {
    setRefreshingQuotaId(id)
    try {
      const result = await invoke<{ account: Account, warning?: string }>('sync_account', { id })
      updateAccountLocally(result.account)
      clearAvailableModelsState(id)
      if (result.warning) {
        showError('同步警告', result.warning)
      } else {
        showSuccess(t('accounts.refreshSuccess'))
      }
      return { success: true, account: result.account }
    } catch (e) {
      const errorMsg = String(e)
      if (errorMsg.includes('BANNED')) {
        showError(t('accounts.accountBanned'))
      } else if (errorMsg.includes('AUTH_ERROR') || errorMsg.includes('401') || errorMsg.includes('invalid') || errorMsg.includes('失效')) {
        showError(t('accounts.tokenInvalid'))
      } else if (errorMsg.includes('error sending request') || errorMsg.includes('connection') || errorMsg.includes('network') || errorMsg.includes('timeout')) {
        showError('❌ 网络连接失败\n\n可能原因：\n• 网络不稳定\n• 代理设置有误\n• 防火墙拦截\n\n解决方法：\n1. 检查网络连接\n2. 检查代理设置\n3. 关闭防火墙或添加白名单')
      } else {
        showError(errorMsg.slice(0, 100))
      }
      return { success: false, error: errorMsg }
    } finally {
      setRefreshingQuotaId(null)
    }
  }, [clearAvailableModelsState, updateAccountLocally, t])

  // 包装刷新函数，添加 toast 通知
  const handleRefreshWithNotify = useCallback(async (id: string) => {
    const result = await handleRefreshStatus(id)
    if (result.success) {
      clearAvailableModelsState(id)
      showSuccess(t('accounts.refreshSuccess'))
    } else if (result.error) {
      const errorMsg = result.error
      if (errorMsg.includes('BANNED')) {
        showError(t('accounts.accountBanned'))
      } else if (errorMsg.includes('AUTH_ERROR')) {
        // AUTH_ERROR: 静默处理，不弹窗
        console.log('[Sync] Token 已失效，已自动标记账号状态')
      } else if (errorMsg.includes('401') || errorMsg.includes('invalid')) {
        showError(t('accounts.tokenInvalid'))
      } else if (errorMsg.includes('error sending request') || errorMsg.includes('connection') || errorMsg.includes('network') || errorMsg.includes('timeout')) {
        showError('❌ 网络连接失败\n\n可能原因：\n• 网络不稳定\n• 代理设置有误\n• 防火墙拦截\n\n解决方法：\n1. 检查网络连接\n2. 检查代理设置\n3. 关闭防火墙或添加白名单')
      } else {
        showError(errorMsg.slice(0, 100))
      }
    }
    return result
  }, [clearAvailableModelsState, handleRefreshStatus, t])

  // 刷新 Token（刷新后自动获取最新配额）
  const handleRefreshToken = useCallback(async (id: string) => {
    setRefreshingTokenId(id)
    try {
      // 先刷新 token
      await invoke('refresh_token', { id })
      // 再获取配额（使用新 token）
      const result = await invoke<{ account: Account, warning?: string }>('get_usage_limits', { id })
      updateAccountLocally(result.account)
      clearAvailableModelsState(id)
      if (result.warning) {
        showError('刷新成功，但有警告', result.warning)
      } else {
        showSuccess('Token 刷新成功')
      }
      return { success: true, account: result.account }
    } catch (e) {
      const errorMsg = String(e)
      if (errorMsg.includes('BANNED')) {
        showError('账号已封禁')
      } else if (errorMsg.includes('AUTH_ERROR') || errorMsg.includes('401') || errorMsg.includes('invalid') || errorMsg.includes('失效')) {
        showError('Token 无效，刷新失败')
      } else if (errorMsg.includes('error sending request') || errorMsg.includes('connection') || errorMsg.includes('network') || errorMsg.includes('timeout')) {
        showError('❌ 网络连接失败\n\n可能原因：\n• 网络不稳定\n• 代理设置有误\n• 防火墙拦截\n\n解决方法：\n1. 检查网络连接\n2. 检查代理设置\n3. 关闭防火墙或添加白名单')
      } else {
        showError(errorMsg.slice(0, 100))
      }
      return { success: false, error: errorMsg }
    } finally {
      setRefreshingTokenId(null)
    }
  }, [clearAvailableModelsState, updateAccountLocally])

  // 切换账号启用/禁用
  const handleToggleEnabled = useCallback(async (account: any, enabled: boolean) => {
    // 乐观更新：立即更新本地状态
    updateAccountLocally({ ...account, enabled })

    try {
      const updated = await invoke<any>('update_account', { params: { id: account.id, enabled } })
      updateAccountLocally(updated)
    } catch (e) {
      // 失败时回滚
      updateAccountLocally({ ...account, enabled: !enabled })
      console.error('Toggle enabled failed:', e)
      showError('启用/禁用切换失败', String(e))
    }
  }, [updateAccountLocally])

  // 切换超额开关
  const handleToggleOverage = useCallback(async (account: any, enabled: boolean) => {
    setTogglingOverageId(account.id)

    // 乐观更新：立即更新本地状态
    updateAccountLocally({
      ...account,
      usageData: {
        ...account.usageData,
        overageConfiguration: {
          ...account.usageData?.overageConfiguration,
          overageStatus: enabled ? 'ENABLED' : 'DISABLED'
        }
      }
    })

    try {
      await invoke('set_overage_status', { id: account.id, enabled })
      // API 成功后，获取最新配额确保数据一致（不需要刷新 token）
      const result = await invoke<any>('get_usage_limits', { id: account.id })
      if (result?.account) {
        updateAccountLocally(result.account)
      }
    } catch (e) {
      // 失败时回滚状态
      updateAccountLocally({
        ...account,
        usageData: {
          ...account.usageData,
          overageConfiguration: {
            ...account.usageData?.overageConfiguration,
            overageStatus: !enabled ? 'ENABLED' : 'DISABLED'
          }
        }
      })
      console.error('Failed to toggle overage:', e)
      showError('超额开关切换失败', String(e))
    } finally {
      setTogglingOverageId(null)
    }
  }, [updateAccountLocally])

  // 删除单个账号
  const handleDelete = useCallback(async (id: string) => {
    // 防呆：检查是否是当前账号
    const account = accounts.find(a => a.id === id)
    const isCurrent = localToken?.refreshToken && account?.refreshToken === localToken.refreshToken

    if (isCurrent) {
      const confirmed = await showConfirm(
        '⚠️ 删除当前账号',
        '您正在删除当前使用的账号！\n\n删除后 Kiro IDE 将无法使用，需要重新登录。\n\n确定要删除吗？'
      )
      if (!confirmed) return
    } else {
      const confirmed = await showConfirm(t('accounts.delete'), t('accounts.confirmDelete'))
      if (!confirmed) return
    }

    await invoke('delete_account', { id })
    removeAccountsLocally([id])
  }, [accounts, localToken, removeAccountsLocally, showConfirm, t])

  // 远程删除账号（从 AWS 服务端注销）
  const handleDeleteRemote = useCallback(async (account: any) => {
    const confirmed = await showConfirm(
      '⚠️ ' + t('accountCard.deleteRemote'),
      '远程删除将从 AWS 服务端注销此账号！\n\n此操作不可恢复，账号将永久失效。\n\n' + t('accountCard.deleteRemoteConfirm')
    )
    if (confirmed) {
      try {
        await invoke('delete_account_remote', { id: account.id, deleteLocal: true })
        removeAccountsLocally([account.id])
      } catch (e) {
        // 错误已通过 showError 显示
      }
    }
  }, [removeAccountsLocally, showConfirm, t])

  // 批量删除
  const onBatchDelete = useCallback(async () => {
    if (selectedIds.length === 0) return

    // 防呆：检查是否包含当前账号
    const currentAccount = accounts.find(a => localToken?.refreshToken && a.refreshToken === localToken.refreshToken)
    const includesCurrent = currentAccount && selectedIds.includes(currentAccount.id)

    if (includesCurrent) {
      const confirmed = await showConfirm(
        '⚠️ 批量删除包含当前账号',
        `您选择了 ${selectedIds.length} 个账号，其中包含当前使用的账号！\n\n删除后 Kiro IDE 将无法使用，需要重新登录。\n\n确定要删除吗？`
      )
      if (!confirmed) return
    } else {
      const confirmed = await showConfirm(t('accounts.batchDelete'), t('accounts.confirmDeleteMultiple', { count: selectedIds.length }))
      if (!confirmed) return
    }

    await invoke('delete_accounts', { ids: selectedIds })
    removeAccountsLocally(selectedIds)
    setSelectedIds([]) // 清除选中状态
  }, [accounts, selectedIds, localToken, removeAccountsLocally, showConfirm, t])

  return {
    updateAccountLocally,
    removeAccountsLocally,
    refreshingTokenId,
    refreshingQuotaId,
    togglingOverageId,
    handleRefreshQuota,
    handleRefreshWithNotify,
    handleRefreshToken,
    handleToggleEnabled,
    handleToggleOverage,
    handleDelete,
    handleDeleteRemote,
    onBatchDelete}
}
