import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { ListAvailableModelsResponse } from '../../../../types/account'

export function useAvailableModels(setAccounts: any) {
  const [availableModelsById, setAvailableModelsById] = useState<Record<string, any>>({})
  const [availableModelsLoadingById, setAvailableModelsLoadingById] = useState<Record<string, boolean>>({})
  const [availableModelsErrorById, setAvailableModelsErrorById] = useState<Record<string, string>>({})

  const clearAvailableModelsState = useCallback((id: string) => {
    setAvailableModelsById(prev => {
      if (!(id in prev)) return prev
      const next = { ...prev }
      delete next[id]
      return next
    })
    setAvailableModelsLoadingById(prev => {
      if (!(id in prev)) return prev
      const next = { ...prev }
      delete next[id]
      return next
    })
    setAvailableModelsErrorById(prev => {
      if (!(id in prev)) return prev
      const next = { ...prev }
      delete next[id]
      return next
    })
  }, [])

  const handleLoadAvailableModels = useCallback(async (id: string, options: any = {}) => {
    const { forceRefresh = false } = options
    setAvailableModelsLoadingById(prev => ({ ...prev, [id]: true }))
    setAvailableModelsErrorById(prev => {
      if (!(id in prev)) return prev
      const next = { ...prev }
      delete next[id]
      return next
    })

    try {
      const response = await invoke<ListAvailableModelsResponse>('list_available_models', { id, forceRefresh })
      const models = response.availableModels
      setAvailableModelsById(prev => ({ ...prev, [id]: models }))
      setAccounts(prev => prev.map(account => (
        account.id === id
          ? {
              ...account,
              availableModelsCache: {
                response,
                cachedAt: Math.floor(Date.now() / 1000)}}
          : account
      )))
      return response
    } catch (e) {
      const message = String(e)
      setAvailableModelsErrorById(prev => ({ ...prev, [id]: message }))
      throw e
    } finally {
      setAvailableModelsLoadingById(prev => ({ ...prev, [id]: false }))
    }
  }, [setAccounts])

  return {
    availableModelsById,
    availableModelsLoadingById,
    availableModelsErrorById,
    clearAvailableModelsState,
    handleLoadAvailableModels}
}
