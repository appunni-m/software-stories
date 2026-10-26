import { Select } from '@radix-ui/themes';

export interface AppSelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

interface AppSelectProps {
  ariaLabel: string;
  value: string;
  options: readonly AppSelectOption[];
  onValueChange: (value: string) => void;
  id?: string;
  disabled?: boolean;
  clearLabel?: string;
  size?: '1' | '2' | '3';
  className?: string;
  variant?: 'classic' | 'surface' | 'soft' | 'ghost';
}

/** Shared accessible select for repository, reader, connection and review controls. */
export function AppSelect({
  ariaLabel,
  value,
  options,
  onValueChange,
  id,
  disabled = false,
  clearLabel,
  size = '2',
  className,
  variant = 'surface',
}: AppSelectProps) {
  const clearValue = 'code-atlas:select:clear';
  return (
    <Select.Root size={size} value={clearLabel && value === '' ? clearValue : value} onValueChange={(next) => onValueChange(next === clearValue ? '' : next)} disabled={disabled}>
      <Select.Trigger
        id={id}
        className={`app-select-trigger${className ? ` ${className}` : ''}`}
        aria-label={ariaLabel}
        variant={variant}
        color="gray"
      />
      <Select.Content className="app-select-content" position="popper">
        {clearLabel && <Select.Item value={clearValue}>{clearLabel}</Select.Item>}
        {options.map((option) => (
          <Select.Item key={option.value} value={option.value} disabled={option.disabled}>
            {option.label}
          </Select.Item>
        ))}
      </Select.Content>
    </Select.Root>
  );
}
