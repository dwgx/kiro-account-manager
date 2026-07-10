import { Copy, Check } from 'lucide-react'

interface AccountCredentialForm {
  label: string;
  accessToken: string;
  refreshToken: string;
  clientId: string;
  clientSecret: string;
  machineId: string;
  addedAt: string;
  expiresAt: string;
}

interface AccountCredentialFieldsProps {
  form: AccountCredentialForm;
  onFormChange: (patch: Partial<AccountCredentialForm>) => void;
  handleCopy: (text: string, field: string) => void;
  copiedField: string | null;
  isIdCAccount: boolean;
  colors: { inputFocus: string };
  t: (k: string) => string;
}

export function AccountCredentialFields({ form, onFormChange, handleCopy, copiedField, isIdCAccount, colors, t }: AccountCredentialFieldsProps) {
  return (
    <>
      {/* 账号别名 */}
      <div>
        <label className={`block text-sm font-medium text-foreground mb-2`}>
          {t('accounts.remark')}
        </label>
        <input
          type="text"
          placeholder={t('editAccount.labelPlaceholder')}
          value={form.label}
          onChange={(e) => onFormChange({ label: e.target.value })}
          className={`w-full px-4 py-3 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
        />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div>
          <label className={`block text-sm font-medium text-foreground mb-2`}>
            添加时间
          </label>
          <input
            type="text"
            placeholder="YYYY/MM/DD HH:mm:ss"
            value={form.addedAt}
            onChange={(e) => onFormChange({ addedAt: e.target.value })}
            className={`w-full px-4 py-3 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none font-mono`}
          />
        </div>
        <div>
          <label className={`block text-sm font-medium text-foreground mb-2`}>
            Token 到期时间
          </label>
          <input
            type="text"
            placeholder="YYYY/MM/DD HH:mm:ss"
            value={form.expiresAt}
            onChange={(e) => onFormChange({ expiresAt: e.target.value })}
            className={`w-full px-4 py-3 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none font-mono`}
          />
        </div>
      </div>

      {/* Access Token */}
      <div>
        <label className={`block text-sm font-medium text-foreground mb-2`}>
          Access Token
        </label>
        <div className="relative">
          <textarea
            placeholder="access token"
            value={form.accessToken}
            onChange={(e) => onFormChange({ accessToken: e.target.value })}
            rows={3}
            className={`w-full px-4 py-3 pr-10 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 resize-none outline-none font-mono`}
          />
          <button
            onClick={() => handleCopy(form.accessToken, 'accessToken')}
            className={`absolute right-3 top-3 p-1.5 rounded-lg hover:bg-muted/50 cursor-pointer`}
            title={copiedField === 'accessToken' ? '已复制' : '复制'}
          >
            {copiedField === 'accessToken' ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={"text-muted-foreground"} />}
          </button>
        </div>
      </div>

      {/* Refresh Token */}
      <div>
        <label className={`block text-sm font-medium text-foreground mb-2`}>
          Refresh Token {isIdCAccount && <span className="text-destructive">*</span>}
        </label>
        <div className="relative">
          <textarea
            placeholder="aorAAAAA..."
            value={form.refreshToken}
            onChange={(e) => onFormChange({ refreshToken: e.target.value })}
            rows={3}
            className={`w-full px-4 py-3 pr-10 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 resize-none outline-none font-mono`}
          />
          <button
            onClick={() => handleCopy(form.refreshToken, 'refreshToken')}
            className={`absolute right-3 top-3 p-1.5 rounded-lg hover:bg-muted/50 cursor-pointer`}
            title={copiedField === 'refreshToken' ? '已复制' : '复制'}
          >
            {copiedField === 'refreshToken' ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={"text-muted-foreground"} />}
          </button>
        </div>
      </div>

      {/* Machine ID */}
      <div>
        <label className={`block text-sm font-medium text-foreground mb-2`}>
          {t('addAccount.machineId')}
        </label>
        <div className="relative">
          <input
            type="text"
            placeholder={t('addAccount.machineIdPlaceholder')}
            value={form.machineId}
            onChange={(e) => onFormChange({ machineId: e.target.value })}
            className={`w-full px-4 py-3 pr-10 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none`}
          />
          <button
            onClick={() => handleCopy(form.machineId, 'machineId')}
            className={`absolute right-3 top-1/2 -translate-y-1/2 p-1.5 rounded-lg hover:bg-muted/50 cursor-pointer`}
            title={copiedField === 'machineId' ? '已复制' : '复制'}
          >
            {copiedField === 'machineId' ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={"text-muted-foreground"} />}
          </button>
        </div>
      </div>

      {isIdCAccount && (
        <>
          <div>
            <label className={`block text-sm font-medium text-foreground mb-2`}>
              Client ID <span className="text-destructive">*</span>
            </label>
            <div className="relative">
              <input
                type="text"
                placeholder="刷新 Token 需要"
                value={form.clientId}
                onChange={(e) => onFormChange({ clientId: e.target.value })}
                className={`w-full px-4 py-3 pr-10 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 outline-none font-mono`}
              />
              <button
                onClick={() => handleCopy(form.clientId, 'clientId')}
                className={`absolute right-3 top-1/2 -translate-y-1/2 p-1.5 rounded-lg hover:bg-muted/50 cursor-pointer`}
                title={copiedField === 'clientId' ? '已复制' : '复制'}
              >
                {copiedField === 'clientId' ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={"text-muted-foreground"} />}
              </button>
            </div>
          </div>
          <div>
            <label className={`block text-sm font-medium text-foreground mb-2`}>
              Client Secret <span className="text-destructive">*</span>
            </label>
            <div className="relative">
              <textarea
                placeholder="刷新 Token 需要"
                value={form.clientSecret}
                onChange={(e) => onFormChange({ clientSecret: e.target.value })}
                rows={2}
                className={`w-full px-4 py-3 pr-10 border rounded-xl text-sm text-foreground bg-background border-input ${colors.inputFocus} focus:ring-2 resize-none outline-none font-mono`}
              />
              <button
                onClick={() => handleCopy(form.clientSecret, 'clientSecret')}
                className={`absolute right-3 top-3 p-1.5 rounded-lg hover:bg-muted/50 cursor-pointer`}
                title={copiedField === 'clientSecret' ? '已复制' : '复制'}
              >
                {copiedField === 'clientSecret' ? <Check size={16} className="text-green-500" /> : <Copy size={16} className={"text-muted-foreground"} />}
              </button>
            </div>
          </div>
        </>
      )}
    </>
  )
}
