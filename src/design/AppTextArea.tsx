import { TextArea } from '@radix-ui/themes';
import type { FocusEventHandler } from 'react';

interface AppTextAreaProps {
  id?: string;
  value: string;
  onValueChange?: (value: string) => void;
  ariaLabel?: string;
  ariaDescribedBy?: string;
  autoFocus?: boolean;
  autoComplete?: string;
  className?: string;
  disabled?: boolean;
  maxLength?: number;
  onFocus?: FocusEventHandler<HTMLTextAreaElement>;
  placeholder?: string;
  readOnly?: boolean;
  required?: boolean;
  resize?: 'none' | 'vertical' | 'horizontal' | 'both';
  rows?: number;
  size?: '1' | '2' | '3';
  spellCheck?: boolean;
}

/** Shared controlled Radix text area for longer user-facing text fields. */
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
  onFocus,
  placeholder,
  readOnly = false,
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
      onChange={onValueChange ? (event) => onValueChange(event.target.value) : undefined}
      onFocus={onFocus}
      placeholder={placeholder}
      readOnly={readOnly}
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
