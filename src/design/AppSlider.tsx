import { Range, Root, Thumb, Track, type SliderProps } from '@radix-ui/react-slider';

/** Shared Radix slider for time, amount and other continuous controls. */
export interface AppSliderProps extends SliderProps {
  ariaLabel: string;
}

export function AppSlider({ ariaLabel, className, ...props }: AppSliderProps) {
  return <Root {...props} className={['app-slider', className].filter(Boolean).join(' ')}>
    <Track className="app-slider-track"><Range className="app-slider-range" /></Track>
    <Thumb className="app-slider-thumb" aria-label={ariaLabel} />
  </Root>;
}
