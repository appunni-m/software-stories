import type { InputHTMLAttributes } from 'react';

export type AppRadioProps = Omit<InputHTMLAttributes<HTMLInputElement>, 'type'>;

/** Shared native radio control with consistent sizing, accent and focus treatment. */
export function AppRadio({ className, ...props }: AppRadioProps) {
  return <input {...props} type="radio" className={`app-radio${className ? ` ${className}` : ''}`} />;
}
