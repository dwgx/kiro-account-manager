import { useRef } from 'react'
import { Button } from '../../shared/button'

export function LegacyButton({ color, leftSection, className = '', children, ...props }: any) {
  const colorClass = color === 'red'
    ? 'text-red-600 hover:text-red-700'
    : color === 'blue'
      ? 'text-blue-600 hover:text-blue-700'
      : color === 'violet' || color === 'grape'
        ? 'text-purple-600 hover:text-purple-700'
        : ''
  return (
    <Button {...props} variant="secondary" size={props.size === 'xs' || props.size === 'sm' ? 'sm' : 'default'} className={`${colorClass} ${className}`.trim()}>
      {leftSection}
      {children}
    </Button>
  )
}

export function FileButton({ onChange, accept, children }: any) {
  const inputRef = useRef<HTMLInputElement>(null)
  const triggerProps = { onClick: () => inputRef.current?.click() }
  const handleChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0] || null
    if (file) {
      await onChange(file)
    }
    event.target.value = ''
  }
  return (
    <>
      <input ref={inputRef} type="file" accept={accept} className="hidden" onChange={handleChange} />
      {children(triggerProps)}
    </>
  )
}
