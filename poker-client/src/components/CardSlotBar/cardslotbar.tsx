import React from 'react';

export interface CardSlotBarProps<T> {
    cards: (T | null)[];
    initialSlotLimit: number;
    containerClassName?: string;
    containerStyle?: React.CSSProperties;
    renderCard: (card: T | null, index: number, isExtraSlot: boolean) => React.ReactNode;
}

export function CardSlotBar<T>({
    cards,
    initialSlotLimit,
    containerClassName,
    containerStyle,
    renderCard,
}: CardSlotBarProps<T>) {
    const totalSlotsCount = Math.max(initialSlotLimit, cards.length);

    return (
        <div className={containerClassName} style={containerStyle}>
            {Array.from({ length: totalSlotsCount }).map((_, idx) => {
                const card = cards[idx] ?? null;
                if (idx >= initialSlotLimit && !card) {
                    return null;
                }
                return renderCard(card, idx, idx >= initialSlotLimit);
            })}
        </div>
    );
}
