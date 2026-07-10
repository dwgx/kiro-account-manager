export const parseAllowedIps = (value: string | string[]): string[] => {
  if (Array.isArray(value)) return value
  return String(value || '')
    .split(/[\n,]+/)
    .map(item => item.trim())
    .filter(Boolean)
}

export const parseClientApiKeys = (value: string | string[]): string[] => {
  if (Array.isArray(value)) return value
  return String(value || '')
    .split(/[\n,]+/)
    .map(item => item.trim())
    .filter(Boolean)
    .filter((item, index, items) => items.indexOf(item) === index)
}

export const getPrimaryClientApiKey = (value: string | string[]): string =>
  parseClientApiKeys(value)[0] || ''
