import { TabsContent } from '@/components/ui/tabs'
import { Stack, Group } from '@/components/shared/layout'
import { Alert } from '@/components/ui/alert'
import { FileJson, AlertCircle, CheckCircle } from 'lucide-react'
import { LegacyButton, FileButton } from './ImportControls'

export function ImportJsonTab({ jsonText, setJsonText, parseJson, handleFileSelect, parseResult, colors, t }: any) {
  return (
            <TabsContent value="json" className="px-6 pb-4 pt-4 outline-none">
              <Stack gap="lg">
                <Group>
                  <FileButton onChange={handleFileSelect} accept=".json">
                    {(props: any) => <LegacyButton {...props} leftSection={<FileJson size={16} />}>{t('import.selectFile')}</LegacyButton>}
                  </FileButton>
                  <LegacyButton
                    color="blue"
                    size="sm"
                    onClick={() => {
                      try {
                        const parsed = JSON.parse(jsonText)
                        const formatted = JSON.stringify(parsed, null, 2)
                        setJsonText(formatted)
                        parseJson(formatted)
                      } catch {
                        // 如果解析失败，尝试格式化为数组
                        try {
                          const text = jsonText.trim()
                          if (text && !text.startsWith('[')) {
                            const formatted = JSON.stringify([JSON.parse(text)], null, 2)
                            setJsonText(formatted)
                            parseJson(formatted)
                          }
                        } catch {}
                      }
                    }}
                  >
                    格式化
                  </LegacyButton>
                  <LegacyButton color="blue" size="sm" onClick={() => { const text = JSON.stringify([{ refreshToken: "", provider: "Google" }], null, 2); setJsonText(text); parseJson(text) }}>
                    Social 模板
                  </LegacyButton>
                  <LegacyButton color="violet" size="sm" onClick={() => { const text = JSON.stringify([{ refreshToken: "", clientId: "", clientSecret: "", provider: "BuilderId" }], null, 2); setJsonText(text); parseJson(text) }}>
                    BuilderId 模板
                  </LegacyButton>
                  <LegacyButton color="grape" size="sm" onClick={() => { const text = JSON.stringify([{ refreshToken: "", clientId: "", clientSecret: "", provider: "Enterprise" }], null, 2); setJsonText(text); parseJson(text) }}>
                    Enterprise 模板
                  </LegacyButton>
                </Group>

                <textarea
                  value={jsonText}
                  onChange={(e) => { setJsonText(e.target.value); parseJson(e.target.value) }}
                  rows={10}
                  placeholder={`[{"refreshToken": "aor...", "provider": "Google"}]`}
                  className={`w-full p-4 rounded-xl text-foreground bg-background border border-input ${colors.inputFocus} font-mono text-sm outline-none resize-none`}
                />

                {parseResult && (
                  <Stack gap="xs">
                    {parseResult.valid.length > 0 && (
                      <Alert variant="success">
                        <CheckCircle size={16} />
                        {t('import.parseSuccess')}: {parseResult.valid.length} {t('import.validRecords')}
                      </Alert>
                    )}
                    {parseResult.errors.length > 0 && (
                      <Alert variant="destructive">
                        <AlertCircle size={16} />
                        <div className={`text-sm font-medium text-foreground`}>{t('import.validationError')}</div>
                        <Stack gap={2} mt="xs">
                          {parseResult.errors.slice(0, 5).map((err: string, i: number) => (
                            <div key={i} className={`text-xs text-foreground`}>{err}</div>
                          ))}
                          {parseResult.errors.length > 5 && (
                            <div className={`text-xs text-foreground`}>{t('import.moreErrors', { count: parseResult.errors.length - 5 })}</div>
                          )}
                        </Stack>
                      </Alert>
                    )}
                  </Stack>
                )}
              </Stack>
            </TabsContent>
  )
}
