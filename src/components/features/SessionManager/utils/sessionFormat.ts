export function decodeWorkspaceName(hash: string) {
  try {
    // 移除末尾的 __ 或 _
    const cleaned = hash.replace(/_+$/, '')
    // Base64 解码
    const decoded = atob(cleaned)
    // 提取最后一个路径段作为显示名称
    const parts = decoded.split(/[/\\]/)
    const name = parts[parts.length - 1] || parts[parts.length - 2] || decoded
    return name
  } catch {
    return hash
  }
}

export function formatFileSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

export function formatDate(timestamp?: number) {
  if (!timestamp) return '-'
  return new Date(timestamp * 1000).toLocaleString('zh-CN')
}
