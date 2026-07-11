import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { getConcurrency } from '../../../../utils/concurrency'
import { getAccountDisplayName } from '../../../../utils/accountStats'
import { isBannedStatus } from '../../../../utils/accountStatus'
import { validateAccount } from '../utils/importAccountValidation'

interface UseImportAccountOptions {
  onSuccess?: (data: { added: any[]; updated: any[] }) => void;
}

export function useImportAccount({ onSuccess }: UseImportAccountOptions) {
  const [activeTab, setActiveTab] = useState('json')
  const [osType, setOsType] = useState('')
  const [jsonText, setJsonText] = useState('')
  const [parseResult, setParseResult] = useState<any>(null)
  const [importing, setImporting] = useState(false)
  const [importProgress, setImportProgress] = useState({ current: 0, total: 0 })
  const [importResult, setImportResult] = useState<any>(null)

  // 从 Kiro 导入相关状态
  const [kiroAccounts, setKiroAccounts] = useState<any[]>([])
  const [kiroLoading, setKiroLoading] = useState(false)
  const [kiroError, setKiroError] = useState<string | null>(null)
  const [kiroImporting, setKiroImporting] = useState(false)
  const [kiroProgress, setKiroProgress] = useState({ current: 0, total: 0 })
  const [kiroResult, setKiroResult] = useState<any>(null)

  // 从 kiro-cli 导入相关状态
  // 在线登录相关状态
  const [onlineLoginPending, setOnlineLoginPending] = useState(false)
  const [onlineLoginError, setOnlineLoginError] = useState('')
  const [showWaitingModal, setShowWaitingModal] = useState(false)
  const [waitingProviderName, setWaitingProviderName] = useState('')
  const [supportedProviders, setSupportedProviders] = useState<string[]>([])
  const [showEnterpriseModal, setShowEnterpriseModal] = useState(false)
  const [enterpriseStartUrl, setEnterpriseStartUrl] = useState('')
  const [enterpriseRegion, setEnterpriseRegion] = useState('us-east-1')

  // 从 kiro-cli 导入相关状态
  const [kiroCliDbPath, setKiroCliDbPath] = useState('')
  const [kiroCliDetected, setKiroCliDetected] = useState(false)
  const [kiroCliDetecting, setKiroCliDetecting] = useState(false)
  const [kiroCliImporting, setKiroCliImporting] = useState(false)
  const [kiroCliResult, setKiroCliResult] = useState<any>(null)
  const isWindowsOs = osType === 'windows'

  useEffect(() => {
    let isMounted = true

    const detectOsType = async () => {
      try {
        const info = await invoke<any>('get_system_machine_guid')
        if (isMounted && info?.osType) {
          setOsType(info.osType)
          return
        }
      } catch (_) {
        // ignore and fallback to userAgent detection
      }

      if (!isMounted) return
      const userAgent = (navigator.userAgent || '').toLowerCase()
      if (userAgent.includes('windows')) {
        setOsType('windows')
      } else if (userAgent.includes('mac os') || userAgent.includes('macos')) {
        setOsType('macos')
      } else if (userAgent.includes('linux')) {
        setOsType('linux')
      }
    }

    detectOsType()

    return () => {
      isMounted = false
    }
  }, [])

  // 自动检测 kiro-cli 数据库路径
  useEffect(() => {
    if (activeTab === 'kiro-cli' && !kiroCliDbPath) {
      detectKiroCliPath()
    }
  }, [activeTab, kiroCliDbPath])

  const detectKiroCliPath = async () => {
    setKiroCliDetecting(true)
    try {
      const defaultPath = await invoke<string>('get_kiro_cli_default_path')
      if (defaultPath) {
        setKiroCliDbPath(defaultPath)
        setKiroCliDetected(true)
      } else {
        setKiroCliDetected(false)
      }
    } catch (e) {
      console.error('获取默认路径失败:', e)
      setKiroCliDetected(false)
    } finally {
      setKiroCliDetecting(false)
    }
  }

  // 自动检测 Kiro 账号
  useEffect(() => {
    if (activeTab === 'kiro') {
      detectKiroAccounts()
    }
  }, [activeTab])

  const detectKiroAccounts = async () => {
    setKiroLoading(true)
    setKiroError(null)
    try {
      const accounts = await invoke<any[]>('read_kiro_accounts')
      setKiroAccounts(accounts)
    } catch (e) {
      setKiroError(String(e))
      setKiroAccounts([])
    } finally {
      setKiroLoading(false)
    }
  }

  const parseJson = (text: string) => {
    if (!text.trim()) {
      setParseResult(null)
      return
    }

    try {
      let data = JSON.parse(text)
      if (!Array.isArray(data)) data = [data]

      const valid: any[] = []
      const invalid: any[] = []
      const errors: string[] = []

      data.forEach((item, index) => {
        const result = validateAccount(item, index)
        if (result.valid) {
          valid.push({ ...item, _type: result.type, _index: index, _inferredProvider: result.inferredProvider })
        } else {
          invalid.push({ ...item, _index: index })
          errors.push(...result.errors)
        }
      })

      setParseResult({ valid, invalid, errors })
    } catch (e: any) {
      setParseResult({ valid: [], invalid: [], errors: [`JSON 解析失败: ${e.message}`] })
    }
  }

  const handleFileSelect = async (file: File) => {
    if (!file) return
    const text = await file.text()
    setJsonText(text)
    parseJson(text)
  }

  const runConcurrent = async (items: any[], handler: any, onProgress: any) => {
    const results = []
    let completed = 0
    const concurrency = getConcurrency(items.length)

    for (let i = 0; i < items.length; i += concurrency) {
      const batch = items.slice(i, i + concurrency)
      const batchResults = await Promise.all(
        batch.map(async (item) => {
          const result = await handler(item)
          completed++
          onProgress(completed)
          return result
        })
      )
      results.push(...batchResults)
    }
    return results
  }

  const handleJsonImport = async () => {
    if (!parseResult?.valid.length) return

    setImporting(true)
    setImportProgress({ current: 0, total: parseResult.valid.length })

    const added: any[] = []
    const updated: any[] = []
    const failed: any[] = []

    const importOne = async (item: any) => {
      try {
        let result: any
        const provider = item._inferredProvider || item.provider
        if (item._type === 'external_idp') {
          result = await invoke('add_account_by_external_idp', {
            refreshToken: item.refreshToken ?? item.refresh_token,
            clientId: item.clientId ?? item.client_id,
            profileArn: item.profileArn ?? item.profile_arn,
            clientSecret: (item.clientSecret ?? item.client_secret) || null,
            accessToken: (item.accessToken ?? item.access_token) || null,
            tokenEndpoint: (item.tokenEndpoint ?? item.token_endpoint) || null,
            issuerUrl: (item.issuerUrl ?? item.issuer_url) || null,
            scopes: (item.scopes) || null,
            region: (item.authRegion ?? item.auth_region ?? item.region) || null,
            machineId: (item.machineId ?? item.machine_id) || null,
            email: (item.email) || null,
            expiresAt: (item.expiresAt ?? item.expires_at) || null,
          })
        } else if (item._type === 'social') {
          result = await invoke('add_account_by_social', {
            refreshToken: item.refreshToken,
            provider: provider,
            machineId: item.machineId || null,
            accessToken: item.accessToken || null
          })
        } else {
          // IdC 账号：统一调用 add_account_by_idc
          const params = {
            provider,  // BuilderId 或 Enterprise
            refreshToken: item.refreshToken,
            clientId: item.clientId,
            clientSecret: item.clientSecret,
            region: item.region || null,
            machineId: item.machineId || null,
            accessToken: item.accessToken || null,
            password: item.password || null,
            startUrl: item.startUrl || null,  // Enterprise 可能需要
            clientIdHash: item.clientIdHash || null  // Enterprise 可用 clientIdHash 替代 startUrl
          }

          result = await invoke('add_account_by_idc', params)
        }

        const account = result.account
        if (isBannedStatus(account.status)) {
          return { success: true, index: item._index + 1, email: getAccountDisplayName(account), account, isNew: result.isNew, banned: true }
        }
        return { success: true, index: item._index + 1, email: getAccountDisplayName(account), account, isNew: result.isNew }
      } catch (e) {
        const errorMsg = String(e)
        if (errorMsg.includes('BANNED')) {
          return { success: false, index: item._index + 1, error: '账号已封禁', banned: true }
        }
        return { success: false, index: item._index + 1, error: errorMsg.slice(0, 50) }
      }
    }

    const results = await runConcurrent(
      parseResult.valid,
      importOne,
      (completed: number) => setImportProgress({ current: completed, total: parseResult.valid.length })
    )

    results.forEach(r => {
      if (r.success) {
        if (r.isNew) {
          added.push({ index: r.index, email: r.email, account: r.account })
        } else {
          updated.push({ index: r.index, email: r.email, account: r.account })
        }
      } else {
        failed.push({ index: r.index, error: r.error })
      }
    })

    setImportResult({ added, updated, failed })
    setImporting(false)
    if (added.length > 0 || updated.length > 0) onSuccess?.({ added, updated })
  }

  const handleKiroImport = async () => {
    if (kiroAccounts.length === 0) return

    setKiroImporting(true)
    setKiroProgress({ current: 0, total: kiroAccounts.length })

    const added: any[] = []
    const updated: any[] = []
    const failed: any[] = []

    const importOne = async (account: any) => {
      try {
        let result: any
        if (account.authMethod === 'IdC') {
          // IdC 账号：统一调用 add_account_by_idc
          const params = {
            provider: account.provider,  // BuilderId 或 Enterprise
            refreshToken: account.refreshToken,
            clientId: account.clientId,
            clientSecret: account.clientSecret,
            region: account.region || null,
            machineId: null,
            accessToken: account.accessToken || null,
            password: null,
            startUrl: null,  // 从 Kiro 导入时不需要 startUrl（使用 clientIdHash）
            clientIdHash: account.clientIdHash || null  // 使用 Kiro 提供的 clientIdHash
          }

          result = await invoke('add_account_by_idc', params)
        } else {
          result = await invoke('add_account_by_social', {
            refreshToken: account.refreshToken,
            provider: account.provider,
            machineId: null,
            accessToken: account.accessToken || null
          })
        }

        const acc = result.account
        if (isBannedStatus(acc.status)) {
          return { success: true, email: acc.email, account: acc, isNew: result.isNew, banned: true }
        }
        return { success: true, email: acc.email, account: acc, isNew: result.isNew }
      } catch (e) {
        const errorMsg = String(e)
        if (errorMsg.includes('BANNED')) {
          return { success: false, error: '账号已封禁', banned: true }
        }
        return { success: false, error: errorMsg.slice(0, 80) }
      }
    }

    const results = await runConcurrent(
      kiroAccounts,
      importOne,
      (completed: number) => setKiroProgress({ current: completed, total: kiroAccounts.length })
    )

    results.forEach(r => {
      if (r.success) {
        if (r.isNew) {
          added.push({ email: r.email, account: r.account })
        } else {
          updated.push({ email: r.email, account: r.account })
        }
      } else {
        failed.push({ error: r.error })
      }
    })

    setKiroResult({ added, updated, failed })
    setKiroImporting(false)

    if (added.length > 0 || updated.length > 0) {
      onSuccess?.({ added, updated })
    }
  }

  const handleKiroCliImport = async () => {
    if (!kiroCliDbPath) return

    setKiroCliImporting(true)
    setKiroCliResult(null)

    try {
      const result = await invoke<any>('import_from_kiro_cli', {
        dbPath: kiroCliDbPath
      })

      if (result.success) {
        setKiroCliResult({
          success: true,
          isNew: result.is_new,
          email: result.account?.email || result.account?.userId || '未知账号'
        })

        onSuccess?.({
          added: result.is_new ? [{ email: result.account?.email || result.account?.userId || '未知账号', account: result.account }] : [],
          updated: result.is_new ? [] : [{ email: result.account?.email || result.account?.userId || '未知账号', account: result.account }]})
      } else {
        setKiroCliResult({
          success: false,
          error: result.error || '导入失败'
        })
      }
    } catch (e) {
      setKiroCliResult({
        success: false,
        error: String(e)
      })
    } finally {
      setKiroCliImporting(false)
    }
  }

  return {
    activeTab, setActiveTab,
    osType, isWindowsOs,
    jsonText, setJsonText,
    parseResult, parseJson,
    handleFileSelect,
    importing, importProgress, importResult,
    kiroAccounts, kiroLoading, kiroError, kiroImporting, kiroProgress, kiroResult, detectKiroAccounts,
    kiroCliDbPath, setKiroCliDbPath, kiroCliDetected, setKiroCliDetected, kiroCliDetecting, kiroCliImporting, kiroCliResult,
    handleJsonImport, handleKiroImport, handleKiroCliImport,
  }
}
