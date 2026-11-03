import { forwardRef, type FocusEventHandler, type KeyboardEventHandler, type ReactNode } from 'react';
import { TextField } from '@radix-ui/themes';
import { Search } from 'lucide-react';

interface AppSearchFieldProps {
  ariaLabel: string;
  placeholder: string;
  value: string;
  onValueChange: (value: string) => void;
  className?: string;
  disabled?: boolean;
  focusKey?: string;
  maxLength?: number;
  onFocus?: FocusEventHandler<HTMLInputElement>;
  onKeyDown?: KeyboardEventHandler<HTMLInputElement>;
  rightSlot?: ReactNode;
  size?: '1' | '2' | '3';
  type?: 'search' | 'text';
}

/** Shared accessible Radix input for repository, chapter and source filtering. */
export const AppSearchField = forwardRef<HTMLInputElement, AppSearchFieldProps>(function AppSearchField({
  ariaLabel,
  placeholder,
  value,
  onValueChange,
  className,
  disabled = false,
  focusKey,
  maxLength,
  onFocus,
  onKeyDown,
  rightSlot,
  size = '2',
  type = 'text',
}, ref) {
  return (
    <TextField.Root
      aria-label={ariaLabel}
      ref={ref}
      placeholder={placeholder}
      value={value}
      onChange={(event) => onValueChange(event.target.value)}
      onFocus={onFocus}
      onKeyDown={onKeyDown}
      disabled={disabled}
      maxLength={maxLength}
      data-focus-key={focusKey}
      size={size}
      type={type}
      variant="surface"
      color="gray"
      className={className}
    >
      <TextField.Slot>
        <Search size={13} aria-hidden="true" />
      </TextField.Slot>
      {rightSlot && <TextField.Slot side="right">{rightSlot}</TextField.Slot>}
    </TextField.Root>
  );
});
