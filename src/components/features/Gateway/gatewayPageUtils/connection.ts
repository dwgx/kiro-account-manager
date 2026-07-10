import { parseClientApiKeys } from './parsing'

export const buildGatewayConnectHost = (host: string, localOnly: boolean): string => {
  const value = String(host || '').trim()
  if (!value) {
    return '127.0.0.1'
  }
  if (value === '0.0.0.0' || value === '::') {
    return localOnly ? '127.0.0.1' : 'localhost'
  }

  return value
}

export const buildGatewayBaseUrl = (host: string, port: number, localOnly: boolean): string => {
  const connectHost = buildGatewayConnectHost(host, localOnly)
  const needsBrackets = connectHost.includes(':') && !connectHost.startsWith('[')
  const normalizedHost = needsBrackets ? `[${connectHost}]` : connectHost
  return `http://${normalizedHost}:${Number(port) || 8765}`
}

export const applyGatewayLocalOnlyChange = (
  config: any,
  nextLocalOnly: boolean,
  createApiKey: () => string
): any => {
  const nextConfig = {
    ...config,
    localOnly: !!nextLocalOnly
  }

  if (!nextLocalOnly && !parseClientApiKeys(config?.clientApiKeysText || config?.apiKey).length) {
    const generatedKey = createApiKey()
    return {
      ...nextConfig,
      apiKey: generatedKey,
      clientApiKeysText: generatedKey
    }
  }

  return nextConfig
}
