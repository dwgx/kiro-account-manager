import { useState, useCallback, useEffect } from 'react'
import { getTags, getGroups } from '../../../../api/groupTag'

export function useTagGroupDefinitions() {
  const [tagDefinitions, setTagDefinitions] = useState<any[]>([])
  const [groupDefinitions, setGroupDefinitions] = useState<any[]>([])

  // 加载标签定义
  const loadTagDefinitions = useCallback(() => {
    getTags()
      .then(tags => {
        setTagDefinitions(tags as any[])
      })
      .catch(() => {
        // 静默处理
      })
  }, [])

  // 加载分组定义
  const loadGroupDefinitions = useCallback(() => {
    getGroups().then(setGroupDefinitions).catch(() => {})
  }, [])

  useEffect(() => {
    loadTagDefinitions()
    loadGroupDefinitions()
  }, [loadTagDefinitions, loadGroupDefinitions])

  return {
    tagDefinitions,
    groupDefinitions,
    loadTagDefinitions,
    loadGroupDefinitions}
}
