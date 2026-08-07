import { useEffect, useRef } from 'react';
import { PixelBox } from '../PixelBox/pixelbox';
import baseStyles from './numbersetting.module.css';
import { soundManager } from '../../utils/sound';

export interface NumberSettingProps {
    label: string;
    range: string;
    value: number | '';
    min: number;
    max: number;
    step?: number;
    disabled?: boolean;
    onChange: (val: number | '') => void;
    styles: Record<string, string>;
    boxBorderColour: string;
}

export const NumberSetting = ({
    label = '',
    range = '',
    value,
    min,
    max,
    step = 1,
    disabled = false,
    onChange,
    styles,
    boxBorderColour
}: NumberSettingProps) => {
    const timerRef = useRef<{ timeout?: ReturnType<typeof setTimeout>, interval?: ReturnType<typeof setInterval> }>({});
    const valueRef = useRef(value);

    // Keeps the ref synced with the current value to avoid stale closures in the interval
    useEffect(() => {
        valueRef.current = value;
    }, [value]);

    const stopHold = () => {
        clearTimeout(timerRef.current.timeout);
        clearInterval(timerRef.current.interval);
    };

    const startHold = (direction: 1 | -1) => {
        if (disabled) return;

        const performStep = () => {
            const currentVal = Number(valueRef.current) || 0;
            const nextVal = Math.max(min, Math.min(max, currentVal + (step * direction)));
            if (nextVal !== valueRef.current) {
                soundManager.playSound("button_click");
            }
            onChange(nextVal);
        };

        // click once initially
        performStep();

        // then fire event every 75ms after 200ms of being held
        timerRef.current.timeout = setTimeout(() => {
            timerRef.current.interval = setInterval(performStep, 75);
        }, 200);
    };

    useEffect(() => {
        return stopHold;
    }, []);

    return (
        <div className={styles.formGroup} style={{ flex: 1 }}>
            {label !== '' &&
                <label>
                    {label} {range !== '' && <span style={{ fontFamily: '"Minecraft", "Pixeloid Sans", sans-serif', fontSize: '0.6em', opacity: 0.8 }}>({range})</span>}
                </label>
            }
            <div className={styles.inputOuter}>
                <PixelBox innerClassName={`${styles.inputInner} ${styles.capacityInner}`} borderColour={boxBorderColour}>
                    {!disabled && (
                        <button
                            type="button"
                            data-no-sound="true"
                            className={`${styles.capacityBtn} ${baseStyles.capacityBtn}`}
                            onMouseDown={() => startHold(-1)}
                            onMouseUp={stopHold}
                            onMouseLeave={stopHold}
                            onTouchStart={(e) => { e.preventDefault(); startHold(-1); }}
                            onTouchEnd={stopHold}
                        >
                            -
                        </button>
                    )}
                    <input
                        type="number" step={step} min={min} max={max}
                        className={`${styles.formControl} ${baseStyles.capacityValue}`}
                        value={value}
                        disabled={disabled}
                        onChange={(e) => onChange(e.target.value === '' ? '' : parseInt(e.target.value))}
                    />
                    {!disabled && (
                        <button
                            type="button"
                            data-no-sound="true"
                            className={`${styles.capacityBtn} ${baseStyles.capacityBtn}`}
                            onMouseDown={() => startHold(1)}
                            onMouseUp={stopHold}
                            onMouseLeave={stopHold}
                            onTouchStart={(e) => { e.preventDefault(); startHold(1); }}
                            onTouchEnd={stopHold}
                        >
                            +
                        </button>
                    )}
                </PixelBox>
            </div>
        </div>
    );
};