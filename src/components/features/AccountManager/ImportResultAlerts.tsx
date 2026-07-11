import { Stack } from '@/components/shared/layout'
import { Alert } from '@/components/ui/alert'
import { AlertCircle, CheckCircle } from 'lucide-react'

export function ImportResultAlerts({ result }: { result: any }) {
  return (
  <Stack gap="md" p="sm">
    {result.added && result.added.length > 0 && (
      <Alert variant="success">
        <CheckCircle size={20} />
        <div className={`font-medium text-foreground`}>✅ 新增 {result.added.length} 个账号</div>
        {result.added.length > 0 && (
          <div className={`text-sm mt-2 text-foreground`}>{result.added.map((s: any) => s.email).join(', ')}</div>
        )}
      </Alert>
    )}

    {result.updated && result.updated.length > 0 && (
      <Alert variant="info">
        <CheckCircle size={20} />
        <div className={`font-medium text-foreground`}>📝 更新 {result.updated.length} 个账号</div>
        {result.updated.length > 0 && (
          <div className={`text-sm mt-2 text-foreground`}>{result.updated.map((s: any) => s.email).join(', ')}</div>
        )}
      </Alert>
    )}

    {result.failed && result.failed.length > 0 && (
      <Alert variant="destructive">
        <AlertCircle size={20} />
        <div className={`font-medium text-foreground`}>❌ 失败 {result.failed.length} 个</div>
        <Stack gap={4} mt="xs" p={0}>
          {result.failed.map((f: any, i: number) => (
            <div key={i} className={`text-sm text-foreground`}>{f.error}</div>
          ))}
        </Stack>
      </Alert>
    )}
  </Stack>
  )
}
