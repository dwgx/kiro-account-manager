import { useState, useEffect, useMemo, useCallback } from 'react'
import { Users, Zap, Shield, TrendingUp, Sparkles, Server, RefreshCw, ArrowRightLeft, Terminal } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { useApp } from '../../../hooks/useApp'
import { useDialog } from '../../../contexts/DialogContext'
import { showSuccess } from '../../../utils/toast'
import { useAccount } from '../../../contexts/AccountContext'
import { usePrivacy } from '../../../contexts/PrivacyContext'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import { Card, CardContent } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'

// 子组件
import LoadingSkeleton from './LoadingSkeleton'
import StatCard from './StatCard'
import CliAccountDetail from './CliAccountDetail'
import CurrentAccountDetail from './CurrentAccountDetail'

interface HomeProps {
  onNavigate: (path: string) => void;
}

function Home({ onNavigate }: HomeProps) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])

  const { showError } = useDialog()
  const { maskEmail } = usePrivacy()
  const {
    accounts: tokens,
    localToken,
    loading,
    refreshing,
    stats,
    currentAccount,
    currentQuotaInfo,
    refresh,
    refreshAccount,
  } = useAccount()
  
  const [refreshingAccount, setRefreshingAccount] = useState(false)
  const [switching, setSwitching] = useState(false)
  const [mcpToolCount, setMcpToolCount] = useState(0)
  const [ideInstallInfo, setIdeInstallInfo] = useState<any>(null)

  const handleRefresh = useCallback(() => refresh(), [refresh])

  // 检测 IDE 安装状态
  useEffect(() => {
    const checkIdeInstallation = async () => {
      try {
        const info = await invoke<any>('check_ide_installation')
        setIdeInstallInfo(info)
      } catch (e) {
        console.error('检测 IDE 安装状态失败:', e)
      }
    }
    checkIdeInstallation()
  }, [])

  // 加载 MCP 工具数量
  useEffect(() => {
    const loadMcpToolCount = async () => {
      try {
        const statsResult = await invoke<any>('get_mcp_tool_stats', { projectDir: null })
        setMcpToolCount(statsResult.estimatedTools)
      } catch (e) {
        // 静默处理
      }
    }
    loadMcpToolCount()
  }, [])

  // 监听自动切号事件，弹气泡提醒
  useEffect(() => {
    let unlisten: (() => void) | null = null
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen<{ email: string }>('account-switched', (event) => {
        showSuccess(`自动切号：已切换到 ${event.payload?.email || '新账号'}`)
        refresh()
      }).then(fn => { unlisten = fn })
    })
    return () => { unlisten?.() }
  }, [refresh])

  // 刷新当前账号
  const handleRefreshCurrentAccount = useCallback(async () => {
    if (!currentAccount || refreshingAccount) return
    setRefreshingAccount(true)
    try {
      await refreshAccount(currentAccount.id)
    } catch (e) {
      showError(t('common.refreshFailed'), String(e))
    } finally {
      setRefreshingAccount(false)
    }
  }, [currentAccount, refreshingAccount, refreshAccount, showError, t])

  // 一键换号
  const handleQuickSwitch = useCallback(async () => {
    if (switching) return
    setSwitching(true)
    try {
      const email = await invoke<string>('quick_switch_next')
      showSuccess(`已切换到 ${email}`)
      refresh()
    } catch (e: any) {
      showError('切换失败', String(e))
    } finally {
      setSwitching(false)
    }
  }, [switching, refresh, showError])

  // CLI 账号数据
  const [cliSnapshot, setCliSnapshot] = useState<any>(null)
  const [cliLoading, setCliLoading] = useState(false)
  const [cliPath, setCliPath] = useState('')
  const [cliInstalled, setCliInstalled] = useState(false)

  // 加载 CLI 账号
  useEffect(() => {
    const loadCliData = async () => {
      setCliLoading(true)
      try {
        const info = await invoke<any>('check_cli_installation')
        // 只根据可执行文件是否存在判断 CLI 是否安装
        setCliInstalled(info?.cli_installed || false)

        const path = await invoke<string>('get_kiro_cli_default_path')
        if (path) {
          setCliPath(path)
          try {
            const snapshot = await invoke<any>('read_cli_db_snapshot', { dbPath: path })
            setCliSnapshot(snapshot)
          } catch {
            // 数据库存在但读取失败，或未登录
          }
        }
      } catch (e) {
        // CLI 未安装
      } finally {
        setCliLoading(false)
      }
    }
    loadCliData()
  }, [])

  // 统计卡片
  const statCards = useMemo(() => [
    { icon: Users, iconBg: "info-badge", iconColor: accent.text, value: stats.total, label: t('home.totalAccounts'), delay: 'delay-100' },
    { icon: Shield, iconBg: "success-badge", iconColor: accent.text, value: `${stats.active}/${stats.unavailable}`, label: t('home.activeVsUnavailable'), delay: 'delay-200' },
    { icon: Zap, iconBg: "bg-purple-500/10 text-purple-500", iconColor: accent.text, value: stats.proPlus + stats.pro, label: t('home.proAccounts'), delay: 'delay-300' },
    { icon: TrendingUp, iconBg: "warning-badge", iconColor: 'text-orange-500', value: `${stats.usagePercent}%`, label: t('home.usagePercent'), delay: 'delay-400' },
    { 
      icon: Server, 
      iconBg: "bg-cyan-500/10 text-cyan-500", 
      iconColor: accent.text,
      value: mcpToolCount, 
      label: 'MCP 工具', 
      delay: 'delay-500',
      onClick: () => onNavigate?.('kiroConfig'),
      warning: mcpToolCount > 50
    },
  ], [accent, stats, mcpToolCount, t, onNavigate])

  if (loading) {
    return <LoadingSkeleton />
  }

  return (
    <div className="h-full overflow-auto glass-main p-6">
      <div className="w-full">
        {/* Header（紧凑）*/}
        <div className="mb-4 flex items-center gap-2.5 animate-slide-in-left">
          <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-md ring-1 ring-primary/20`}>
            <Sparkles size={20} className="text-white" />
          </div>
          <div className="flex flex-col">
            <h1 className="text-lg font-semibold text-foreground leading-tight">{t('home.title')}</h1>
            <p className="text-sm text-muted-foreground leading-tight">{t('home.subtitle')}</p>
          </div>
        </div>

        {/* 统计卡片 */}
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3 mb-3">
          {statCards.map((card, index) => (
            <StatCard key={index} {...card} />
          ))}
        </div>

        {/* 操作按钮 */}
        <div className="flex items-center gap-2 mb-3 animate-scale-in delay-200">
          <Button
            onClick={handleQuickSwitch}
            disabled={switching || tokens.length < 2}
            className="flex-1 h-11 text-sm font-semibold gap-2"
            variant="default"
          >
            <Zap size={16} className={switching ? 'animate-spin' : ''} />
            {switching ? '切换中...' : '一键换号'}
          </Button>
          <Button
            onClick={() => onNavigate?.('accounts')}
            variant="outline"
            className="flex-1 h-11 text-sm font-medium gap-2"
          >
            <ArrowRightLeft size={14} />
            查看全部账号
          </Button>
        </div>

        {/* 主卡片：当前账号 | CLI 账号 */}
        <Card className="card-glow animate-scale-in delay-300">
          <div className="flex items-center justify-between px-4 py-2.5 border-b border-border">
            <div className="flex items-center gap-2">
              <Sparkles size={14} className={accent.text} />
              <span className="text-sm font-semibold text-foreground">Kiro 账号</span>
            </div>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={handleRefreshCurrentAccount}
                    disabled={refreshingAccount || refreshing}
                    className={`h-7 w-7 ${refreshingAccount ? 'spinning' : ''}`}
                  >
                    <RefreshCw size={13} className="text-muted-foreground" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{t('common.refresh')}</TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>

          <CardContent className="p-0">
            <div className="grid grid-cols-1 md:grid-cols-[3fr_2fr]">
              {/* 左：当前 IDE 账号 */}
              <div className="p-4 flex flex-col gap-3">
                <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider">
                  当前 IDE 账号
                </span>
                {currentAccount ? (
                  <CurrentAccountDetail
                    account={currentAccount}
                    accent={accent}
                    maskEmail={maskEmail}
                    t={t}
                  />
                ) : (
                  <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm py-10">
                    {localToken ? '未匹配到账号' : (
                      ideInstallInfo?.ide_installed === false
                        ? (ideInstallInfo?.ide_executable_exists === false
                            ? 'Kiro IDE 未安装'
                            : 'Kiro IDE 已安装，未登录')
                        : t('home.notLoggedIn')
                    )}
                  </div>
                )}
              </div>

              {/* 右：CLI 账号 */}
              <div className="p-4 flex flex-col gap-3 bg-muted/20 border-t md:border-t-0 md:border-l border-border">
                <span className="text-[10px] font-bold uppercase text-muted-foreground tracking-wider flex items-center gap-1.5">
                  <Terminal size={11} />
                  当前 CLI 账号
                </span>
                {cliLoading ? (
                  <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
                    加载中...
                  </div>
                ) : cliSnapshot ? (
                  <CliAccountDetail snapshot={cliSnapshot} cliPath={cliPath} />
                ) : cliInstalled ? (
                  <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm flex-col gap-1.5 py-8">
                    <Terminal size={20} className="text-muted-foreground/50" />
                    <span>CLI 已安装，未登录</span>
                    <span className="text-[11px] text-muted-foreground/70">请运行 kiro-cli login 登录</span>
                  </div>
                ) : (
                  <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm flex-col gap-1.5 py-8">
                    <Terminal size={20} className="text-muted-foreground/50" />
                    <span>CLI 未安装</span>
                    <span className="text-[11px] text-muted-foreground/70">请安装 Kiro CLI 后重启</span>
                  </div>
                )}
              </div>
            </div>

          </CardContent>
        </Card>
      </div>
    </div>
  )
}

export default Home
