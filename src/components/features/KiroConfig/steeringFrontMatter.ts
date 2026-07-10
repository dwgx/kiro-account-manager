// 解析 front-matter（v0.10.32: inclusion + name + description + fileMatchPattern）
export const parseFrontMatter = (content: string) => {
  const match = content.match(/^---\n([\s\S]*?)\n---\n?([\s\S]*)$/)
  if (!match) return { inclusion: 'always', filePattern: '', name: '', description: '', body: content }
  const [, fm, body] = match
  return {
    inclusion: fm.match(/inclusion:\s*(\w+)/)?.[1] || 'always',
    filePattern: fm.match(/fileMatchPattern:\s*['"]?([^'"\n]+)['"]?/)?.[1] || '',
    name: fm.match(/name:\s*['"]?([^'"\n]+)['"]?/)?.[1]?.trim() || '',
    description: fm.match(/description:\s*['"]?([^'"\n]+)['"]?/)?.[1]?.trim() || '',
    body
  }
}

// 组装 front-matter（v0.10.32: inclusion + name + description + fileMatchPattern）
export const buildContent = (inclusion: string, filePattern: string, body: string, name: string, description: string) => {
  let fm = `---\ninclusion: ${inclusion}`
  if (name?.trim()) fm += `\nname: "${name.trim()}"`
  if (description?.trim()) fm += `\ndescription: "${description.trim()}"`
  if (inclusion === 'fileMatch' && filePattern.trim()) fm += `\nfileMatchPattern: '${filePattern.trim()}'`
  return fm + '\n---\n' + body
}

// 格式化文件大小
export const formatSize = (bytes: number) => bytes < 1024 ? `${bytes} B` : `${(bytes / 1024).toFixed(1)} KB`

// inclusion 模式配色映射
export const getInclusionStyles = (accent: any): any => ({
  always:    { color: 'text-green-500',  bg: 'bg-green-500/15', border: 'border-green-500/30', dot: 'bg-green-500', label: '始终' },
  auto:      { color: accent.text, bg: accent.bgSoft, border: accent.borderSoft, dot: accent.solidBg, label: '自动' },
  fileMatch: { color: accent.text, bg: accent.bgSoft, border: accent.borderSoft, dot: accent.solidBg, label: '匹配' },
  manual:    { color: 'text-orange-500', bg: 'bg-orange-500/15', border: 'border-orange-500/30', dot: 'bg-orange-500', label: '手动' }})
