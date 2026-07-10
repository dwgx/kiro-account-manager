import { FolderOpen, Globe } from 'lucide-react'
import { getInclusionStyles } from './steeringFrontMatter'

// scope 徽章
export const ScopeBadge = ({ scope, accent }: any) => {
  if (scope === 'project') {
    return (
      <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-500/15 text-amber-500 border border-amber-500/30">
        <FolderOpen size={10} />项目
      </span>
    )
  }
  return (
    <span className={`flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium ${accent.scopeBadge}`}>
      <Globe size={10} />用户
    </span>
  )
}

// inclusion 徽章
export const InclusionBadge = ({ inclusion, accent }: any) => {
  const styles = getInclusionStyles(accent)
  const s = styles[inclusion] || styles.always
  return (
    <span className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium ${s.bg} ${s.color} border ${s.border}`}>
      <span className={`w-1.5 h-1.5 rounded-full ${s.dot}`} />
      {s.label}
    </span>
  )
}
