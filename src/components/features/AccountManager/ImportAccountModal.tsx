import { useMemo } from 'react'
import { Tabs, TabsList } from '@/components/ui/tabs'

import { Progress } from '@/components/ui/progress'
import { Alert } from '@/components/ui/alert'
import { Upload, FileJson, AlertCircle, CheckCircle, Loader2, Database } from 'lucide-react'
import { useApp } from '../../../hooks/useApp'

import {
  DialogRoot,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody,
  DialogFooter} from '../../shared/dialog'
import { Button } from '../../shared/button'
import { getThemeAccent } from '../KiroConfig/themeAccent'
import { useImportAccount } from './hooks/useImportAccount'
import { ImportResultAlerts } from './ImportResultAlerts'
import { ImportJsonTab } from './ImportJsonTab'
import { ImportKiroTab } from './ImportKiroTab'
import { ImportKiroCliTab } from './ImportKiroCliTab'

interface ImportAccountModalProps {
  onClose: () => void;
  onSuccess?: (data: { added: any[]; updated: any[] }) => void;
  onNavigate?: (path: string) => void;
}

function ImportAccountModal({ onClose, onSuccess, onNavigate }: ImportAccountModalProps) {
  const { t, theme } = useApp()
  const accent = useMemo(() => getThemeAccent(theme), [theme])
  const colors = useMemo(() => ({
    inputFocus: 'focus:ring-primary/20 focus:border-primary',
    ringColor: theme === 'dark' ? 'ring-primary/40' : 'ring-primary/20'
  }), [theme])

  const {
    activeTab, setActiveTab,
    isWindowsOs,
    jsonText, setJsonText,
    parseResult, parseJson,
    handleFileSelect,
    importing, importProgress, importResult,
    kiroAccounts, kiroLoading, kiroError, kiroImporting, kiroProgress, kiroResult, detectKiroAccounts,
    kiroCliDbPath, setKiroCliDbPath, kiroCliDetected, setKiroCliDetected, kiroCliDetecting, kiroCliImporting, kiroCliResult,
    handleJsonImport, handleKiroImport, handleKiroCliImport,
  } = useImportAccount({ onSuccess })

  return (
  <DialogRoot open={true} onOpenChange={(open) => !open && onClose()}>
    <DialogContent maxWidth="700px">
      <DialogHeader>
        <div className="flex items-center gap-3">
          <div className={`w-10 h-10 rounded-xl bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-md ${accent.shadow}`}>
            <Upload size={20} className="text-white" strokeWidth={2} />
          </div>
          <div>
            <DialogTitle>{t('import.title')}</DialogTitle>
            <DialogDescription>{t('import.subtitle') || '批量导入账号数据'}</DialogDescription>
          </div>
        </div>
      </DialogHeader>

      <DialogBody noPadding>
        {importResult || kiroResult || kiroCliResult ? (
          <div className="px-6 py-4">
            {importResult && <ImportResultAlerts result={importResult} />}
            {kiroResult && <ImportResultAlerts result={kiroResult} />}
            {kiroCliResult && (
              <Alert variant={kiroCliResult.success ? "success" : "destructive"}>
                {kiroCliResult.success ? <CheckCircle size={16} /> : <AlertCircle size={16} />}
                <div className={`text-sm font-medium text-foreground`}>
                  {kiroCliResult.success
                    ? (kiroCliResult.isNew
                      ? `✅ 新增账号: ${kiroCliResult.email}`
                      : `📝 更新账号: ${kiroCliResult.email}`)
                    : '❌ 导入失败'}
                </div>
                {kiroCliResult.error && (
                  <div className={`text-xs mt-1 text-muted-foreground`}>{kiroCliResult.error}</div>
                )}
              </Alert>
            )}
          </div>
        ) : importing || kiroImporting || kiroCliImporting ? (
          <div className="px-6 py-6">
            <div className={`p-5 rounded-xl bg-muted/30 border border-border`}>
              <div className="flex items-center gap-4 mb-4">
                <div className={`w-10 h-10 rounded-xl flex items-center justify-center bg-muted/30`}>
                  <Loader2 size={20} className={"text-primary animate-spin"} />
                </div>
                <div>
                  <div className={`font-medium text-foreground`}>
                    {importing ? t('import.importing') : kiroImporting ? '正在从 Kiro 导入...' : '正在从 kiro-cli 导入...'}
                  </div>
                  <div className={`text-sm text-muted-foreground`}>
                    {kiroCliImporting ? '请稍候...' : `${(importing ? importProgress : kiroProgress).current}/${(importing ? importProgress : kiroProgress).total}`}
                  </div>
                </div>
              </div>
              <Progress
                value={((importing ? importProgress : kiroProgress).total > 0) ? ((importing ? importProgress : kiroProgress).current /
                  (importing ? importProgress : kiroProgress).total * 100) : 0}
                className="h-3 rounded-xl"
              />
            </div>
          </div>
        ) : (
          <Tabs value={activeTab} onValueChange={setActiveTab}>
            <TabsList className="px-6 pt-2 pb-3 border-b-0 bg-transparent h-auto">
              <div className={`grid grid-cols-4 gap-1 p-1 rounded-xl border border-border bg-muted/30 w-full`}>
                <button
                  onClick={() => setActiveTab('json')}
                  className={`py-2 px-3 text-sm rounded-lg transition-all duration-200 font-medium cursor-pointer ${activeTab === 'json'
                    ? `glass-card shadow-sm ring-1 ${colors.ringColor} text-foreground`
                    : `hover:bg-muted/50 text-muted-foreground`
                  }`}
                >
                  <div className="flex items-center justify-center gap-2">
                    <FileJson size={16} />
                    <span>{t('import.jsonTab')}</span>
                  </div>
                </button>
                <button
                  onClick={() => setActiveTab('kiro')}
                  className={`py-2 px-3 text-sm rounded-lg transition-all duration-200 font-medium cursor-pointer ${activeTab === 'kiro'
                    ? `glass-card shadow-sm ring-1 ${colors.ringColor} text-foreground`
                    : `hover:bg-muted/50 text-muted-foreground`
                  }`}
                >
                  <div className="flex items-center justify-center gap-2">
                    <Upload size={16} />
                    <span>{t('import.kiroTab')}</span>
                  </div>
                </button>
                <button
                  onClick={() => setActiveTab('kiro-cli')}
                  className={`py-2 px-3 text-sm rounded-lg transition-all duration-200 font-medium cursor-pointer ${activeTab === 'kiro-cli'
                    ? `glass-card shadow-sm ring-1 ${colors.ringColor} text-foreground`
                    : `hover:bg-muted/50 text-muted-foreground`
                  }`}
                >
                  <div className="flex items-center justify-center gap-2">
                    <Database size={16} />
                    <span>{t('import.kiroCliTab')}</span>
                  </div>
                </button>

              </div>
            </TabsList>
            <ImportJsonTab
              jsonText={jsonText}
              setJsonText={setJsonText}
              parseJson={parseJson}
              handleFileSelect={handleFileSelect}
              parseResult={parseResult}
              colors={colors}
              t={t}
            />

            <ImportKiroTab
              kiroLoading={kiroLoading}
              kiroError={kiroError}
              kiroAccounts={kiroAccounts}
              detectKiroAccounts={detectKiroAccounts}
              accent={accent}
            />

            <ImportKiroCliTab
              isWindowsOs={isWindowsOs}
              kiroCliDetecting={kiroCliDetecting}
              kiroCliDetected={kiroCliDetected}
              kiroCliDbPath={kiroCliDbPath}
              setKiroCliDbPath={setKiroCliDbPath}
              setKiroCliDetected={setKiroCliDetected}
              colors={colors}
              accent={accent}
              t={t}
            />


          </Tabs>
        )}
      </DialogBody>

      <DialogFooter>
        <Button variant="secondary" onClick={onClose} disabled={importing || kiroImporting || kiroCliImporting}>
          {importResult || kiroResult || kiroCliResult ? t('common.close') : t('common.cancel')}
        </Button>
        {!(importResult || kiroResult || kiroCliResult) && (
          <Button
            onClick={activeTab === 'json' ? handleJsonImport : activeTab === 'kiro' ? handleKiroImport : handleKiroCliImport}
            disabled={importing || kiroImporting || kiroCliImporting || (activeTab === 'json' && !parseResult?.valid.length) || (activeTab === 'kiro' && kiroAccounts.length === 0) || (activeTab === 'kiro-cli' && !kiroCliDbPath)}
            loading={importing || kiroImporting || kiroCliImporting}
          >
            {t('common.import')}
          </Button>
        )}
      </DialogFooter>
    </DialogContent>
  </DialogRoot>
  )
}

export default ImportAccountModal
