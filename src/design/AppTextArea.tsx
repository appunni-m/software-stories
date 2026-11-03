import { TextArea } from '@radix-ui/themes';
import { forwardRef, type ChangeEventHandler, type FocusEventHandler, type KeyboardEventHandler } from 'react';

interface AppTextAreaProps {
  id?: string;
  dataFocusKey?: string;
  value: string;
  onValueChange?: (value: string) => void;
  onChange?: ChangeEventHandler<HTMLTextAreaElement>;
  onKeyDown?: KeyboardEventHandler<HTMLTextAreaElement>;
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
export const AppTextArea = forwardRef<HTMLTextAreaElement, AppTextAreaProps>(function AppTextArea({
  id,
  dataFocusKey,
  value,
  onValueChange,
  onChange,
  onKeyDown,
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
}, ref) {
  return (
    <TextArea
      ref={ref}
      aria-describedby={ariaDescribedBy}
      aria-label={ariaLabel}
      autoComplete={autoComplete}
      autoFocus={autoFocus}
      className={['app-text-area', className].filter(Boolean).join(' ')}
      disabled={disabled}
      id={id}
      data-focus-key={dataFocusKey}
      maxLength={maxLength}
      onChange={onValueChange || onChange ? (event) => {
        onValueChange?.(event.target.value);
        onChange?.(event);
      } : undefined}
      onFocus={onFocus}
      onKeyDown={onKeyDown}
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
});
