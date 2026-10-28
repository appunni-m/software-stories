import { forwardRef } from 'react';
import { Button as RadixButton, type ButtonProps as RadixButtonProps } from '@radix-ui/themes';

export type AppButtonIntent = 'primary' | 'secondary' | 'quiet' | 'icon' | 'danger' | 'custom';

export type AppButtonProps = Omit<RadixButtonProps, 'variant'> & {
  intent?: AppButtonIntent;
};

const intentStyles: Record<Exclude<AppButtonIntent, 'custom'>, { variant: 'solid' | 'surface' | 'ghost'; className: string }> = {
  primary: { variant: 'solid', className: 'primary-button' },
  secondary: { variant: 'surface', className: 'secondary-button' },
  quiet: { variant: 'ghost', className: 'text-button' },
  icon: { variant: 'ghost', className: 'icon-button' },
  danger: { variant: 'surface', className: 'danger-button' },
};

/** Shared accessible action primitive; keep existing visual variants during migration. */
export const AppButton = forwardRef<HTMLButtonElement, AppButtonProps>(function AppButton({
  intent = 'secondary',
  className,
  type = 'button',
  ...props
}, ref) {
  const style = intent === 'custom' ? { variant: 'ghost' as const, className: '' } : intentStyles[intent];
  return <RadixButton
    {...props}
    ref={ref}
    type={type}
    variant={style.variant}
    className={[style.className, className].filter(Boolean).join(' ')}
  />;
});
