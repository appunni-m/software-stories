import { TextArea } from '@radix-ui/themes';

interface AppTextAreaProps {
  id?: string;
  value: string;
  onValueChange: (value: string) => void;
  ariaLabel?: string;
  ariaDescribedBy?: string;
  autoFocus?: boolean;
  autoComplete?: string;
  className?: string;
  disabled?: boolean;
  maxLength?: number;
  placeholder?: string;
  required?: boolean;
  resize?: 'none' | 'vertical' | 'horizontal' | 'both';
  rows?: number;
  size?: '1' | '2' | '3';
  spellCheck?: boolean;
}

/** Shared controlled Radix text area for narration and chapter editing. */
export function AppTextArea({
  id,
  value,
  onValueChange,
  ariaLabel,
  ariaDescribedBy,
  autoFocus = false,
  autoComplete,
  className,
  disabled = false,
  maxLength,
  placeholder,
  required = false,
  resize = 'vertical',
  rows = 3,
  size = '2',
  spellCheck,
}: AppTextAreaProps) {
  return (
    <TextArea
      aria-describedby={ariaDescribedBy}
      aria-label={ariaLabel}
      autoComplete={autoComplete}
      autoFocus={autoFocus}
      className={['app-text-area', className].filter(Boolean).join(' ')}
      disabled={disabled}
      id={id}
      maxLength={maxLength}
      onChange={(event) => onValueChange(event.target.value)}
      placeholder={placeholder}
      required={required}
      resize={resize}
      rows={rows}
      size={size}
      spellCheck={spellCheck}
      value={value}
      variant="surface"
      color="gray"
    />
  );
}
