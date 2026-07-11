import { useState, useCallback, useMemo, useEffect, useRef } from 'react'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { useAccounts } from './hooks/useAccounts'
import { useSwitchAccount } from './hooks/useSwitchAccount'
import { useLocalToken } from './hooks/useLocalToken'
import { useTagGroupDefinitions } from './hooks/useTagGroupDefinitions'
import { useAvailableModels } from './hooks/useAvailableModels'
import { useAccountFilters } from './hooks/useAccountFilters'
import { useAccountMutations } from './hooks/useAccountMutations'
import { cn } from '../../../utils/cn'
import { showError } from '../../../utils/toast'
import { normalizeAccountForUi } from './utils/accountRuntime'
import AccountHeader from './AccountHeader'
import AccountTable from './AccountTable'
import AccountListView from './AccountListView'
import ImportAccountModal from './ImportAccountModal'
import AccountDetailModal from './AccountDetailModal'
import EditAccountModal from './EditAccountModal'
import BatchEditModal from './BatchEditModal'
import ConfirmModal from './ConfirmModal'
import EmptyAccountsState from './EmptyAccountsState'
import { AccountListSkeleton, AccountTableSkeleton } from '../../shared/Skeleton'
import { getThemeAccent } from '../KiroConfig/themeAccent'

interface AccountManagerProps {
  onNavigate: (path: string) => void;
}

function AccountManager({ onNavigate }: AccountManagerProps) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const { showConfirm } = useDialog()

  const [selectedIds, setSelectedIds] = useState<string[]>([])

  // 优化：将 selectedIds 转为 Set，提升查找性能（O(1) vs O(n)）
  const selectedIdsSet = useMemo(() => new Set(selectedIds), [selectedIds])
  const [editingAccount, setEditingAccount] = useState<any>(null)
  const [editingLabelAccount, setEditingLabelAccount] = useState<any>(null)
  const [showImportModal, setShowImportModal] = useState(false)
  const [showBatchEditModal, setShowBatchEditModal] = useState(false)
  const [copiedId, setCopiedId] = useState<string | null>(null)
  const [viewMode, setViewMode] = useState(() => localStorage.getItem('accountViewMode') || 'card')

  // 用于管理复制提示的timer
  const copiedTimerRef = useRef<NodeJS.Timeout | null>(null)

  // 当前登录的本地 token
  const { localToken, setLocalToken } = useLocalToken()

  // 切换账号 hook
  const {
    switchingId,
    switchDialog,
    setSwitchDialog,
    handleSwitchAccount,
    handleLogoutAccount,
    confirmSwitch,
    closeSwitchDialog} = useSwitchAccount(setLocalToken)

  // 标签/分组定义
  const {
    tagDefinitions,
    groupDefinitions,
    loadTagDefinitions,
    loadGroupDefinitions} = useTagGroupDefinitions()

  // 清理timer
  useEffect(() => {
    return () => {
      if (copiedTimerRef.current) {
        clearTimeout(copiedTimerRef.current)
      }
    }
  }, [])

  const {
    accounts,
    setAccounts,
    loading,
    loadAccounts,
    autoRefreshing,
    refreshProgress,
    lastRefreshTime,
    refreshingId,
    batchRefreshAccounts,
    handleRefreshStatus,
    handleExport} = useAccounts()

  // 可用模型（依赖 setAccounts）—— 必须在 mutations 之前
  const {
    availableModelsById,
    availableModelsLoadingById,
    availableModelsErrorById,
    clearAvailableModelsState,
    handleLoadAvailableModels} = useAvailableModels(setAccounts)

  // 筛选/排序（依赖 accounts + tagDefinitions）
  const {
    searchTerm,
    selectedTag,
    selectedStatus,
    selectedGroup,
    advancedFilters,
    setAdvancedFilters,
    sortBy,
    setSortBy,
    allTags,
    filteredAccounts,
    handleSearchChange,
    handleGroupFilter,
    handleTagFilter,
    handleStatusFilter} = useAccountFilters(accounts, tagDefinitions)

  // 写操作（依赖上面几乎所有东西）
  const {
    updateAccountLocally,
    refreshingTokenId,
    refreshingQuotaId,
    togglingOverageId,
    handleRefreshQuota,
    handleRefreshToken,
    handleToggleEnabled,
    handleToggleOverage,
    handleDelete,
    handleDeleteRemote,
    onBatchDelete} = useAccountMutations({
      accounts,
      localToken,
      selectedIds,
      setAccounts,
      setSelectedIds,
      clearAvailableModelsState,
      handleRefreshStatus,
      t,
      showConfirm})

  const accountRowStateById = useMemo(() => {
    const result: Record<string, any> = {}
    for (const account of filteredAccounts) {
      const id = account.id
      result[id] = {
        isRefreshing: refreshingId === id,
        isRefreshingToken: refreshingTokenId === id,
        isRefreshingQuota: refreshingQuotaId === id,
        isSwitching: switchingId === id,
        isTogglingOverage: togglingOverageId === id,
        isCopied: copiedId === id,
        availableModels: availableModelsById[id] ?? null,
        availableModelsLoading: Boolean(availableModelsLoadingById[id]),
        availableModelsError: availableModelsErrorById[id] ?? ''}
    }
    return result
  }, [filteredAccounts, refreshingId, refreshingTokenId, refreshingQuotaId, switchingId, togglingOverageId, copiedId, availableModelsById, availableModelsLoadingById, availableModelsErrorById])


  const handleViewModeChange = useCallback((mode: string) => {
    setViewMode(mode)
    localStorage.setItem('accountViewMode', mode)
  }, [])
  const handleSelectAll = useCallback((checked: boolean) => {
    setSelectedIds(checked ? filteredAccounts.map(a => a.id) : [])
  }, [filteredAccounts])

  const handleSelectOne = useCallback((id: string, checked: boolean) => {
    setSelectedIds(prev => checked ? [...prev, id] : prev.filter(i => i !== id))
  }, [])
  const handleCopy = useCallback((text: string, id: string) => {
    navigator.clipboard.writeText(text).catch(e => console.error('Copy failed:', e))
    setCopiedId(id)
    if (copiedTimerRef.current) {
      clearTimeout(copiedTimerRef.current)
    }
    copiedTimerRef.current = setTimeout(() => setCopiedId(null), 1500)
  }, [])

  return (
    <div className={cn('h-full flex flex-col', "glass-main")}>
      <div className="flex-1 flex flex-col min-h-0">
      <AccountHeader
        searchTerm={searchTerm}
        onSearchChange={handleSearchChange}
        selectedCount={selectedIds.length}
        onBatchDelete={onBatchDelete}
        onBatchEdit={() => setShowBatchEditModal(true)}
        onImport={() => setShowImportModal(true)}
        onExport={async () => {
          if (selectedIds.length === 0) {
            showError(t('accounts.exportSelectFirst') || '请先选择要导出的账号')
            return
          }
          await handleExport(selectedIds)
          setSelectedIds([]) // 清除选中状态
        }}
        onRefresh={loadAccounts}
        onRefreshAll={async () => {
          if (selectedIds.length === 0) {
            showError(t('accounts.refreshSelectFirst') || '请先选择要刷新的账号')
            return
          }
          await batchRefreshAccounts(selectedIds, accounts)
          setSelectedIds([]) // 清除选中状态
        }}
        autoRefreshing={autoRefreshing}
        refreshProgress={refreshProgress}
        allGroups={groupDefinitions}
        selectedGroup={selectedGroup}
        onGroupFilter={handleGroupFilter}
        allTags={allTags}
        selectedTag={selectedTag}
        onTagFilter={handleTagFilter}
        selectedStatus={selectedStatus}
        onStatusFilter={handleStatusFilter}
        sortBy={sortBy}
        onSortChange={setSortBy}
        viewMode={viewMode}
        onViewModeChange={handleViewModeChange}
        advancedFilters={advancedFilters}
        onAdvancedFiltersChange={setAdvancedFilters}
        totalCount={filteredAccounts.length}
        onSelectAll={handleSelectAll}
        onDeselectAll={() => setSelectedIds([])}
      />
      <div className="flex-1 flex flex-col min-h-0">
      {loading ? (
        viewMode === 'card' ? <AccountListSkeleton count={8} /> : <AccountTableSkeleton count={8} />
      ) : filteredAccounts.length === 0 ? (
        <EmptyAccountsState
          hasFilter={Boolean(searchTerm || selectedGroup || selectedTag || selectedStatus)}
          accent={accent}
          onImport={() => setShowImportModal(true)}
        />
      ) : viewMode === 'card' ? (
        <AccountTable
          accounts={filteredAccounts}
          totalCount={accounts.length}
          selectedIds={selectedIds}
          onSelectAll={handleSelectAll}
          onSelectOne={handleSelectOne}
          copiedId={copiedId}
          onCopy={handleCopy}
          onLogin={handleSwitchAccount}
          onLogout={handleLogoutAccount}
          onRefresh={handleRefreshQuota}
          onRefreshToken={handleRefreshToken}
          onEdit={setEditingAccount}
          onEditLabel={setEditingLabelAccount}
          onToggleEnabled={handleToggleEnabled}
          onToggleOverage={handleToggleOverage}
          onDelete={handleDelete}
          onDeleteRemote={handleDeleteRemote}
          onAdd={() => setShowImportModal(true)}
          localToken={localToken}
          tagDefinitions={tagDefinitions}
          groupDefinitions={groupDefinitions}
          accountRowStateById={accountRowStateById}
          onLoadAvailableModels={handleLoadAvailableModels}
        />
      ) : (
        <AccountListView
          accounts={filteredAccounts}
          totalCount={accounts.length}
          selectedIds={selectedIds}
          selectedIdsSet={selectedIdsSet}
          onSelectAll={handleSelectAll}
          onSelectOne={handleSelectOne}
          onCopy={handleCopy}
          onLogin={handleSwitchAccount}
          onLogout={handleLogoutAccount}
          onRefresh={handleRefreshQuota}
          onRefreshToken={handleRefreshToken}
          onEdit={setEditingAccount}
          onEditLabel={setEditingLabelAccount}
          onToggleEnabled={handleToggleEnabled}
          onToggleOverage={handleToggleOverage}
          onDelete={handleDelete}
          onDeleteRemote={handleDeleteRemote}
          onAdd={() => setShowImportModal(true)}
          localToken={localToken}
          tagDefinitions={tagDefinitions}
          groupDefinitions={groupDefinitions}
          accountRowStateById={accountRowStateById}
          onLoadAvailableModels={handleLoadAvailableModels}
          sortBy={sortBy}
          onSortChange={setSortBy}
          onDeselectAll={() => setSelectedIds([])}
        />

      )}
      </div>
      {editingAccount && (
        <AccountDetailModal
          account={editingAccount}
          onClose={() => setEditingAccount(null)}
          onRefresh={loadAccounts}
        />
      )}
      {editingLabelAccount && (
        <EditAccountModal
          account={editingLabelAccount}
          onClose={() => setEditingLabelAccount(null)}
          onSuccess={(updatedAccount: any) => {
            setEditingLabelAccount(null)
            if (updatedAccount) {
              updateAccountLocally(updatedAccount)
            }
            loadTagDefinitions()
            loadGroupDefinitions()
          }}
        />
      )}
      {showImportModal && (
        <ImportAccountModal
          onClose={() => setShowImportModal(false)}
          onSuccess={({ added = [], updated = [] }) => {
            setShowImportModal(false)
            setAccounts(prev => {
              const next = [...prev]
              const upsert = (entry: any) => {
                const account = normalizeAccountForUi(entry?.account)
                if (!account?.id) return
                const index = next.findIndex(item => item.id === account.id)
                if (index >= 0) next[index] = account
                else next.unshift(account)
              }
              added.forEach(upsert)
              updated.forEach(upsert)
              return next
            })
          }}
          onNavigate={onNavigate}
        />
      )}
      {showBatchEditModal && (
        <BatchEditModal
          accountIds={selectedIds}
          accounts={accounts}
          onClose={() => setShowBatchEditModal(false)}
          onSuccess={({ accountIds: updatedIds, selectedTagIds, selectedGroupId }) => {
            setShowBatchEditModal(false)
            setAccounts(prev => prev.map(account => {
              if (!updatedIds.includes(account.id)) return account
              const nextTagLinks = Array.isArray(selectedTagIds)
                ? selectedTagIds.map(tagId => ({ tagId }))
                : account.tagLinks
              return {
                ...account,
                tagLinks: nextTagLinks,
                groupId: selectedGroupId
              }
            }))
            loadTagDefinitions()
            setSelectedIds([])
          }}
        />
      )}


      {/* 切换账号弹窗 */}
      {switchDialog && (
        <ConfirmModal
          type={switchDialog.type}
          title={switchDialog.title}
          message={switchDialog.message}
          onConfirm={switchDialog.type === 'confirm' ? confirmSwitch : closeSwitchDialog}
          onCancel={closeSwitchDialog}
          confirmText={switchDialog.type === 'confirm'
            ? ((switchDialog as any).mode === 'logout' ? t('switch.confirmLogout') : t('switch.confirmBtn'))
            : t('common.ok')}
          customContent={switchDialog.type === 'confirm' ? (
            <div className="flex items-center gap-3 mt-3 p-3 rounded-xl bg-muted/30 border border-border">
              <span className="text-xs text-muted-foreground font-medium shrink-0">切换目标</span>
              <div className="flex gap-1.5 flex-1">
                {(['ide', 'cli', 'both'] as const).map(target => (
                  <button
                    key={target}
                    onClick={() => setSwitchDialog({ ...switchDialog, switchTarget: target })}
                    className={`flex-1 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all duration-200 ${
                      (switchDialog as any).switchTarget === target
                        ? 'bg-primary text-primary-foreground shadow-sm'
                        : 'bg-muted/50 text-muted-foreground hover:bg-muted hover:text-foreground'
                    }`}
                  >
                    {target === 'ide' ? '🖥 IDE' : target === 'cli' ? '⌨ CLI' : '🔗 Both'}
                  </button>
                ))}
              </div>
            </div>
          ) : null}
        />
      )}
      </div>
    </div>
  )
}

export default AccountManager
