import { useState, useCallback, useMemo, useEffect } from 'react'
import { applyFilters } from '../utils/filterUtils'
import { getAccountDisplayName, calcAccountUsagePercent } from '../../../../utils/accountStats'
import { normalizeAccountStatus } from '../../../../utils/accountStatus'

export function useAccountFilters(accounts: any[], tagDefinitions: any[]) {
  const [searchTerm, setSearchTerm] = useState('')
  const [selectedTag, setSelectedTag] = useState<string | null>(null)
  const [selectedStatus, setSelectedStatus] = useState<string | null>(null)
  const [selectedGroup, setSelectedGroup] = useState<string | null>(null)
  const [advancedFilters, setAdvancedFilters] = useState<any>({
    subscriptions: [],
    statuses: [],
    providers: [],
    usageRange: null
  })
  const [sortBy, setSortBy] = useState('trialAsc')

  // 获取所有标签（从标签定义中获取）
  const allTags = useMemo(() => {
    // 收集账号中使用的标签 ID（从 tagLinks 中提取）
    const usedTagIds = new Set<string>()
    accounts.forEach(a => {
      if (a.tagLinks) a.tagLinks.forEach((link: any) => usedTagIds.add(link.tagId))
    })
    // 返回被使用的标签定义
    return tagDefinitions.filter(t => usedTagIds.has(t.id))
  }, [accounts, tagDefinitions])

  // 当选中的标签不存在时，重置筛选（排除 __none__ 和 __has__ 特殊值）
  useEffect(() => {
    if (selectedTag && selectedTag !== '__none__' && selectedTag !== '__has__' && !allTags.find(t => t.id === selectedTag)) {
      setSelectedTag(null)
    }
  }, [allTags, selectedTag])


  // 优化：将 tagDefinitions 转为 Map，提升查找性能
  const tagDefinitionsMap = useMemo(() => {
    return new Map(tagDefinitions.map(t => [t.id, t]))
  }, [tagDefinitions])

  const normalizedAccounts = useMemo(() => accounts.map((account) => {
    const tagIds = (account.tagLinks || []).map((link: any) => link.tagId)
    const displayName = getAccountDisplayName(account).toLowerCase()
    const label = String(account.label || '').toLowerCase()
    const tagNames = tagIds
      .map(tagId => String(tagDefinitionsMap.get(tagId)?.name || ''))
      .filter(Boolean)
      .join(' ')
      .toLowerCase()

    return {
      account,
      tagIds,
      tagNames,
      label,
      displayName}
  }), [accounts, tagDefinitionsMap])

  const getTrialExpiry = useCallback((account: any) => {
    const expiry = account.usageData?.usageBreakdownList?.[0]?.freeTrialInfo?.freeTrialExpiry
    if (!expiry) return Number.POSITIVE_INFINITY
    return Number(expiry)
  }, [])

  const getUsagePercent = useCallback((account: any) => {
    return calcAccountUsagePercent(account)
  }, [])

  const filteredAccounts = useMemo(() => {
    const term = searchTerm.toLowerCase().trim()
    let result = normalizedAccounts.filter(({ account, tagIds, tagNames, label, displayName }) => {
      const matchSearch = !term || displayName.includes(term) || label.includes(term) || tagNames.includes(term)
      const matchGroup = !selectedGroup ||
        (selectedGroup === '__none__' ? !account.groupId :
         selectedGroup === '__has__' ? !!account.groupId :
         account.groupId === selectedGroup)
      const matchTag = !selectedTag ||
        (selectedTag === '__none__' ? tagIds.length === 0 :
         selectedTag === '__has__' ? tagIds.length > 0 :
         tagIds.includes(selectedTag))
      const matchStatus = !selectedStatus ||
        normalizeAccountStatus(account) === selectedStatus
      return matchSearch && matchGroup && matchTag && matchStatus
    })

    const hasAdvancedFilters = Boolean(
      advancedFilters?.usageRange ||
      advancedFilters?.subscriptions?.length ||
      advancedFilters?.statuses?.length ||
      advancedFilters?.providers?.length
    )

    if (hasAdvancedFilters) {
      const filteredIds = new Set(
        applyFilters(result.map(({ account }) => account), advancedFilters).map(account => account.id)
      )
      result = result.filter(({ account }) => filteredIds.has(account.id))
    }

    const sorted = [...result].sort((a, b) => {
      const accountA = a.account
      const accountB = b.account
      switch (sortBy) {
        case 'usageAsc':
          return getUsagePercent(accountA) - getUsagePercent(accountB)
        case 'usageDesc':
          return getUsagePercent(accountB) - getUsagePercent(accountA)
        case 'trialAsc':
          return getTrialExpiry(accountA) - getTrialExpiry(accountB)
        case 'trialDesc':
          return getTrialExpiry(accountB) - getTrialExpiry(accountA)
        case 'addedAsc':
          return new Date(accountA.addedAt || 0).getTime() - new Date(accountB.addedAt || 0).getTime()
        case 'addedDesc':
          return new Date(accountB.addedAt || 0).getTime() - new Date(accountA.addedAt || 0).getTime()
        default:
          return 0
      }
    })

    return sorted.map(({ account }) => account)
  }, [advancedFilters, getTrialExpiry, getUsagePercent, normalizedAccounts, searchTerm, selectedGroup, selectedStatus, selectedTag, sortBy])

  const handleSearchChange = useCallback((term: string) => { setSearchTerm(term) }, [])
  const handleGroupFilter = useCallback((group: string | null) => { setSelectedGroup(group) }, [])
  const handleTagFilter = useCallback((tag: string | null) => { setSelectedTag(tag) }, [])
  const handleStatusFilter = useCallback((status: string | null) => { setSelectedStatus(status) }, [])

  return {
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
    handleStatusFilter}
}
