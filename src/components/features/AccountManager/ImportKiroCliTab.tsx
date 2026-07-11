import { TabsContent } from '@/components/ui/tabs'
import { Stack } from '@/components/shared/layout'
import { Alert } from '@/components/ui/alert'
import { AlertCircle, CheckCircle, Loader2 } from 'lucide-react'
import { LegacyButton, FileButton } from './ImportControls'

export function ImportKiroCliTab({ isWindowsOs, kiroCliDetecting, kiroCliDetected, kiroCliDbPath, setKiroCliDbPath, setKiroCliDetected, colors, accent, t }: any) {
  return (
            <TabsContent value="kiro-cli" className="px-6 pb-4 pt-4 outline-none">
              <Stack gap="lg">
                <Alert variant="info">
                  <div className={`text-sm font-medium text-foreground`}>{t('import.kiroCliTitle')}</div>
                  <div className={`text-xs mt-1 text-muted-foreground`}>
                    {t('import.kiroCliHint')}
                  </div>
                  <div className={`text-xs mt-1 text-muted-foreground`}>
                    {t('import.kiroCliInstallPrefix')} <code>{t('import.kiroCliInstallCommand')}</code>
                  </div>
                </Alert>

                {isWindowsOs && (
                  <Alert variant="info">
                    <AlertCircle size={16} />
                    <div className={`text-sm font-medium text-foreground`}>{t('import.kiroCliWindowsTitle')}</div>
                    <div className={`text-xs mt-1 text-muted-foreground`}>
                      {t('import.kiroCliWindowsHint')}
                    </div>
                  </Alert>
                )}

                {kiroCliDetecting ? (
                  <div className={`p-5 rounded-xl bg-muted/30 border border-border`}>
                    <div className="flex items-center gap-3">
                      <Loader2 size={20} className={`animate-spin ${accent.text}`} />
                      <div className={`text-sm text-foreground`}>{t('import.kiroCliDetecting')}</div>
                    </div>
                  </div>
                ) : kiroCliDetected ? (
                  <Alert variant="success">
                    <CheckCircle size={16} />
                    <div className={`text-sm font-medium text-foreground`}>{t('import.kiroCliDetected')}</div>
                    <div className={`text-xs mt-1 text-muted-foreground`}>{kiroCliDbPath}</div>
                  </Alert>
                ) : (
                  <Alert variant="default">
                    <AlertCircle size={16} />
                    <div className={`text-sm text-foreground`}>{t('import.kiroCliNotDetected')}</div>
                    <div className={`text-xs mt-1 text-muted-foreground`}>
                      {t('import.kiroCliPathHintManual')}
                    </div>
                  </Alert>
                )}

                <div className={`p-4 rounded-xl bg-muted/30 border border-border`}>
                  <Stack gap="md">
                    <div>
                      <label className={`text-sm font-medium text-foreground block mb-2`}>
                        {t('import.kiroCliPathLabel')}
                      </label>
                      <div className="flex gap-2">
                        <input
                          type="text"
                          value={kiroCliDbPath}
                          onChange={(e) => {
                            setKiroCliDbPath(e.target.value)
                            setKiroCliDetected(false)
                          }}
                          placeholder={t('import.kiroCliPathPlaceholder')}
                          className={`flex-1 px-4 py-3 border rounded-xl text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 transition-all outline-none`}
                        />
                        <FileButton
                          onChange={(file: any) => {
                            if (file) {
                              setKiroCliDbPath(file.path)
                              setKiroCliDetected(false)
                            }
                          }}
                          accept=".sqlite3,.db"
                        >
                          {(props: any) => (
                            <LegacyButton
                              {...props}
                              className="px-4"
                            >
                              浏览
                            </LegacyButton>
                          )}
                        </FileButton>
                      </div>
                    </div>
                  </Stack>
                </div>
              </Stack>
            </TabsContent>
  )
}
