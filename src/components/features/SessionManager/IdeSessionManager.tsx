import { Badge } from '@/components/ui/badge'
import { MessageSquare } from 'lucide-react'
import { useIdeSessions } from './hooks/useIdeSessions'
import WorkspaceSidebar from './WorkspaceSidebar'
import SessionDetailPanel from './SessionDetailPanel'

export default function IdeSessionManager() {
  const s = useIdeSessions()

  return (
    <div className="flex h-full flex-col overflow-hidden bg-gradient-to-br from-background via-background to-muted/40">
      {/* Header */}
      <div className="border-b border-border/70 bg-card/70 px-6 py-4 shadow-sm backdrop-blur-xl">
        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <div className="relative flex h-11 w-11 items-center justify-center rounded-2xl bg-gradient-to-br from-primary via-primary/90 to-primary/70 shadow-lg shadow-primary/20 ring-1 ring-primary/25">
              <div className="absolute inset-0 rounded-2xl bg-white/10" />
              <MessageSquare size={21} className="relative text-primary-foreground" />
            </div>
            <div className="flex flex-col">
              <h1 className="text-xl font-semibold tracking-tight text-foreground">会话管理</h1>
              <p className="text-sm text-muted-foreground">浏览、搜索和导出 Kiro IDE 的历史对话</p>
            </div>
          </div>

          <div className="hidden items-center gap-2 md:flex">
            <Badge variant="secondary" className="h-8 rounded-full px-3 font-normal">
              {s.workspaces.length} 个工作区
            </Badge>
            <Badge variant="outline" className="h-8 rounded-full px-3 font-normal bg-background/70">
              已加载 {s.loadedSessionCount} 个会话
            </Badge>
            {s.selectedWorkspaceHashes.size > 0 && (
              <Badge variant="destructive" className="h-8 rounded-full px-3 font-normal">
                已选 {s.selectedWorkspaceHashes.size}
              </Badge>
            )}
          </div>
        </div>
      </div>

      <div className="flex min-h-0 flex-1 overflow-hidden p-4 gap-4">
        {/* Left Sidebar - Workspaces with expandable sessions */}
        <WorkspaceSidebar {...s} />

        {/* Right Panel - Session Detail */}
        <SessionDetailPanel
          loading={s.loading}
          selectedSession={s.selectedSession}
          handleExportSession={s.handleExportSession}
        />
      </div>
    </div>
  )
}
