import { lazy, Suspense } from 'react'
import { Terminal, MonitorSmartphone } from 'lucide-react'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import IdeSessionManager from './IdeSessionManager'

const CliSessionManager = lazy(() => import('../CliSessionManager/index'))

export default function SessionManager() {
  return (
    <Tabs defaultValue="ide" className="flex flex-col h-full">
      <TabsList className="glass-card mb-3 flex h-9 w-fit justify-start rounded-lg border-none p-0.5 no-scrollbar">
        <TabsTrigger value="ide" className="gap-1.5 px-3 h-8 shrink-0 text-xs font-medium data-[state=active]:shadow-sm">
          <MonitorSmartphone size={13} />
          Kiro IDE
        </TabsTrigger>
        <TabsTrigger value="cli" className="gap-1.5 px-3 h-8 shrink-0 text-xs font-medium data-[state=active]:shadow-sm">
          <Terminal size={13} />
          Kiro CLI
        </TabsTrigger>
      </TabsList>

      <TabsContent value="ide" className="flex-1 min-h-0 mt-0">
        <IdeSessionManager />
      </TabsContent>
      <TabsContent value="cli" className="flex-1 min-h-0 mt-0">
        <Suspense fallback={<div className="flex items-center justify-center h-full text-muted-foreground text-sm">加载中...</div>}>
          <CliSessionManager />
        </Suspense>
      </TabsContent>
    </Tabs>
  )
}