import { createContext, useContext, useState, useEffect, ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'

export interface AppSettings {
  lockModel: boolean;
  lockedModel: string | null;
  autoRefresh: boolean;
  autoRefreshInterval: number;
  browserPath: string;
  privacyMode: boolean;
  autoSwitchEnabled: boolean;
  autoSwitchThreshold: number;
  autoSwitchInterval: number;
  switchTarget: 'ide' | 'cli' | 'both';
  enableCodebaseIndexing: boolean;
  enableTabAutocomplete: boolean;
  usageSummary: boolean;
  enableDebugLogs: boolean;
  notifyActionRequired: boolean;
  notifyFailure: boolean;
  notifySuccess: boolean;
  notifyBilling: boolean;
  trustedTools: string[];
  referenceTracker: boolean;
  configureMcp: 'Enabled' | 'Disabled' | string;
  telemetryContentCollection: boolean;
  telemetryUsageAnalytics: boolean;
  telemetryEditStats: boolean;
  telemetryFeedback: boolean;
  appProxyMode: 'followKiro' | 'disabled' | string;
}

interface AppSettingsContextValue {
  settings: AppSettings | null;
  loading: boolean;
  updateSettings: (updates: Partial<AppSettings>) => Promise<AppSettings | null>;
  reload: () => Promise<void>;
}

const AppSettingsContext = createContext<AppSettingsContextValue | null>(null)

// 默认设置
const DEFAULT_SETTINGS: AppSettings = {
  lockModel: false,
  lockedModel: null,
  autoRefresh: true,
  autoRefreshInterval: 50,
  browserPath: '',
  privacyMode: true,
  autoSwitchEnabled: false,
  autoSwitchThreshold: 1,
  autoSwitchInterval: 5,
  switchTarget: 'ide',
  enableCodebaseIndexing: true,
  enableTabAutocomplete: true,
  usageSummary: true,
  enableDebugLogs: false,
  notifyActionRequired: true,
  notifyFailure: true,
  notifySuccess: true,
  notifyBilling: true,
  trustedTools: [],
  referenceTracker: false,
  configureMcp: 'Enabled',
  telemetryContentCollection: false,
  telemetryUsageAnalytics: false,
  telemetryEditStats: false,
  telemetryFeedback: false,
  appProxyMode: 'followKiro'
}

export function AppSettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [loading, setLoading] = useState(true)

  // 加载设置
  const loadSettings = async () => {
    try {
      const appSettings = await invoke<AppSettings>('get_app_settings')
      setSettings(appSettings || DEFAULT_SETTINGS)
    } catch (err) {
      console.error('[AppSettings] 加载失败:', err)
      setSettings(DEFAULT_SETTINGS)
    } finally {
      setLoading(false)
    }
  }

  // 更新设置
  const updateSettings = async (updates: Partial<AppSettings>) => {
    try {
      await invoke('save_app_settings', { settings: updates })
      // 先同步算出合并后的完整值再 setState：以前在异步更新器回调里给 nextSettings
      // 赋值，但 return 早于回调执行，永远返回 null，导致所有走此函数的开关都误报
      // "保存失败"（尽管后端已保存成功）。
      const nextSettings: AppSettings = { ...(settings || DEFAULT_SETTINGS), ...updates }
      setSettings(nextSettings)
      return nextSettings
    } catch (err) {
      console.error('[AppSettings] 保存失败:', err)
      return null
    }
  }

  useEffect(() => {
    loadSettings()

    let unlisten: UnlistenFn | null = null

    const setupListener = async () => {
      unlisten = await listen<AppSettings | null>('app-settings-changed', (event) => {
        if (event.payload) {
          setSettings(event.payload)
        } else {
          loadSettings()
        }
      })
    }

    setupListener()

    return () => {
      if (unlisten) unlisten()
    }
  }, [])

  return (
    <AppSettingsContext.Provider value={{ settings, loading, updateSettings, reload: loadSettings }}>
      {children}
    </AppSettingsContext.Provider>
  )
}

export function useAppSettings() {
  const context = useContext(AppSettingsContext)
  if (context === null) {
    throw new Error('useAppSettings must be used within AppSettingsProvider')
  }
  return context
}
