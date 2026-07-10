export const formatGatewayTimestamp = (date = new Date()): string => {
  const pad = (value: number) => String(value).padStart(2, '0')

  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

export const redactGatewayApiKey = (apiKey: string): string => {
  const value = String(apiKey || '').trim()
  if (!value) {
    return 'sk-your-gateway-api-key'
  }
  if (value.length <= 8) {
    return `${value.slice(0, 3)}***`
  }
  return `${value.slice(0, 6)}...${value.slice(-4)}`
}

export const formatGatewayRequestDuration = (durationMs: number): string => {
  const duration = Number(durationMs) || 0
  if (duration < 1000) {
    return `${duration} ms`
  }
  return `${(duration / 1000).toFixed(duration >= 10_000 ? 0 : 2)} s`
}

export const getGatewayRequestOutcomeColor = (outcome: string): string => {
  if (outcome === 'success') return 'teal'
  if (outcome === 'stream') return 'blue'
  if (outcome === 'error') return 'red'
  return 'gray'
}
