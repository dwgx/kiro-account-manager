// 与后端 src-tauri/src/clients/http_client.rs::SUPPORTED_KIRO_REGIONS 对齐。
// 改这里时同步改后端那份；GatewayConfig.tsx 的 <SelectItem> 列表也要保持一致。
export const ALLOWED_REGIONS = [
  'us-east-1',
  'us-east-2',
  'us-west-1',
  'us-west-2',
  'eu-west-1',
  'eu-west-2',
  'eu-west-3',
  'eu-central-1',
  'eu-central-2',
  'eu-north-1',
  'eu-south-1',
  'eu-south-2',
  'ap-northeast-1',
  'ap-northeast-2',
  'ap-northeast-3',
  'ap-southeast-1',
  'ap-southeast-2',
  'ap-southeast-3',
  'ap-southeast-4',
  'ap-southeast-5',
  'ap-southeast-7',
  'ap-south-1',
  'ap-south-2',
  'ap-east-1',
  'ca-central-1',
  'ca-west-1',
  'sa-east-1',
  'me-south-1',
  'me-central-1',
  'il-central-1',
  'mx-central-1',
  'af-south-1',
  'us-gov-west-1',
  'us-gov-east-1',
  'cn-north-1',
  'cn-northwest-1',
] as const

const isValidIpv4Address = (value: string): boolean => {
  if (!/^\d{1,3}(\.\d{1,3}){3}$/.test(value)) {
    return false
  }

  return value
    .split('.')
    .every(part => {
      const number = Number(part)
      return Number.isInteger(number) && number >= 0 && number <= 255
    })
}

const isValidIpv6Address = (value: string): boolean => {
  try {
    const parsed = new URL(`http://[${value}]/`)
    return parsed.hostname === value
  } catch {
    return false
  }
}

export const isValidGatewayHost = (value: string): boolean => {
  const host = String(value || '').trim()
  if (!host) {
    return false
  }

  if (host === 'localhost' || host === '0.0.0.0' || host === '::' || host === '::1') {
    return true
  }

  return isValidIpv4Address(host) || isValidIpv6Address(host)
}

export const isValidAllowlistEntry = (value: string): boolean => {
  const entry = String(value || '').trim()
  if (!entry) {
    return false
  }

  if (isValidIpv4Address(entry) || isValidIpv6Address(entry)) {
    return true
  }

  const [host, prefix, ...rest] = entry.split('/')
  if (!host || !prefix || rest.length) {
    return false
  }

  if (!/^\d+$/.test(prefix)) {
    return false
  }

  const prefixNumber = Number(prefix)
  if (isValidIpv4Address(host)) {
    return prefixNumber >= 0 && prefixNumber <= 32
  }
  if (isValidIpv6Address(host)) {
    return prefixNumber >= 0 && prefixNumber <= 128
  }

  return false
}
