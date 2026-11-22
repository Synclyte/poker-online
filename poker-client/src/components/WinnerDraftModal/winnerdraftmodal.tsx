import React from 'react';
import { PixelBox } from '../PixelBox/pixelbox';
import { SpecialCard, getSpecialCardInfo } from '../Card/card';
import styles from '../../pages/Game/game.module.css';

interface WinnerDraftModalProps {
    options: string[] | null | undefined;
    timeRemaining: number | null;
    blindSize?: number;
    anteMult?: number;
    gambleSuccessChance?: number;
    boxBorderColour: string;
    onPick: (choiceIndex: number) => void;
}

export const WinnerDraftModal: React.FC<WinnerDraftModalProps> = ({
    options,
    timeRemaining,
    blindSize,
    anteMult,
    gambleSuccessChance,
    boxBorderColour,
    onPick,
}) => {
    if (!options || options.length === 0) return null;

    return (
        <div className={styles.winnerDraftOverlay}>
            <PixelBox
                unclipped
                className={styles.winnerDraftPixelBox}
                innerClassName={styles.winnerDraftInner}
                borderColour={boxBorderColour}
                backgroundColour="#1a1a24"
            >
                <div className={styles.winnerDraftHeader}>
                    <h2 className={styles.winnerDraftTitle}>CHOOSE A CARD</h2>
                    {timeRemaining !== null && (
                        <span className={`${styles.winnerDraftTimer} ${timeRemaining <= 3 ? styles.seatTimerLow : ''}`}>
                            {timeRemaining}s
                        </span>
                    )}
                </div>

                <div className={styles.winnerDraftOptions}>
                    {options.map((cardValue, index) => {
                        const info = getSpecialCardInfo(
                            cardValue,
                            blindSize,
                            anteMult,
                            gambleSuccessChance,
                        );

                        return (
                            <button
                                key={`draft-option-${cardValue}-${index}`}
                                type="button"
                                className={styles.winnerDraftOptionBtn}
                                onClick={() => onPick(index)}
                            >
                                <div className={styles.winnerDraftTooltip}>
                                    <strong>{info.label}</strong>
                                    <span className={styles.specialSelectionDesc}>{info.description}</span>
                                </div>
                                <PixelBox
                                    className={styles.winnerDraftOptionBox}
                                    innerClassName={styles.winnerDraftOptionInner}
                                    borderColour={boxBorderColour}
                                    backgroundColour="#232332"
                                >
                                    <div className={styles.winnerDraftCardPreview}>
                                        <SpecialCard card={{ visibility: "visible", value: cardValue }} />
                                    </div>
                                    <div className={styles.winnerDraftOptionName}>{info.label}</div>
                                </PixelBox>
                            </button>
                        );
                    })}
                </div>
            </PixelBox>
        </div>
    );
};
