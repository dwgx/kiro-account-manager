import { Wand2, Save } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { getSolidAccentButton } from './themeAccent'
import { ScopeBadge } from './SteeringBadges'

// 编辑器组件
export function Editor({ file, editState, hasChanges, saving, refining, inclusionOptions, onContentChange, onInclusionChange, onFilePatternChange, onNameChange, onDescriptionChange, onSave, onRefine, surface, accent, colors, t }: any) {
  const accentSolidButtonClass = getSolidAccentButton(accent)
  return (
    <>
      <div className={`p-3 border-b border-border flex items-center justify-between`}>
        <div className="flex items-center gap-2">
          <h3 className={`font-semibold text-foreground`}>{file.fileName}</h3>
          <ScopeBadge scope={file.scope} accent={accent} />
          {hasChanges && <span className="text-xs text-orange-500">● {t('steering.save')}</span>}
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={onRefine}
            disabled={refining}
            className={`cursor-pointer flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200 focus:outline-none focus:ring-2 ${accent.ring} bg-muted/30 disabled:opacity-50`}
          >
            <Wand2 size={14} />
            {refining ? t('steering.refining') : t('steering.refine')}
          </button>
          <button
            onClick={onSave}
            disabled={!hasChanges || saving}
            className={`cursor-pointer flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200 focus:outline-none focus:ring-2 ${accent.ring} ${
              hasChanges ? accentSolidButtonClass : colors.btnDisabled
            } disabled:opacity-50`}
          >
            <Save size={14} />
            {saving ? t('steering.saving') : t('steering.save')}
          </button>
        </div>
      </div>
      {/* frontmatter 编辑区 */}
      <div className={`px-4 py-3 border-b border-border space-y-2`}>
        <div className="flex items-center gap-4 flex-wrap">
          <div className="flex items-center gap-2">
            <span className={`text-xs text-muted-foreground`}>{t('steering.inclusionMode')}:</span>
            <Select value={editState.inclusion} onValueChange={onInclusionChange}>
              <SelectTrigger className={`text-foreground bg-background border-input ${colors.inputFocus}`} style={{ minWidth: '120px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}>
                <SelectValue placeholder="选择模式..." />
              </SelectTrigger>
              <SelectContent className={`glass-card border border-border`}>
                {inclusionOptions.map((opt: any) => (
                  <SelectItem key={opt.value} value={opt.value} className={"text-foreground"}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {editState.inclusion === 'fileMatch' && (
            <div className="flex items-center gap-2">
              <span className={`text-xs text-muted-foreground`}>{t('steering.filePattern')}:</span>
              <Input
                value={editState.filePattern}
                onChange={(e) => onFilePatternChange(e.target.value)}
                placeholder="**/*.jsx"
                className={`text-foreground bg-background border-input ${colors.inputFocus}`}
                style={{ width: '128px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}
              />
            </div>
          )}
        </div>
        <div className="flex items-center gap-4 flex-wrap">
          <div className="flex items-center gap-2">
            <span className={`text-xs text-muted-foreground`}>{t('steering.fmName')}:</span>
            <Input
              value={editState.name}
              onChange={(e) => onNameChange(e.target.value)}
              placeholder={t('steering.fmNamePlaceholder')}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ width: '140px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}
            />
          </div>
          <div className="flex items-center gap-2 flex-1">
            <span className={`text-xs text-muted-foreground`}>{t('steering.fmDescription')}:</span>
            <Input
              value={editState.description}
              onChange={(e) => onDescriptionChange(e.target.value)}
              placeholder={t('steering.fmDescriptionPlaceholder')}
              className={`text-foreground bg-background border-input ${colors.inputFocus}`}
              style={{ flex: 1, minWidth: '200px', borderRadius: '0.5rem', height: '1.5rem', padding: '0 0.5rem', fontSize: '0.75rem' }}
            />
          </div>
        </div>
      </div>
      <div className="flex-1 p-4 overflow-hidden">
        <Textarea
          value={editState.content}
          onChange={(e) => onContentChange(e.target.value)}
          placeholder={t('steering.contentPlaceholder')}
          className={`flex-1 w-full h-full min-h-[400px] p-4 rounded-xl text-sm leading-relaxed font-mono resize-none border border-input ${colors.inputFocus}`}
          style={{
            color: surface.editorText,
            backgroundColor: surface.editorBg,
            borderColor: surface.editorBorder
          }}
        />
      </div>
    </>
  )
}
