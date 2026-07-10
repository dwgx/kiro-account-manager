import { SessionSummary } from '@/types/session'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Loader2,
  Search,
  Trash2,
  MessageSquare,
  ChevronRight,
  ChevronDown
} from 'lucide-react'
import { decodeWorkspaceName, formatFileSize } from './utils/sessionFormat'

interface WorkspaceSidebarProps {
  workspaces: string[]
  searchQuery: string
  setSearchQuery: React.Dispatch<React.SetStateAction<string>>
  selectedWorkspaceHashes: Set<string>
  toggleSelectAllWorkspaces: () => void
  toggleWorkspaceSelection: (workspaceHash: string) => void
  handleBatchDeleteWorkspaces: () => void
  filteredSessions: SessionSummary[]
  selectedSessionId: string | undefined
  selectedWorkspace: string | null
  setSelectedWorkspace: React.Dispatch<React.SetStateAction<string | null>>
  expandedWorkspaces: Set<string>
  getWorkspaceSessions: (workspaceHash: string) => SessionSummary[]
  toggleWorkspace: (workspaceHash: string) => void
  handleSelectSession: (workspaceHash: string, session: SessionSummary) => void
  handleDeleteSession: (workspaceHash: string, session: SessionSummary) => void
  handleDeleteWorkspace: (workspaceHash: string) => void
  loading: boolean
}

export default function WorkspaceSidebar({
  workspaces,
  searchQuery,
  setSearchQuery,
  selectedWorkspaceHashes,
  toggleSelectAllWorkspaces,
  toggleWorkspaceSelection,
  handleBatchDeleteWorkspaces,
  filteredSessions,
  selectedSessionId,
  selectedWorkspace,
  setSelectedWorkspace,
  expandedWorkspaces,
  getWorkspaceSessions,
  toggleWorkspace,
  handleSelectSession,
  handleDeleteSession,
  handleDeleteWorkspace,
  loading
}: WorkspaceSidebarProps) {
  return (
    <div className="w-80 shrink-0 overflow-hidden rounded-2xl border border-border/70 bg-card/80 shadow-sm backdrop-blur-xl flex flex-col">
      <div className="border-b border-border/70 bg-muted/20 p-4 space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-semibold text-foreground">工作区与会话</h2>
          {selectedWorkspaceHashes.size > 0 && (
            <Button
              variant="destructive"
              size="sm"
              className="h-7 rounded-full px-3 text-[11px]"
              onClick={handleBatchDeleteWorkspaces}
            >
              <Trash2 className="h-3 w-3 mr-1" />
              删除 ({selectedWorkspaceHashes.size})
            </Button>
          )}
        </div>
        <div className="flex items-center justify-between rounded-xl bg-background/70 px-3 py-2 text-[11px] text-muted-foreground ring-1 ring-border/50">
          <span>{workspaces.length} 个工作区</span>
          {workspaces.length > 0 && (
            <Button
              variant="ghost"
              size="sm"
              className="h-6 rounded-full px-2 text-[11px] hover:bg-primary/10 hover:text-primary"
              onClick={toggleSelectAllWorkspaces}
            >
              {selectedWorkspaceHashes.size === workspaces.length ? '取消全选' : '全选'}
            </Button>
          )}
        </div>
        {/* Search */}
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            placeholder="搜索会话..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="h-9 rounded-xl border-border/70 bg-background/80 pl-8 text-xs shadow-inner focus-visible:ring-primary/30"
          />
        </div>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-3 space-y-2">
          {/* 搜索模式：显示所有匹配的会话 */}
          {searchQuery && (
            <div className="space-y-2">
              {filteredSessions.length === 0 ? (
                <div className="rounded-2xl border border-dashed border-border/70 bg-muted/20 px-4 py-10 text-center text-sm text-muted-foreground">
                  未找到匹配的会话
                </div>
              ) : (
                filteredSessions.map(session => {
                  const isSelected = selectedSessionId === session.sessionId

                  return (
                  <Card
                    key={session.sessionId}
                    className={`group relative cursor-pointer overflow-hidden rounded-2xl p-3 shadow-sm transition-all hover:-translate-y-0.5 hover:border-primary/40 hover:bg-primary/5 hover:shadow-md ${isSelected ? 'border-primary/60 bg-primary/10 shadow-sm ring-1 ring-primary/20' : 'border-border/70 bg-card'}
                    `}
                    onClick={() => handleSelectSession(session.workspaceHash, session)}
                  >
                    {isSelected && (
                      <>
                        <div className="absolute inset-y-2 left-0 w-1 rounded-r-full bg-primary" />
                      </>
                    )}
                    <div className="space-y-2 pl-1">
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex-1 min-w-0">
                          <h3 className={`line-clamp-2 text-sm font-semibold leading-snug ${isSelected ? 'text-primary' : 'text-foreground'}`}>
                            {session.title}
                          </h3>
                          <p className="mt-1 truncate text-xs text-muted-foreground">
                            {decodeWorkspaceName(session.workspaceHash)}
                          </p>
                        </div>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-7 w-7 shrink-0 rounded-lg text-muted-foreground opacity-0 transition-opacity hover:bg-destructive hover:text-destructive-foreground group-hover:opacity-100"
                          onClick={(e) => {
                            e.stopPropagation()
                            handleDeleteSession(session.workspaceHash, session)
                          }}
                          title="删除会话"
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                      <div className="flex items-center gap-2 flex-wrap">
                        <Badge variant="secondary" className="h-5 rounded-full px-2 text-[10px] font-normal">
                          {session.sessionType}
                        </Badge>
                        <span className="flex items-center gap-1 text-xs text-muted-foreground">
                          <MessageSquare className="h-3 w-3" />
                          {session.messageCount}
                        </span>
                        <span className="text-xs text-muted-foreground">
                          {formatFileSize(session.fileSize)}
                        </span>
                      </div>
                    </div>
                  </Card>
                  )
                })
              )}
            </div>
          )}
          {/* 正常模式：显示工作区树 */}
          {!searchQuery && workspaces.map(workspace => {
            const isExpanded = expandedWorkspaces.has(workspace)
            const sessions = getWorkspaceSessions(workspace)

            return (
              <div key={workspace} className="space-y-1">
                {/* Workspace Row */}
                <div
                  className={`group relative overflow-hidden rounded-xl border transition-all ${selectedWorkspace === workspace
                      ? 'border-primary/50 bg-gradient-to-r from-primary/14 to-primary/5 shadow-sm ring-1 ring-primary/20'
                      : 'border-transparent hover:border-border/70 hover:bg-muted/40'
                    }`}
                >
                  <div className="flex items-center gap-2 px-2.5 py-2.5">
                    {/* Expand/Collapse Icon */}
                    <button
                      onClick={() => toggleWorkspace(workspace)}
                      className="shrink-0 rounded-lg p-1 text-muted-foreground transition-colors hover:bg-background hover:text-foreground"
                      title={isExpanded ? '折叠' : '展开'}
                    >
                      {isExpanded ? (
                        <ChevronDown className="h-4 w-4" />
                      ) : (
                        <ChevronRight className="h-4 w-4" />
                      )}
                    </button>

                    {/* Checkbox */}
                    <Checkbox
                      checked={selectedWorkspaceHashes.has(workspace)}
                      onCheckedChange={(checked) => {
                        toggleWorkspaceSelection(workspace)
                      }}
                      onClick={(e) => e.stopPropagation()}
                      className="shrink-0 cursor-pointer border-foreground/40 data-[state=checked]:border-primary data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground"
                    />

                    {/* Workspace Name */}
                    <button
                      onClick={() => {
                        setSelectedWorkspace(workspace)
                        toggleWorkspace(workspace)
                      }}
                      className="flex-1 rounded-lg px-2 py-1 text-left text-sm transition-colors hover:bg-background/70"
                      title={workspace}
                    >
                      <div className="truncate font-medium text-foreground">
                        {decodeWorkspaceName(workspace)}
                      </div>
                      {isExpanded && sessions.length > 0 && (
                        <div className="mt-0.5 text-xs text-muted-foreground">
                          {sessions.length} 个会话
                        </div>
                      )}
                    </button>

                    {/* Delete Button */}
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 shrink-0 rounded-lg text-muted-foreground opacity-0 transition-opacity hover:bg-destructive hover:text-destructive-foreground group-hover:opacity-100"
                      onClick={(e) => {
                        e.stopPropagation()
                        handleDeleteWorkspace(workspace)
                      }}
                      title="删除工作区"
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                </div>

                {/* Sessions under this workspace (when expanded) */}
                {isExpanded && (
                  <div className="ml-7 space-y-1.5 border-l border-border/70 pl-3">
                    {loading && sessions.length === 0 ? (
                      <div className="flex items-center justify-center py-4">
                        <Loader2 className="h-4 w-4 animate-spin" />
                      </div>
                    ) : sessions.length === 0 ? (
                      <div className="rounded-xl bg-muted/30 px-3 py-3 text-xs text-muted-foreground">
                        暂无会话
                      </div>
                    ) : (
                      sessions.map(session => {
                        const isSelected = selectedSessionId === session.sessionId

                        return (
                        <Card
                          key={session.sessionId}
                          className={`group relative cursor-pointer overflow-hidden rounded-2xl p-2.5 transition-all hover:border-primary/40 hover:bg-primary/5 ${isSelected ? 'border-primary/60 bg-primary/10 shadow-sm ring-1 ring-primary/20' : 'border-border/60 bg-card'}
                          `}
                          onClick={() => handleSelectSession(workspace, session)}
                        >
                          {isSelected && (
                            <>
                              <div className="absolute inset-y-1.5 left-0 w-1 rounded-r-full bg-primary" />
                            </>
                          )}
                          <div className="space-y-1.5 pl-1">
                            <div className="flex items-start justify-between gap-2">
                              <h3 className={`line-clamp-2 flex-1 text-xs font-semibold leading-snug ${isSelected ? 'text-primary' : 'text-foreground'}`}>
                                {session.title}
                              </h3>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-6 w-6 shrink-0 rounded-lg text-muted-foreground opacity-0 transition-opacity hover:bg-destructive hover:text-destructive-foreground group-hover:opacity-100"
                                onClick={(e) => {
                                  e.stopPropagation()
                                  handleDeleteSession(workspace, session)
                                }}
                                title="删除会话"
                              >
                                <Trash2 className="h-2.5 w-2.5" />
                              </Button>
                            </div>
                            <div className="flex items-center gap-2 flex-wrap">
                              <Badge variant="secondary" className="h-4 rounded-full px-1.5 text-[10px] font-normal">
                                {session.sessionType}
                              </Badge>
                              <span className="flex items-center gap-1 text-xs text-muted-foreground">
                                <MessageSquare className="h-2.5 w-2.5" />
                                {session.messageCount}
                              </span>
                            </div>
                          </div>
                        </Card>
                        )
                      })
                    )}
                  </div>
                )}
              </div>
            )
          })}
        </div>
      </ScrollArea>
    </div>
  )
}
