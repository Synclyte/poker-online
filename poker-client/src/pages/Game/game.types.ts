import React from 'react';

export type CardView =
    | { visibility: "hidden" }
    | { visibility: "visible"; value: string };

export type PlayerAction =
    | { type: "raise"; amount: number }
    | { type: "fold" }
    | { type: "call" }
    | { type: "timeout" }
    | {
        type: "playSpecial";
        card: string;
        targetId?: number;
        cardIndex?: number;
    }
    | {
        type: "discardSpecial";
        cardIndex: number;
    };

export type MoveAction =
    | { type: "action"; action: PlayerAction }
    | { type: "dealHole"; count: number }
    | { type: "removeHole"; index: number }
    | { type: "dealCommunity"; count: number }
    | { type: "removeCommunity"; index: number }
    | { type: "dealSpecial"; count: number }
    | {
        type: "revealCard";
        targetId: number;
        cardIndex: number;
    }
    | { type: "swapBot" }
    | { type: "swapHuman" }
    | { type: "replaceCommunity"; index: number }
    | { type: "replaceHole"; index: number };

export interface GameConfig {
    maxPlayers: number;
    blindSize: number;
    minRaise: number;
    startingChips: number;
    specialCardLimit: number;
    deckType: string;
    turnTimeout: number;
    isPrivate: boolean;
    roundLimit: number;
    maxBetMultiplier?: number;
}

export interface MoveEvent {
    actorId: number | null;
    action: MoveAction;
    private: boolean;
}

export interface PlayerState {
    id: number;
    chips: number;
    totalBet: number;
    roundBet: number;
    folded: boolean;
    acted: boolean;
    isTurn: boolean;
    isDealer: boolean;
    holeCards: (CardView | null)[];
    specialCards: (CardView | null)[];
    handType: string;
}

export interface GameStateData {
    roundName: string;
    communityCards: CardView[];
    pots: number[];
    roundBetSum: number;
    overallSum: number;
    highestBet: number;
    deckCards: number;
    winningHandType: string;
    players: PlayerState[];
    modifiers: unknown[];
    modifierVars: {
        potMult: number;
        anteMult: number;
        gambleSuccessChance?: number;
    };
    gamesPlayed?: number;
    maxBet?: number | null;
}

export interface MoveResult {
    events: MoveEvent[];
    gameState: GameStateData;
}

export interface PlayerLobbyInfo {
    id: number;
    name: string;
    isHost: boolean;
    isBot: boolean;
}

export interface Particle {
    x: number;
    y: number;
    startX: number;
    startY: number;
    endX: number;
    endY: number;
    startTime: number;
    duration: number;
    type: number;
}

export interface ChipImageInfo {
    ref: React.RefObject<HTMLImageElement | null>;
    size: number;
}

export interface SpecialCardInfo {
    label: string;
    description: string;
    target: "none" | "selfCard" | "other" | "otherCard" | "communityCard";
    imgSrc?: string;
    requiresTargetIndex?: boolean;
}

export interface ActiveModifierInfo {
    id: string;
    name: string;
    description: string;
    duration: string;
    imgSrc: string;
}
