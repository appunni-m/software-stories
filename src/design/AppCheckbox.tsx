import type { InputHTMLAttributes } from 'react';

export type AppCheckboxProps = Omit<InputHTMLAttributes<HTMLInputElement>, 'type'>;

/** Shared native checkbox control with consistent sizing, accent and focus treatment. */
export function AppCheckbox({ className, ...props }: AppCheckboxProps) {
  return <input {...props} type="checkbox" className={`app-checkbox${className ? ` ${className}` : ''}`} />;
}
