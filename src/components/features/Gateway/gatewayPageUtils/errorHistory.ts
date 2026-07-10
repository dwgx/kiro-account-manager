export interface ErrorHistoryEntry {
  message: string
  firstSeenAt: string
  lastSeenAt: string
  count: number
}

export const mergeErrorHistory = (
  history: ErrorHistoryEntry[],
  message: string,
  seenAt: string,
  limit = 8
): ErrorHistoryEntry[] => {
  const normalizedMessage = String(message || '').trim()
  if (!normalizedMessage) {
    return history
  }

  const existingIndex = history.findIndex(item => item.message === normalizedMessage)
  if (existingIndex >= 0) {
    const next = [...history]
    const existing = next[existingIndex]
    next[existingIndex] = {
      ...existing,
      count: existing.count + 1,
      lastSeenAt: seenAt
    }
    return next
  }

  return [
    {
      message: normalizedMessage,
      firstSeenAt: seenAt,
      lastSeenAt: seenAt,
      count: 1
    },
    ...history,
  ].slice(0, limit)
}
