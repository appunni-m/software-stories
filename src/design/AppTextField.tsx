import { TextField } from '@radix-ui/themes';

interface AppTextFieldProps {
  value: string;
  onValueChange: (value: string) => void;
  ariaLabel?: string;
  autoComplete?: string;
  className?: string;
  disabled?: boolean;
  maxLength?: number;
  placeholder?: string;
  size?: '1' | '2' | '3';
  spellCheck?: boolean;
  type?: 'password' | 'text';
}

/** Shared controlled Radix text field for settings and editable values. */
export function AppTextField({
  value,
  onValueChange,
  ariaLabel,
  autoComplete,
  className,
  disabled = false,
  maxLength,
  placeholder,
  size = '2',
  spellCheck,
  type = 'text',
}: AppTextFieldProps) {
  return (
    <TextField.Root
      aria-label={ariaLabel}
      autoComplete={autoComplete}
      className={['app-text-field', className].filter(Boolean).join(' ')}
      disabled={disabled}
      maxLength={maxLength}
      onChange={(event) => onValueChange(event.target.value)}
      placeholder={placeholder}
      size={size}
      spellCheck={spellCheck}
      type={type}
      value={value}
      variant="surface"
      color="gray"
    />
  );
}
