import React from 'react';
import { PixelBox } from '../PixelBox/pixelbox';
import styles from '../../pages/Game/game.module.css';
import { ActiveModifierInfo } from '../../pages/Game/game.types';

interface ActiveModifiersProps {
    displayedModifiers: (ActiveModifierInfo & { isRemoving?: boolean })[];
}

export const ActiveModifiers: React.FC<ActiveModifiersProps> = ({ displayedModifiers }) => {
    if (!displayedModifiers || displayedModifiers.length === 0) return null;

    return (
        <div className={styles.activeModifiersContainer}>
            {displayedModifiers.map((mod) => (
                <div
                    key={mod.id}
                    className={[
                        styles.modifierItemWrapper,
                        mod.isRemoving ? styles.removingModifier : "",
                    ].filter(Boolean).join(" ")}
                >
                    <div className={styles.modifierTooltip}>
                        <strong>{mod.name}</strong>
                        <span className={styles.modifierDesc}>{mod.description}</span>
                        <span className={styles.modifierDuration}>{mod.duration}</span>
                    </div>
                    <PixelBox
                        innerClassName={styles.modifierPixelBox}
                        borderColour='var(--special-colour)'
                        backgroundColour="#1a1a24"
                    >
                        <img
                            src={mod.imgSrc}
                            alt={mod.name}
                            onError={(e) => {
                                (e.target as HTMLElement).style.display = "none";
                            }}
                        />
                        <span className={styles.modifierFallbackIcon}>
                            {mod.name.charAt(0)}
                        </span>
                    </PixelBox>
                </div>
            ))}
        </div>
    );
};
