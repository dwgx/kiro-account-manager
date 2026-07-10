import { getPrimaryClientApiKey } from './parsing'
import { redactGatewayApiKey } from './format'

interface ClientSamples {
  anthropic: { env: string }
  openai: { env: string; curl: string }
  openaiChat: { env: string; curl: string }
  claudeCode: { config: string; apiKey: string }
  codex: { config: string; apiKey: string }
}

export const buildClientSamples = (baseUrl: string, apiKey: string | string[]): ClientSamples => {
  const safeKey = redactGatewayApiKey(getPrimaryClientApiKey(apiKey))
  const realKey = getPrimaryClientApiKey(apiKey)
  const anthropicEnv = `ANTHROPIC_BASE_URL=${baseUrl}\nANTHROPIC_API_KEY=${safeKey}`
  const openaiEnv = `OPENAI_BASE_URL=${baseUrl}\nOPENAI_API_KEY=${safeKey}`
  const openaiResponsesCurl = [
    `curl ${baseUrl}/v1/responses \\`,
    '  -H "Content-Type: application/json" \\',
    `  -H "Authorization: Bearer ${safeKey}" \\`,
    '  -d "{\\"model\\":\\"claude-sonnet-4-5-20250929\\",\\"input\\":[{\\"role\\":\\"user\\",\\"content\\":[{\\"type\\":\\"input_text\\",\\"text\\":\\"hello\\"}]}]}"',
  ].join('\n')
  const openaiChatCurl = [
    `curl ${baseUrl}/v1/chat/completions \\`,
    '  -H "Content-Type: application/json" \\',
    `  -H "Authorization: Bearer ${safeKey}" \\`,
    '  -d "{\\"model\\":\\"claude-sonnet-4-5-20250929\\",\\"messages\\":[{\\"role\\":\\"user\\",\\"content\\":\\"hello\\"}]}"',
  ].join('\n')

  // Claude Code 配置（~/.claude/settings.json）
  const claudeCodeConfig = [
    '{',
    '  "env": {',
    `    "ANTHROPIC_BASE_URL": "${baseUrl}",`,
    `    "ANTHROPIC_AUTH_TOKEN": "${safeKey}"`,
    '  }',
    '}',
  ].join('\n')

  // Codex CLI 配置（~/.codex/config.toml + ~/.codex/auth.json）
  const codexConfig = [
    '# ~/.codex/config.toml',
    'model_provider = "custom"',
    'model = "claude-sonnet-4-5-20250929"',
    '',
    '[model_providers.custom]',
    'name = "custom"',
    `base_url = "${baseUrl}"`,
    'wire_api = "responses"',
    'requires_openai_auth = true',
    '',
    '# ~/.codex/auth.json',
    '{',
    `  "OPENAI_API_KEY": "${safeKey}"`,
    '}',
  ].join('\n')

  return {
    anthropic: {
      env: anthropicEnv
    },
    openai: {
      env: openaiEnv,
      curl: openaiResponsesCurl
    },
    openaiChat: {
      env: openaiEnv,
      curl: openaiChatCurl
    },
    claudeCode: {
      config: claudeCodeConfig,
      apiKey: realKey
    },
    codex: {
      config: codexConfig,
      apiKey: realKey
    }
  }
}
