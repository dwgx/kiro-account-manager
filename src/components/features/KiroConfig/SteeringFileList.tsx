import { FileText, RefreshCw, Trash2, Plus, Wand2, Sparkles } from 'lucide-react'
import { getSolidAccentButton } from './themeAccent'
import { parseFrontMatter, formatSize, getInclusionStyles } from './steeringFrontMatter'
import { ScopeBadge } from './SteeringBadges'

// 文件列表组件
export function FileList({ files, selectedFile, onSelect, onDelete, onRefresh, onCreate, onCreateDefault, onCreateInitial, creatingDefault, initializingProject, hasProjectDir, accent, colors, t }: any) {
  const accentSolidButtonClass = getSolidAccentButton(accent)
  const inclusionStyles = getInclusionStyles(accent)
  // 按 inclusion 分组（保持顺序）
  const groups = [
    { key: 'always',    label: '始终包含' },
    { key: 'auto',      label: '自动激活' },
    { key: 'fileMatch', label: '文件匹配' },
    { key: 'manual',    label: '手动引用' },
  ].map(g => ({
    ...g,
    files: files.filter((f: any) => parseFrontMatter(f.content).inclusion === g.key),
    style: inclusionStyles[g.key]})).filter(g => g.files.length > 0)

  return (
    <div className={`w-72 flex flex-col glass-card border border-border rounded-xl overflow-hidden`}>
      <div className={`p-3 border-b border-border flex items-center justify-between`}>
        <div className="flex items-center gap-2">
          <FileText size={18} className={accent.text} />
          <span className={`text-sm font-semibold text-foreground`}>Steering</span>
          <span className={`text-xs text-muted-foreground`}>({files.length})</span>
        </div>
        <div className="flex gap-2">
          <button
            onClick={onCreateDefault}
            disabled={creatingDefault}
            className={`p-2 rounded-lg hover:bg-muted/50 transition-colors disabled:opacity-50 cursor-pointer`}
            title={t('steering.defaultTemplate')}
          >
            <Wand2 size={16} className={accent.text} />
          </button>
          {hasProjectDir && (
            <button
              onClick={onCreateInitial}
              disabled={initializingProject}
              className={`p-2 rounded-lg hover:bg-muted/50 transition-colors disabled:opacity-50 cursor-pointer`}
              title={t('steering.initializeProject')}
            >
              <Sparkles size={16} className={accent.text} />
            </button>
          )}
          <button
            onClick={onCreate}
            className={`p-2 rounded-lg hover:bg-muted/50 transition-colors cursor-pointer`}
            title={t('steering.newSteering')}
          >
              <Plus size={16} className={accent.text} />
          </button>
          <button
            onClick={onRefresh}
            className={`p-2 rounded-lg hover:bg-muted/50 transition-colors cursor-pointer`}
            title={t('common.refresh')}
          >
            <RefreshCw size={16} className={"text-muted-foreground"} />
          </button>
        </div>
      </div>
      <div className="flex-1 overflow-auto p-4">
        {files.length === 0 ? (
          <div className={`text-center py-16 text-muted-foreground`}>
            <FileText size={48} className="mx-auto mb-3 opacity-20" />
            <p className="text-sm">{t('steering.noFiles')}</p>
            <button
              onClick={onCreate}
              className={`mt-4 px-4 py-2 rounded-lg text-sm transition-colors cursor-pointer ${accentSolidButtonClass}`}
            >
              {t('steering.newSteering')}
            </button>
          </div>
        ) : (
          <div className="space-y-5">
            {groups.map(group => (
              <div key={group.key}>
                {/* 分组标题 - 紧凑风格 */}
                <div className={`flex items-center gap-2 mb-2 px-1`}>
                  <span className={`w-2 h-2 rounded-full ${group.style.dot}`} />
                  <span className={`text-xs font-semibold text-muted-foreground uppercase tracking-wider`}>{group.label}</span>
                  <span className={`text-[10px] text-muted-foreground opacity-60`}>{group.files.length}</span>
                  <div className={`flex-1 h-px border-border opacity-50`} />
                </div>

                {/* 文件卡片 */}
                <div className="space-y-2">
                  {group.files.map((file: any) => {
                    const parsed = parseFrontMatter(file.content)
                    const isSelected = selectedFile?.fileName === file.fileName && selectedFile?.scope === file.scope
                    return (
                      <div
                        key={`${file.scope}-${file.fileName}`}
                        onClick={() => onSelect(file)}
                        className={`p-3 rounded-xl cursor-pointer group transition-all duration-200 ${
                          isSelected
                            ? `${accent.bg} ring-2 ${accent.ring} shadow-lg border ${accent.border}`
                            : `glass-card border border-border hover:bg-muted/50 hover:shadow-md`
                        }`}
                      >
                        <div className="flex items-start justify-between gap-2">
                          <div className="flex items-center gap-2.5 flex-1 min-w-0">
                            <div className={`flex items-center justify-center w-7 h-7 rounded-lg flex-shrink-0 transition-colors ${
                              isSelected ? accent.bg : "bg-muted/30"
                            }`}>
                              <FileText size={15} className={isSelected ? accent.text : "text-muted-foreground"} />
                            </div>
                            <div className="flex-1 min-w-0">
                              <span className={`font-semibold text-sm ${isSelected ? accent.text : "text-foreground"} truncate block leading-tight`}>
                                {parsed.name || file.fileName.replace('.md', '')}
                              </span>
                              {parsed.description && (
                                <span className={`text-xs text-muted-foreground truncate block mt-0.5 leading-tight`}>{parsed.description}</span>
                              )}
                            </div>
                          </div>
                          <button
                            onClick={(e) => { e.stopPropagation(); onDelete(file) }}
                            className="cursor-pointer opacity-0 group-hover:opacity-100 p-1.5 rounded-lg hover:bg-red-500/20 flex-shrink-0 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-red-500/60"
                            title={t('common.delete')}
                          >
                            <Trash2 size={14} className="text-red-500" />
                          </button>
                        </div>
                        <div className={`flex items-center gap-2 text-xs text-muted-foreground mt-2 flex-wrap`} style={{ marginLeft: '2.375rem' }}>
                          <ScopeBadge scope={file.scope} accent={accent} />
                          <span className={`px-1.5 py-0.5 rounded bg-muted/30 text-[10px] font-medium`}>
                            {formatSize(file.size)}
                          </span>
                          {parsed.filePattern && (
                            <code className={`px-1.5 py-0.5 rounded ${accent.bgSoft} border ${accent.borderSoft} font-mono text-[10px] ${accent.textSoft} truncate max-w-[120px]`}>
                              {parsed.filePattern}
                            </code>
                          )}
                        </div>
                      </div>
                    )
                  })}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
