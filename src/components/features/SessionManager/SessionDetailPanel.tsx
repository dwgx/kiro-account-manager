import { IdeSession } from '@/types/session'
import { Card } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Loader2, Download, MessageSquare } from 'lucide-react'

interface SessionDetailPanelProps {
  loading: boolean
  selectedSession: IdeSession | null
  handleExportSession: (format: 'json' | 'markdown') => void
}

export default function SessionDetailPanel({
  loading,
  selectedSession,
  handleExportSession
}: SessionDetailPanelProps) {
  return (
    <div className="min-w-0 flex-1 overflow-hidden rounded-2xl border border-border/70 bg-card/70 shadow-sm backdrop-blur-xl flex flex-col">
      {loading && selectedSession === null ? (
        <div className="flex-1 flex items-center justify-center text-muted-foreground">
          <div className="flex items-center gap-3 rounded-2xl border border-border/70 bg-background/80 px-5 py-4 shadow-sm">
            <Loader2 className="h-5 w-5 animate-spin text-primary" />
            <span className="text-sm">正在加载会话...</span>
          </div>
        </div>
      ) : selectedSession ? (
        <>
          <div className="border-b border-border/70 bg-background/75 px-5 py-3.5 shadow-sm flex items-center justify-between">
            <div className="flex-1 min-w-0">
              <h2 className="truncate text-base font-semibold tracking-tight text-foreground">{selectedSession.title}</h2>
              <p className="text-[11px] text-muted-foreground mt-0.5 truncate font-mono">
                {selectedSession.workspaceDirectory}
              </p>
            </div>
            <div className="flex gap-1.5 ml-3">
              <Button
                variant="outline"
                size="sm"
                className="h-8 rounded-full px-3 text-xs"
                onClick={() => handleExportSession('json')}
              >
                <Download className="h-3.5 w-3.5 mr-1" />
                JSON
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-8 rounded-full px-3 text-xs"
                onClick={() => handleExportSession('markdown')}
              >
                <Download className="h-3.5 w-3.5 mr-1" />
                Markdown
              </Button>
            </div>
          </div>
          <ScrollArea className="flex-1">
            <div className="mx-auto max-w-5xl space-y-4 p-5">
              {/* Conversation Summary - 从第一条消息中提取 */}
              {selectedSession.history.length > 0 &&
                selectedSession.history[0].message.role === 'user' &&
                selectedSession.history[0].message.content.length > 0 &&
                (selectedSession.history[0].message.content[0].text.includes('CONTEXT TRANSFER') ||
                  selectedSession.history[0].message.content[0].text.includes('## TASK') ||
                  selectedSession.title.includes('(Continued)')) && (
                  <Card className="overflow-hidden rounded-2xl border-blue-200/80 bg-gradient-to-br from-blue-50 to-cyan-50 p-0 shadow-sm dark:border-blue-800/70 dark:from-blue-950/70 dark:to-cyan-950/40">
                    <div className="flex items-start gap-3 p-4">
                      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-blue-500/10 text-xl ring-1 ring-blue-500/20">📝</div>
                      <div className="min-w-0 flex-1">
                        <div className="mb-2 font-semibold text-blue-900 dark:text-blue-100">
                          对话摘要（上下文压缩）
                        </div>
                        <div className="max-h-72 overflow-auto whitespace-pre-wrap break-words rounded-xl bg-white/55 p-3 text-sm leading-6 text-blue-900 ring-1 ring-blue-200/60 dark:bg-black/20 dark:text-blue-100 dark:ring-blue-800/50">
                          {selectedSession.history[0].message.content[0].text}
                        </div>
                      </div>
                    </div>
                  </Card>
                )}

              {/* Messages */}
              {selectedSession.history.length === 0 ? (
                <div className="rounded-2xl border border-dashed border-border/70 bg-muted/20 px-6 py-14 text-center text-sm text-muted-foreground">
                  此会话没有消息
                </div>
              ) : (
                selectedSession.history.map((item, index) => {
                  // 跳过第一条摘要消息（如果是压缩会话）
                  const isSummaryMessage = index === 0 &&
                    item.message.role === 'user' &&
                    item.message.content.length > 0 &&
                    (item.message.content[0].text.includes('CONTEXT TRANSFER') ||
                      item.message.content[0].text.includes('## TASK') ||
                      selectedSession.title.includes('(Continued)'))

                  if (isSummaryMessage) {
                    return null
                  }

                  return (
                    <Card key={item.message.id} className={`overflow-hidden rounded-2xl border-border/70 p-0 shadow-sm ${item.message.role === 'user' ? 'bg-background' : 'bg-muted/25'}`}>
                      <div className="flex items-start gap-3 p-4">
                        <div className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl text-lg ring-1 ${item.message.role === 'user' ? 'bg-primary/10 ring-primary/20' : 'bg-emerald-500/10 ring-emerald-500/20'}`}>
                          {item.message.role === 'user' ? '👤' : '🤖'}
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="mb-2 flex items-center gap-2">
                            <span className="text-sm font-semibold text-foreground">
                              {item.message.role === 'user' ? 'User' : 'Assistant'}
                            </span>
                            <Badge variant="outline" className="h-5 rounded-full px-2 text-[10px] font-normal">
                              #{index + 1}
                            </Badge>
                          </div>
                          {item.message.content.map((content, i) => (
                            <div key={i} className="whitespace-pre-wrap break-words rounded-xl bg-background/70 p-3 text-sm leading-6 text-foreground/90 ring-1 ring-border/50">
                              {content.text}
                            </div>
                          ))}
                        </div>
                      </div>
                    </Card>
                  )
                })
              )}
            </div>
          </ScrollArea>
        </>
      ) : (
        <div className="flex-1 flex items-center justify-center p-8">
          <div className="max-w-sm rounded-3xl border border-dashed border-border/80 bg-background/70 px-8 py-10 text-center shadow-sm">
            <div className="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-primary/10 ring-1 ring-primary/20">
              <MessageSquare className="h-7 w-7 text-primary" />
            </div>
            <p className="font-medium text-foreground">选择一个会话查看详情</p>
            <p className="mt-2 text-sm text-muted-foreground">从左侧工作区树或搜索结果中选择历史对话。</p>
          </div>
        </div>
      )}
    </div>
  )
}
