import { useState } from 'react'
import { FileText, X } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { getGradientAccentButton } from './themeAccent'

// 创建弹窗组件
export function CreateModal({ inclusionOptions, onCreate, onClose, accent, colors, t, hasProjectDir }: any) {
  const accentGradientButtonClass = getGradientAccentButton(accent)
  const [fileName, setFileName] = useState('')
  const [inclusion, setInclusion] = useState('always')
  const [filePattern, setFilePattern] = useState('')
  const [scope, setScope] = useState('user')
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4" onClick={onClose}>
      <div
        className={`glass-card rounded-2xl w-full max-w-[380px] shadow-2xl border border-border overflow-hidden`}
        onClick={e => e.stopPropagation()}
        style={{ animation: 'dialogIn 0.2s ease-out' }}
      >
        <div className={`flex items-center justify-between px-5 py-4 ${colors.dialogHeader}`}>
          <div className="flex items-center gap-3">
            <div className={`w-10 h-10 rounded-xl ${colors.info} flex items-center justify-center`}>
              <FileText size={20} className={accent.text} />
            </div>
            <h2 className={`text-base font-semibold text-foreground`}>{t('steering.newSteering')}</h2>
          </div>
          <button onClick={onClose} className={`p-1.5 rounded-lg transition-colors hover:bg-muted/50 cursor-pointer`}>
            <X size={18} className={"text-muted-foreground"} />
          </button>
        </div>

        <div className="p-5 space-y-4">
          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('steering.fileName')}</label>
            <Input
              placeholder={t('steering.fileNamePlaceholder')}
              value={fileName}
              onChange={(e) => setFileName(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
            <p className={`text-xs text-muted-foreground mt-1`}>{t('steering.fileNameHint')}</p>
          </div>

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('steering.fmName')}</label>
            <Input
              placeholder={t('steering.fmNamePlaceholder')}
              value={name}
              onChange={(e) => setName(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
          </div>

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('steering.fmDescription')}</label>
            <Input
              placeholder={t('steering.fmDescriptionPlaceholder')}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ borderRadius: '0.5rem' }}
            />
          </div>

          {hasProjectDir && (
            <div>
              <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('kiroConfig.scope')}</label>
              <Select value={scope} onValueChange={setScope}>
                <SelectTrigger className={`text-foreground bg-background border-input ${colors.inputFocus}`} style={{ borderRadius: '0.5rem' }}>
                  <SelectValue placeholder={t('kiroConfig.scopeUser')} />
                </SelectTrigger>
                <SelectContent className={`glass-card border border-border`}>
                  <SelectItem value="user" className={"text-foreground"}>{t('kiroConfig.scopeUser')}</SelectItem>
                  <SelectItem value="project" className={"text-foreground"}>{t('kiroConfig.scopeProject')}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          )}

          <div>
            <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('steering.inclusionMode')}</label>
            <Select value={inclusion} onValueChange={setInclusion}>
              <SelectTrigger className={`text-foreground bg-background border-input ${colors.inputFocus}`} style={{ borderRadius: '0.5rem' }}>
                <SelectValue placeholder="选择模式" />
              </SelectTrigger>
              <SelectContent className={`glass-card border border-border`}>
                {inclusionOptions.map((opt: any) => (
                  <SelectItem key={opt.value} value={opt.value} className={"text-foreground"}>
                    {opt.label} - {opt.desc}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {inclusion === 'fileMatch' && (
            <div>
              <label className={`block text-xs font-medium text-muted-foreground mb-1.5`}>{t('steering.filePattern')}</label>
              <Input
                placeholder={t('steering.filePatternPlaceholder')}
                value={filePattern}
                onChange={(e) => setFilePattern(e.target.value)}
                className={`text-foreground bg-background border-input ${colors.inputFocus}`}
                style={{ borderRadius: '0.5rem' }}
              />
            </div>
          )}

          <button
            onClick={() => onCreate(fileName, inclusion, filePattern, scope, name.trim(), description.trim())}
            disabled={!fileName.trim()}
            className={`cursor-pointer w-full px-4 py-3 rounded-xl text-sm font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.98] ${accentGradientButtonClass}`}
          >
            {t('common.add')}
          </button>
        </div>
      </div>
    </div>
  )
}
