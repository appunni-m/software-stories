import { TextField } from '@radix-ui/themes';
import { Search } from 'lucide-react';

interface AppSearchFieldProps {
  ariaLabel: string;
  placeholder: string;
  value: string;
  onValueChange: (value: string) => void;
  className?: string;
  focusKey?: string;
  size?: '1' | '2' | '3';
}

/** Shared Radix search input for repository and relationship filtering. */
export function AppSearchField({
  ariaLabel,
  placeholder,
  value,
  onValueChange,
  className,
  focusKey,
  size = '2',
}: AppSearchFieldProps) {
  return (
    <TextField.Root
      aria-label={ariaLabel}
      placeholder={placeholder}
      value={value}
      onChange={(event) => onValueChange(event.target.value)}
      data-focus-key={focusKey}
      size={size}
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
