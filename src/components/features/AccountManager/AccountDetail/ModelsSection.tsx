import { RefreshCw, Cpu, Loader2, FileText, Image as ImageIcon, Zap, Hash, ChevronDown } from 'lucide-react'
import { AvailableModel } from '../../../../types/account'
import { formatTokenLimit, formatModelList, formatEffortLabel } from '../utils/accountModelFormat'

interface ModelsSectionProps {
  models: AvailableModel[];
  modelsLoading: boolean;
  modelsError: string | null;
  modelsExpanded: boolean;
  t: any;
  onToggle: () => void;
  onForceRefresh: () => void;
}

export function ModelsSection({
  models,
  modelsLoading,
  modelsError,
  modelsExpanded,
  t,
  onToggle,
  onForceRefresh
}: ModelsSectionProps) {
  return (
    <div className={`px-6 py-4`}>
      <div
        className="flex items-center gap-2 cursor-pointer select-none"
        onClick={onToggle}
      >
        <div className={`p-1.5 rounded-lg bg-muted/30`}>
          <Cpu size={18} className={"text-muted-foreground"} />
        </div>
        <span className={`text-sm font-semibold text-foreground`}>{t('detail.availableModels')}</span>
        <span className={`ml-auto text-xs px-2 py-0.5 rounded-full bg-primary/10 text-primary border border-primary/20 font-medium`}>
          {models.length}
        </span>
        <button
          onClick={(e) => { e.stopPropagation(); onForceRefresh() }}
          disabled={modelsLoading}
          className="p-1.5 rounded-lg hover:bg-muted/50 transition-colors disabled:opacity-50"
          title="强制刷新模型列表"
        >
          <RefreshCw size={14} className={modelsLoading ? "animate-spin text-muted-foreground" : "text-muted-foreground"} />
        </button>
        <ChevronDown size={16} className={`text-muted-foreground transition-transform duration-200 ${modelsExpanded ? '' : '-rotate-90'}`} />
      </div>
      {modelsExpanded && (
      <div className="bg-gradient-to-br from-muted/20 to-muted/40 border rounded-xl p-4 mt-4">
        {modelsLoading ? (
          <div className="flex items-center justify-center py-8 text-muted-foreground">
            <Loader2 size={20} className="animate-spin mr-2" />
            <span className="text-sm">{t('detail.loadingModels')}</span>
          </div>
        ) : modelsError ? (
          <div className="text-center py-8">
            <p className="text-red-500 text-sm">{modelsError}</p>
          </div>
        ) : models.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground text-sm">
            {t('detail.noModels')}
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 max-h-[320px] overflow-y-auto pr-1">
            {models.map((model) => (
              <div
                key={model.modelId}
                className={`group p-3 bg-background rounded-xl border shadow-sm hover:shadow-md hover:border-primary/30 transition-all duration-200 ${
                  model.isDefault ? 'ring-1 ring-primary/20' : ''
                }`}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1.5">
                      <div className={`w-2 h-2 rounded-full shrink-0 ${
                        model.isDefault ? 'bg-primary animate-pulse' : 'bg-muted-foreground/40'
                      }`} />
                      <code className="text-xs font-bold text-foreground truncate">
                        {model.modelId}
                      </code>
                    </div>
                    {model.modelName && model.modelName !== model.modelId && (
                      <p className="text-[11px] text-primary/80 font-medium mb-1 truncate">{model.modelName}</p>
                    )}
                    <p className="text-[11px] text-muted-foreground line-clamp-2 leading-relaxed">
                      {model.description || t('detail.noDescription')}
                    </p>
                  </div>
                </div>
                <div className="flex flex-wrap items-center gap-1.5 mt-2 pt-2 border-t border-border/50">
                  {model.provider && (
                    <span className="text-[10px] px-1.5 h-5 bg-slate-500/10 text-slate-600 dark:text-slate-300 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                      {model.provider}
                    </span>
                  )}
                  {model.supportedInputTypes?.includes('TEXT') && (
                    <span className="text-[10px] px-1.5 h-5 bg-blue-500/10 text-blue-600 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                      <FileText size={12} />Text
                    </span>
                  )}
                  {model.supportedInputTypes?.includes('IMAGE') && (
                    <span className="text-[10px] px-1.5 h-5 bg-purple-500/10 text-purple-600 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                      <ImageIcon size={12} />Image
                    </span>
                  )}
                  {model.rateMultiplier !== undefined && model.rateMultiplier !== null && (
                    <span className="text-[10px] px-1.5 h-5 bg-amber-500/10 text-amber-600 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                      <Zap size={12} />{model.rateMultiplier}x{model.rateUnit ? ` ${model.rateUnit}` : ''}
                    </span>
                  )}
                  <span className="text-[10px] px-1.5 h-5 bg-emerald-500/10 text-emerald-600 border-0 rounded inline-flex items-center gap-0.5 font-medium font-mono">
                    <Hash size={12} />{formatTokenLimit(model.tokenLimits?.maxInputTokens)} / {formatTokenLimit(model.tokenLimits?.maxOutputTokens)}
                  </span>
                  {model.promptCaching?.supportsPromptCaching !== undefined && model.promptCaching?.supportsPromptCaching !== null && (
                    <span className={`text-[10px] px-1.5 h-5 border-0 rounded inline-flex items-center gap-0.5 font-medium ${
                      model.promptCaching.supportsPromptCaching
                        ? 'bg-green-500/10 text-green-600'
                        : 'bg-muted text-muted-foreground'
                    }`}>
                      Cache {model.promptCaching.supportsPromptCaching ? 'On' : 'Off'}
                    </span>
                  )}
                  {model.contextWindow && (
                    <span className="text-[10px] px-1.5 h-5 bg-cyan-500/10 text-cyan-600 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                      Context {formatTokenLimit(model.contextWindow)}
                    </span>
                  )}
                  {model.effortLevels?.length ? (
                    <span className="text-[10px] px-1.5 h-5 bg-indigo-500/10 text-indigo-600 border-0 rounded inline-flex items-center gap-0.5 font-medium">
                      Effort {formatEffortLabel(model)}
                    </span>
                  ) : null}
                  {model.capabilities?.length ? (
                    <span className="text-[10px] px-1.5 h-5 bg-muted text-muted-foreground border-0 rounded inline-flex items-center gap-0.5 font-medium max-w-full truncate">
                      Cap {formatModelList(model.capabilities)}
                    </span>
                  ) : null}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      )}
    </div>
  )
}
