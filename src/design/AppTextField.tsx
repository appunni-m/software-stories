import { TextField } from '@radix-ui/themes';

interface AppTextFieldProps {
  id?: string;
  value: string;
  onValueChange: (value: string) => void;
  ariaLabel?: string;
  autoComplete?: string;
  autoCapitalize?: 'none' | 'sentences' | 'words' | 'characters';
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
  id,
  value,
  onValueChange,
  ariaLabel,
  autoComplete,
  autoCapitalize,
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
      autoCapitalize={autoCapitalize}
      className={['app-text-field', className].filter(Boolean).join(' ')}
      disabled={disabled}
      id={id}
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
