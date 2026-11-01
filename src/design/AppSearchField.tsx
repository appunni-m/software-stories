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
  size?: '1' | '2' | '3';
  type?: 'search' | 'text';
}

/** Shared accessible Radix input for repository, chapter and source filtering. */
export function AppSearchField({
  ariaLabel,
  placeholder,
  value,
  onValueChange,
  className,
  disabled = false,
  focusKey,
  size = '2',
  type = 'text',
}: AppSearchFieldProps) {
  return (
    <TextField.Root
      aria-label={ariaLabel}
      placeholder={placeholder}
      value={value}
      onChange={(event) => onValueChange(event.target.value)}
      disabled={disabled}
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
    </TextField.Root>
  );
}
