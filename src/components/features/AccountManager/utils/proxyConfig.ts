import { AccountProxyConfig, AccountProxyProtocol } from '../../../../types/account'

export const defaultProxyConfig = (): AccountProxyConfig => ({
  enabled: false,
  protocol: 'http',
  host: '',
  port: 0,
  username: null,
  password: null
})

export const normalizeProxyConfig = (value?: AccountProxyConfig | null): AccountProxyConfig => ({
  ...defaultProxyConfig(),
  ...value,
  protocol: value?.protocol === 'socks5' ? 'socks5' : 'http',
  host: value?.host || '',
  port: Number(value?.port || 0),
  username: value?.username || null,
  password: value?.password || null
})

export const normalizeProxyForSave = (value: AccountProxyConfig): AccountProxyConfig => ({
  ...value,
  host: value.host.trim(),
  port: Number(value.port || 0),
  username: value.username?.trim() || null,
  password: value.password || null
})

export const parseProxyUrl = (value: string): AccountProxyConfig => {
  const raw = value.trim()
  const url = new URL(raw.includes('://') ? raw : `http://${raw}`)
  const protocol: AccountProxyProtocol = url.protocol.startsWith('socks') ? 'socks5' : 'http'
  const port = Number(url.port)

  if (!url.hostname || !Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error('invalid proxy')
  }

  return {
    enabled: true,
    protocol,
    host: url.hostname,
    port,
    username: url.username ? decodeURIComponent(url.username) : null,
    password: url.password ? decodeURIComponent(url.password) : null
  }
}
