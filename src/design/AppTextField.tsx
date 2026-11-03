import { TextField } from '@radix-ui/themes';

interface AppTextFieldProps {
  id?: string;
  list?: string;
  max?: number | string;
  min?: number | string;
  value: string;
  onValueChange: (value: string) => void;
  ariaLabel?: string;
  autoComplete?: string;
  autoCapitalize?: 'none' | 'sentences' | 'words' | 'characters';
  className?: string;
  disabled?: boolean;
  maxLength?: number;
  placeholder?: string;
  required?: boolean;
  size?: '1' | '2' | '3';
  spellCheck?: boolean;
  step?: number | string;
  type?: 'number' | 'password' | 'text';
}

/** Shared controlled Radix text field for settings and editable values. */
export function AppTextField({
  id,
  list,
  max,
  min,
  value,
  onValueChange,
  ariaLabel,
  autoComplete,
  autoCapitalize,
  className,
  disabled = false,
  maxLength,
  placeholder,
  required = false,
  size = '2',
  spellCheck,
  step,
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
      list={list}
      maxLength={maxLength}
      max={max}
      min={min}
      onChange={(event) => onValueChange(event.target.value)}
      placeholder={placeholder}
      required={required}
      size={size}
      spellCheck={spellCheck}
      step={step}
      type={type}
      value={value}
      variant="surface"
      color="gray"
    />
  );
}
