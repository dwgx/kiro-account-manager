import { TabsContent } from '@/components/ui/tabs'
import { Stack } from '@/components/shared/layout'
import { Alert } from '@/components/ui/alert'
import { AlertCircle, CheckCircle, Loader2, RefreshCw } from 'lucide-react'
import { getAccountDisplayName } from '../../../utils/accountStats'
import { getProviderDisplayName } from '../../../utils/accountProvider'
import { LegacyButton } from './ImportControls'

export function ImportKiroTab({ kiroLoading, kiroError, kiroAccounts, detectKiroAccounts, accent }: any) {
  return (
            <TabsContent value="kiro" className="px-6 pb-4 pt-4 outline-none">
              <Stack gap="lg">
                <Alert variant="info">
                  <div className={`text-sm font-medium text-foreground`}>从 Kiro IDE 导入账号</div>
                  <div className={`text-xs mt-1 text-muted-foreground`}>
                    自动读取 Kiro IDE 缓存的账号信息（~/.aws/sso/cache/kiro-auth-token.json）
                  </div>
                </Alert>

                {kiroLoading ? (
                  <div className={`p-5 rounded-xl bg-muted/30 border border-border`}>
                    <div className="flex items-center gap-3">
                      <Loader2 size={20} className={`animate-spin ${accent.text}`} />
                      <div className={`text-sm text-foreground`}>正在检测 Kiro IDE 账号...</div>
                    </div>
                  </div>
                ) : kiroError ? (
                  <Alert variant="destructive">
                    <AlertCircle size={16} />
                    <div className={`text-sm font-medium text-foreground`}>检测失败</div>
                    <div className={`text-xs mt-1 text-muted-foreground`}>{kiroError}</div>
                    <LegacyButton
                      color="red"
                      size="xs"
                      className="mt-3"
                      leftSection={<RefreshCw size={14} />}
                      onClick={detectKiroAccounts}
                    >
                      重新检测
                    </LegacyButton>
                  </Alert>
                ) : kiroAccounts.length > 0 ? (
                  <>
                    <Alert variant="success">
                      <CheckCircle size={16} />
                      <div className={`text-sm font-medium text-foreground`}>检测到 {kiroAccounts.length} 个账号</div>
                    </Alert>

                    <div className={`p-4 rounded-xl bg-muted/30 border border-border max-h-[240px] overflow-y-auto`}>
                      <Stack gap="sm">
                        {kiroAccounts.map((account: any, index: number) => (
                          <div key={index} className={`p-3 rounded-lg glass-card border border-border`}>
                            <div className="flex items-center justify-between">
                              <div>
                                <div className={`text-sm font-medium text-foreground`}>
                                  {getProviderDisplayName(account.provider)} ({account.authMethod})
                                </div>
                                <div className={`text-xs text-muted-foreground`}>
                                  {getAccountDisplayName(account)}
                                </div>
                              </div>
                              <div className={`px-2 py-1 rounded text-xs info-badge`}>
                                {account.authMethod === 'IdC' ? 'IdC' : 'Social'}
                              </div>
                            </div>
                          </div>
                        ))}
                      </Stack>
                    </div>
                  </>
                ) : (
                  <Alert variant="default">
                    <AlertCircle size={16} />
                    <div className={`text-sm text-foreground`}>未检测到 Kiro IDE 账号</div>
                    <div className={`text-xs mt-1 text-muted-foreground`}>
                      请先在 Kiro IDE 中登录账号
                    </div>
                  </Alert>
                )}
              </Stack>
            </TabsContent>
  )
}
