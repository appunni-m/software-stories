import { Slider, type SliderProps } from '@radix-ui/themes';

/** Shared Radix slider for time, amount and other continuous controls. */
export function AppSlider({ className, ...props }: SliderProps) {
  return <Slider
    {...props}
    color="blue"
    className={['app-slider', className].filter(Boolean).join(' ')}
  />;
}
