import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import {
  normalizeAccountStatus,
  isActiveStatus,
  isUnavailableStatus,
  getAccountStatusMeta
} from './accountStatus'

const autoRefresh = await readFile(new URL('../hooks/useAutoRefresh.ts', import.meta.url), 'utf8')
const useAccounts = await readFile(new URL('../components/features/AccountManager/hooks/useAccounts.ts', import.meta.url), 'utf8')
const importModal = await readFile(new URL('../components/features/AccountManager/hooks/useImportAccount.ts', import.meta.url), 'utf8')

const cappedAccount = {
  status: 'active',
  usageData: {
    overageConfiguration: {
      overageStatus: 'DISABLED'
    },
    usageBreakdownList: [
      {
        currentUsage: 50,
        usageLimit: 50
      },
    ]
  }
}

assert.equal(normalizeAccountStatus(cappedAccount), 'capped')
assert.equal(isActiveStatus(cappedAccount), false)
assert.equal(isUnavailableStatus(cappedAccount), false)
assert.deepEqual(getAccountStatusMeta(cappedAccount), {
  key: 'capped',
  label: '封顶',
  tone: 'warning'
})

assert.match(autoRefresh, /export function useAutoRefresh\(\)/)
assert.match(autoRefresh, /return \{\}/)
assert.doesNotMatch(autoRefresh, /status !== 'banned'/)
assert.doesNotMatch(autoRefresh, /invoke\(/)

assert.match(useAccounts, /isUnavailableStatus/)
assert.doesNotMatch(useAccounts, /status === 'banned'/)
assert.doesNotMatch(useAccounts, /status !== 'banned'/)

assert.match(importModal, /isBannedStatus/)
assert.doesNotMatch(importModal, /status === 'banned'/)

console.log('account status compatibility wiring looks correct')
