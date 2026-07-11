import { Upload } from 'lucide-react'

interface EmptyAccountsStateProps {
  hasFilter: boolean;
  accent: any;
  onImport: () => void;
}

function EmptyAccountsState({ hasFilter, accent, onImport }: EmptyAccountsStateProps) {
  return (
    <div className="flex-1 flex items-center justify-center">
      <div className="text-center max-w-md px-6 py-10">
        <div className={`w-16 h-16 mx-auto mb-4 rounded-2xl bg-gradient-to-br ${accent.gradientFrom} ${accent.gradientTo} flex items-center justify-center shadow-md ring-1 ring-primary/20`}>
          <svg width="32" height="32" viewBox="0 0 40 40" fill="none">
            <path d="M20 4C12 4 6 10 6 18C6 22 8 25 8 25C8 25 7 28 7 30C7 32 8 34 10 34C11 34 12 33 13 32C14 33 16 34 20 34C24 34 26 33 27 32C28 33 29 34 30 34C32 34 33 32 33 30C33 28 32 25 32 25C32 25 34 22 34 18C34 10 28 4 20 4ZM14 20C12.5 20 11 18.5 11 17C11 15.5 12.5 14 14 14C15.5 14 17 15.5 17 17C17 18.5 15.5 20 14 20ZM26 20C24.5 20 23 18.5 23 17C23 15.5 24.5 14 26 14C27.5 14 29 15.5 29 17C29 18.5 27.5 20 26 20Z" fill="white" />
          </svg>
        </div>
        <h3 className="text-base font-semibold text-foreground mb-1.5">
          {hasFilter ? '没有找到匹配的账号' : '还没有账号'}
        </h3>
        <p className="text-xs text-muted-foreground mb-5">
          {hasFilter
            ? '试试调整筛选条件或搜索关键词'
            : '导入账号开始管理你的 Kiro IDE 账户'}
        </p>
        {!hasFilter && (
          <button
            onClick={onImport}
            className={`px-4 py-2 rounded-lg text-sm font-medium text-white bg-gradient-to-r ${accent.gradientFrom} ${accent.gradientTo} shadow-md hover:shadow-lg transition-all duration-200 inline-flex items-center gap-1.5 cursor-pointer`}
          >
            <Upload size={14} />
            导入账号
          </button>
        )}
      </div>
    </div>
  )
}

export default EmptyAccountsState
