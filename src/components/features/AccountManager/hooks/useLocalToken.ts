import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

export function useLocalToken() {
  // 当前登录的本地 token
  const [localToken, setLocalToken] = useState<any>(null)

  useEffect(() => {
    invoke<any>('get_kiro_local_token').then(setLocalToken).catch(() => setLocalToken(null))
  }, [])

  return { localToken, setLocalToken }
}
