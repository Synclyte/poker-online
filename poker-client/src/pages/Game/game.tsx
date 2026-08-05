import React, { useState, useEffect, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useSocket } from '../../context/SocketContext';
import { useToast } from '../../context/ToastContext';
import { PixelBox } from '../../components/PixelBox/pixelbox';
import styles from './Game.module.css';
import { NumberSetting } from '../../components/NumberSetting/numbersetting';

type CardView =
    | { visibility: "hidden" }
    | { visibility: "visible"; value: string };

type PlayerAction =
    | { type: "raise"; amount: number }
    | { type: "fold" }
    | { type: "call" }
    | { type: "timeout" }
    | { type: "endMove" }
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

type MoveAction =
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
    | { type: "swapHuman" };

interface GameConfig {
    maxPlayers: number;
    blindSize: number;
    minRaise: number;
    startingChips: number;
    specialCardLimit: number;
    deckType: string;
    turnTimeout: number;
    isPrivate: boolean;
    roundLimit: number;
}

interface MoveEvent {
    actorId: number | null;
    action: MoveAction;
    private: boolean;
}

interface PlayerState {
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

interface GameStateData {
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
    };
}

interface MoveResult {
    events: MoveEvent[];
    gameState: GameStateData;
}

interface PlayerLobbyInfo {
    id: number;
    name: string;
    isHost: boolean;
    isBot: boolean;
}

interface Particle {
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

interface ChipImageInfo {
    ref: React.RefObject<HTMLImageElement | null>;
    size: number;
}

const readCssVar = (name: string, fallback: number) => {
    const raw = getComputedStyle(document.documentElement).getPropertyValue(name);
    const parsed = parseFloat(raw);
    return Number.isFinite(parsed) ? parsed : fallback;
};

// animation timings, derived from css
const timing = {
    betChipAdded: readCssVar("anim-betchip-add-ms", 100),
    flip: readCssVar("--anim-flip-ms", 320),
    deal: readCssVar("--anim-deal-ms", 450),
    dissolve: readCssVar("--anim-dissolve-ms", 400),
    showdown: readCssVar("anim-showdown-delay-ms", 520),
};

const computedStyles = getComputedStyle(document.documentElement);
const boxBorderColour = computedStyles.getPropertyValue('--border-colour').trim() || "#dcdcdc";

/** Takes a card string in format `rank:suit` (i.e. 6:d) and converts 
* the card into a component representation
*/
function parseCardString(cardStr: string) {
    if (!cardStr) {
        return { isHidden: true, rank: "", suitSymbol: "", color: "" };
    }

    const cleanStr = cardStr.startsWith("*") ? cardStr.substring(1) : cardStr;
    const parts = cleanStr.split(":");
    const rawRank = (parts[0] || "").toLowerCase();
    const rawSuit = (parts[1] || "").toLowerCase();

    const suitMap: { [key: string]: { symbol: string; color: string } } = {
        "s": { symbol: "spade", color: "#000000" },
        "c": { symbol: "club", color: "#000000" },
        "h": { symbol: "heart", color: "#e53935" },
        "d": { symbol: "diamond", color: "#e53935" }
    };

    const displayRank = rawRank.toUpperCase();
    const suitInfo = suitMap[rawSuit] || { symbol: "?", color: "#000000" };

    return {
        isHidden: false,
        rank: displayRank,
        suitSymbol: suitInfo.symbol,
        color: suitInfo.color
    };
}

interface SpecialCardInfo {
    label: string;
    description: string;
    target: "none" | "selfCard" | "other" | "otherCard" | "communityCard";
    imgSrc?: string;
    requiresTargetIndex?: boolean;
}

const specialCardInfo: Record<string, SpecialCardInfo> = {
    ReplaceCardSelf: {
        label: "Replace Hole Card",
        description: "Replace a selected hole card with a new random card",
        target: "selfCard",
        requiresTargetIndex: true,
    },
    DrawCardSelf: {
        label: "Draw Hole Card",
        description: "Draw an additional hole card randomly",
        target: "none",
    },
    ReplaceCardCommunity: {
        label: "Replace Community",
        description: "Replace a community card with a new card",
        target: "communityCard",
        requiresTargetIndex: true,
    },
    RemoveCardCommunity: {
        label: "Remove Community",
        description: "Remove a community card",
        target: "communityCard",
        requiresTargetIndex: true,
    },
    DrawCardCommunity: {
        label: "Draw Community",
        description: "Add a new community card",
        target: "none",
    },
    RevealOpponentCard: {
        label: "Reveal Card",
        description: "Reveal one hole card from an opponent",
        target: "otherCard",
        requiresTargetIndex: true,
    },
    WithdrawBet: {
        label: "Withdraw Bet",
        description: "Fold and recover your current round bet. Unaffected by multipliers",
        target: "none",
    },
    AnteUp: {
        label: "Ante Up",
        description: "Permanently double the blind cost",
        target: "none",
    },
    PotMult: {
        label: "Pot Multiplier",
        description: "Multiply the blind for the entirety of the current hand",
        target: "none",
    },
    RaiseBlock: {
        label: "Raise Block",
        description: "Prevent all raises until your next turn",
        target: "none",
    },
    SpecialBlock: {
        label: "Special Block",
        description: "Prevent all special cards from being played until your next turn",
        target: "none",
    },
    ChipBoost: {
        label: "Chip Boost",
        description: "Gain chips proportional to the blind cost",
        target: "none",
    },
    ChipGamble: {
        label: "Chip Gamble",
        description: "Has a chance to give a significant number of chips proportional to blind cost or remove all chips. Failure chance increases with every use",
        target: "none",
    },
    Blackjack: {
        label: "Blackjack",
        description: "Use Blackjack scoring for the current hand. Provides a Draw Hole Card special card to all players on use",
        target: "none",
    },
    DrawHeart: {
        label: "Draw Heart",
        description: "Force the next drawn community card to have the Heart suit",
        target: "none",
    },
    DrawSpade: {
        label: "Draw Spade",
        description: "Force the next drawn community card to have the Spade suit",
        target: "none",
    },
    DrawDiamond: {
        label: "Draw Diamond",
        description: "Force the next drawn community card to have the Diamond suit",
        target: "none",
    },
    DrawClub: {
        label: "Draw Club",
        description: "Force the next drawn community card to have the Club suit",
        target: "none",
    },
    DrawFace: {
        label: "Draw Face",
        description: "Force the next drawn community card to be a Face card",
        target: "none",
    },
    DrawHigh: {
        label: "Draw High",
        description: "Force the next community card to be a 10 or above",
        target: "none",
    },
    DrawLow: {
        label: "Draw Low",
        description: "Force the next community card to be a 5 or below",
        target: "none",
    },
    InvalidateFlush: {
        label: "No Flushes",
        description: "Invalidate flushes for this round.",
        target: "none",
    },
    InvalidateStraight: {
        label: "No Straights",
        description: "Invalidate straights for this round.",
        target: "none",
    },
    InvalidateThreeOfAKind: {
        label: "No Three of a Kind",
        description: "Invalidate three of a kind for this round.",
        target: "none",
    },
    InvalidateTwoPair: {
        label: "No Two Pair",
        description: "Invalidate two-pair hands for this round.",
        target: "none",
    },
    InvalidateFullHouse: {
        label: "No Full House",
        description: "Invalidate full houses for this round.",
        target: "none",
    },
    Special: {
        label: "Special",
        description: "Draw multiple special cards. Can exceed the card limit",
        target: "none",
    },
    DrawHigherThanLast: {
        label: "Draw Higher",
        description: "Force the next drawn community card to have a rank equal or higher than the previous draw",
        target: "none"
    },
    DrawLowerThanLast: {
        label: "Draw Lower",
        description: "Force the next drawn community card to have a rank equal or lower than the previous draw",
        target: "none"
    },
    DrawSameSuitAsLast: {
        label: "Draw Same Suit",
        description: "Force the next drawn community card to be the same suit as the previous draw",
        target: "none"
    },
};

function getSpecialCardInfo(cardStr: string): SpecialCardInfo {
    return specialCardInfo[cardStr] ?? {
        label: cardStr,
        description: "No description",
        target: "none",
    };
}

const handNameMap: Record<string, string> = {
    royalflush: "Royal Flush",
    straightflush: "Straight Flush",
    fourofakind: "Four of a Kind",
    fullhouse: "Full House",
    flush: "Flush",
    straight: "Straight",
    threeofakind: "Three of a Kind",
    twopair: "Two Pair",
    pair: "Pair",
    highcard: "High Card",
    none: "Nothing",
};

export function getSanitisedHandName(internalName: string): string {
    return handNameMap[internalName.toLowerCase()] ?? internalName;
}

export interface ActiveModifierInfo {
    id: string;
    name: string;
    description: string;
    duration: string;
    imgSrc: string;
}

function parseModifier(mod: any, index: number): ActiveModifierInfo {
    let name = "Active Modifier";
    let description = "An active game modifier.";
    let duration = "Active";
    let typeKey = "unknown";

    if (typeof mod.effect === "string") {
        const eff = mod.effect.toLowerCase();
        if (eff.includes("raisesblocked")) {
            name = "Raises Blocked";
            description = "Raises are prevented";
            typeKey = "raises_blocked";
        } else if (eff.includes("specialsblocked")) {
            name = "Specials Blocked";
            description = "Special cards cannot be played";
            typeKey = "specials_blocked";
        } else if (eff.includes("blackjackscoring")) {
            name = "Blackjack Scoring";
            description = "Closest hand sum to 21 wins";
            typeKey = "blackjack_scoring";
        }
    } else if (mod.effect && typeof mod.effect === "object") {
        if ("potMultiplier" in mod.effect || "PotMultiplier" in mod.effect) {
            const val = mod.effect.potMultiplier?.multiplier ?? mod.effect.PotMultiplier?.multiplier ?? 2;
            name = `Pot Multiplier (${val}x)`;
            description = `Multiplies all pot gains by ${val}x.`;
            typeKey = "pot_multiplier";
        } else if ("anteMultiplier" in mod.effect || "AnteMultiplier" in mod.effect) {
            const val = mod.effect.anteMultiplier?.multiplier ?? mod.effect.AnteMultiplier?.multiplier ?? 1.5;
            name = `Ante Multiplier (${val}x)`;
            description = `Multiplies minimum bets and blinds by ${val}x.`;
            typeKey = "ante_multiplier";
        } else if ("invalidateHand" in mod.effect || "InvalidateHand" in mod.effect) {
            const rawHand = mod.effect.invalidateHand ?? mod.effect.InvalidateHand ?? "Hand";
            const handName = getSanitisedHandName(typeof rawHand === "string" ? rawHand : String(rawHand ?? "Hand"));
            name = `${handName} Disabled`;
            description = `${handName} hands are disabled and cannot score`;
            typeKey = "invalidate_hand";
        } else if ("forceCommunityDraw" in mod.effect || "ForceCommunityDraw" in mod.effect) {
            const rule = mod.effect.forceCommunityDraw ?? mod.effect.ForceCommunityDraw;
            const ruleStr = typeof rule === "string" ? rule : String(rule ?? "");

            switch (ruleStr) {
                case "Heart":
                    name = "Draw Heart";
                    description = "Community card draws forced to be Hearts";
                    typeKey = "draw_heart";
                    break;
                case "Spade":
                    name = "Draw Spade";
                    description = "Community card draws forced to be Spades";
                    typeKey = "draw_spade";
                    break;
                case "Diamond":
                    name = "Draw Diamond";
                    description = "Community card draws forced to be Diamonds";
                    typeKey = "draw_diamond";
                    break;
                case "Club":
                    name = "Draw Club";
                    description = "Community card draws forced to be Clubs";
                    typeKey = "draw_club";
                    break;
                case "Face":
                    name = "Draw Face Card";
                    description = "Community card draws forced to be Face cards";
                    typeKey = "draw_face";
                    break;
                case "High":
                    name = "Draw High Card";
                    description = "Community card draws forced to be 10 or higher";
                    typeKey = "draw_high";
                    break;
                case "Low":
                    name = "Draw Low Card";
                    description = "Community card draws forced to be 5 or lower";
                    typeKey = "draw_low";
                    break;
                case "HigherThanLast":
                    name = "Draw Higher";
                    description = "Community card draws forced to be rank equal or higher than previous draw";
                    typeKey = "draw_higher_than_last";
                    break;
                case "LowerThanLast":
                    name = "Draw Lower";
                    description = "Community card draws forced to be rank equal or lower than previous draw";
                    typeKey = "draw_lower_than_last";
                    break;
                case "SameSuitAsLast":
                    name = "Draw Same Suit";
                    description = "Community card draws forced to be same suit as previous draw";
                    typeKey = "draw_same_suit_as_last";
                    break;
                default:
                    name = `Draw ${ruleStr}`;
                    description = `Community card draws forced to follow ${ruleStr} rule`;
                    typeKey = `draw_${ruleStr.toLowerCase()}`;
                    break;
            }
        }
    }

    if (typeof mod.expiry === "string") {
        if (mod.expiry.toLowerCase().includes("gameend")) {
            duration = "Duration: Game";
        }
    } else if (mod.expiry && typeof mod.expiry === "object") {
        if ("onCommunityDraw" in mod.expiry || "OnCommunityDraw" in mod.expiry) {
            const count = mod.expiry.onCommunityDraw?.count ?? mod.expiry.OnCommunityDraw?.count ?? 1;
            duration = `Duration: ${count} Community Draw${count > 1 ? "s" : ""}`;
        } else if ("onPlayerTurn" in mod.expiry || "OnPlayerTurn" in mod.expiry) {
            const count = mod.expiry.onPlayerTurn?.count ?? mod.expiry.OnPlayerTurn?.count ?? 1;
            duration = `Duration: ${count} Turn${count > 1 ? "s" : ""}`;
        } else if ("onRoundEnd" in mod.expiry || "OnRoundEnd" in mod.expiry) {
            const count = mod.expiry.onRoundEnd?.count ?? mod.expiry.OnRoundEnd?.count ?? 1;
            duration = `Duration: ${count} Round${count > 1 ? "s" : ""}`;
        } else if ("onHandEnd" in mod.expiry || "OnHandEnd" in mod.expiry) {
            const count = mod.expiry.onHandEnd?.count ?? mod.expiry.OnHandEnd?.count ?? 1;
            duration = `Duration: ${count} Hand${count > 1 ? "s" : ""}`;
        }
    }

    return {
        id: `modifier-${typeKey}-${index}`,
        name,
        description,
        duration,
        imgSrc: `/src/assets/modifiers/modifier_${typeKey}.png`,
    };
}

// Playing card component
// Contains a deal animation (flying from card stack) and flip animation
interface CardProps {
    card: CardView | null;
    cardType: "playing" | "special";
    selected?: boolean;
    onClick?: () => void;
    innerRef?: React.Ref<HTMLDivElement>;
    slotRef?: React.Ref<HTMLDivElement>;
    slotAttribute?: string;
}

/** Playing card component representation.
*   Handles rendering the card
*/
const Card: React.FC<CardProps> = ({
    card,
    cardType,
    selected = false,
    onClick,
    innerRef,
    slotRef,
    slotAttribute,
}) => {
    const className = cardType === "special" ? styles.specialCardSlot : styles.cardSlot;
    const isFaceUp = card?.visibility === "visible";

    return (
        <div
            ref={slotRef}
            data-slot-attr={slotAttribute}
            className={[className, selected ? styles.selectedSpecialCard : ""].filter(Boolean).join(" ")}
            onClick={onClick}
            role={onClick ? "button" : undefined}
            tabIndex={onClick ? 0 : undefined}
            onKeyDown={event => {
                if (onClick && (event.key === "Enter" || event.key === " ")) {
                    event.preventDefault();
                    onClick();
                }
            }}
        >
            <div
                ref={innerRef}
                className={[styles.cardInner, isFaceUp ? styles.cardFlipped : ""].filter(Boolean).join(" ")}
            >
                {card && (
                    <CardFaces
                        card={card}
                        cardType={cardType}
                    />
                )}
            </div>
        </div>
    );
};

interface CardFacesProps {
    card: CardView;
    cardType: "playing" | "special";
}

const CardFaces: React.FC<CardFacesProps> = ({ card, cardType }) => {
    const cardValue = "value" in card ? card.value : null;
    const playingInfo = cardValue && cardType === "playing" ? parseCardString(cardValue) : null;
    const specialInfo = cardValue && cardType === "special" && cardValue ? getSpecialCardInfo(cardValue) : null;
    const isJoker = cardType === "playing" && cardValue?.startsWith("?") === true;

    return (
        <>
            <div
                className={[
                    styles.cardBackFace,
                    cardType === "special" ? styles.specialBackFace : styles.playingBackFace]
                    .filter(Boolean).join(" ")}
            >
                {cardType === "special" && (
                    <img src="/src/assets/special/specialback.png" alt="" draggable={false} />
                )}
            </div>

            <div
                className={[styles.cardFront, cardType === "playing" && isJoker ? styles.joker : ""].filter(Boolean).join(" ")}
                style={cardType === "playing" ? { color: playingInfo?.color } : undefined}
            >
                {cardType === "special" &&
                    (specialInfo?.imgSrc ? (
                        <img src={specialInfo.imgSrc} alt={specialInfo.label} draggable={false} />
                    ) : (
                        <span>{specialInfo?.label ?? "Special"}</span>
                    ))}

                {cardType === "playing" && playingInfo && !isJoker && (
                    <>
                        <span>{playingInfo?.rank}</span>
                        <img
                            src={`/src/assets/suits/${playingInfo?.suitSymbol}.png`}
                            alt=""
                            draggable={false}
                        />
                    </>
                )}

                {cardType === "playing" && isJoker && (
                    <>
                        <span className={styles.jokerTop}>Joker</span>
                        <span className={styles.jokerBottom}>Joker</span>
                    </>
                )}
            </div>
        </>
    );
};

const PlayingCard: React.FC<Omit<CardProps, "cardType">> = props => (<Card {...props} cardType="playing" />);
const SpecialCard: React.FC<Omit<CardProps, "cardType">> = props => (<Card {...props} cardType="special" />);

export function Game() {
    const { roomId } = useParams<{ roomId: string }>();
    const { socket, isConnected } = useSocket();
    const { showToast } = useToast();
    const navigate = useNavigate();

    const [isAnimating, setIsAnimating] = useState(false);

    const [config, setConfig] = useState<GameConfig | null>(null);
    const [myId, setMyId] = useState<number | null>(null);
    const myIdRef = useRef<number | null>(null);

    useEffect(() => {
        myIdRef.current = myId;
    }, [myId]);
    const [gameState, setGameState] = useState<GameStateData | null>(null);
    const [lobbyPlayers, setLobbyPlayers] = useState<PlayerLobbyInfo[]>([]);
    const [raiseAmount, setRaiseAmount] = useState<number>(10);

    const [activeMove, setActiveMove] = useState<MoveEvent | null>(null);

    const [selectedSpecialTarget, setSelectedSpecialTarget] = useState<number | null>(null);
    const [selectedSpecialCardIndex, setSelectedSpecialCardIndex] = useState<number | null>(null);
    const [selectedTargetCardIndex, setSelectedTargetCardIndex] = useState<number | null>(null);
    const [hoveredSpecialIndex, setHoveredSpecialIndex] = useState<number | null>(null);

    const specialDeckRef = useRef<HTMLDivElement | null>(null);

    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const activeParticlesRef = useRef<Particle[]>([]);
    const animFrameRef = useRef<number | null>(null);
    const chipImageOne = useRef<HTMLImageElement | null>(null);
    const chipImageTen = useRef<HTMLImageElement | null>(null);
    const chipImageFifty = useRef<HTMLImageElement | null>(null);
    const chipImageTwoHundred = useRef<HTMLImageElement | null>(null);
    const chipImageOneThousand = useRef<HTMLImageElement | null>(null);
    const chipImageDict = useRef<{ [key: number]: ChipImageInfo }>(null);

    const seatRefs = useRef<{ [key: number]: HTMLDivElement | null }>({});

    const eventQueueRef = useRef<{ events: MoveEvent[]; nextState: GameStateData }[]>([]);
    const gameStateRef = useRef<GameStateData | null>(gameState);
    const isProcessingQueue = useRef<boolean>(false);

    const deckRef = useRef<HTMLDivElement | null>(null);
    const potRef = useRef<HTMLDivElement | null>(null);

    const meInGame = gameState?.players.find(p => p.id === myId);
    const meInLobby = lobbyPlayers.find(p => p.id === myId);
    const isHost = meInLobby?.isHost || false;

    const cardInnerRefs = useRef<Record<string, HTMLDivElement | null>>({});
    const communitySlotRefs = useRef<Record<number, HTMLDivElement | null>>({});
    const holeSlotRefs = useRef<Record<string, HTMLDivElement | null>>({});
    const specialSlotRefs = useRef<Record<number, HTMLDivElement | null>>({});

    const selectedSpecialCard = selectedSpecialCardIndex === null ? null
        : meInGame?.specialCards[selectedSpecialCardIndex] ?? null;

    const selectedSpecialCardValue = selectedSpecialCard?.visibility === "visible"
        ? selectedSpecialCard.value : null;

    // image-related structs and functions
    // structured this way such that new chip sizes do not require
    // significant modification to any other functions
    const imageDict = {
        1: { ref: chipImageOne, size: 24 },
        10: { ref: chipImageTen, size: 32 },
        50: { ref: chipImageFifty, size: 52 },
        200: { ref: chipImageTwoHundred, size: 64 },
        1000: { ref: chipImageOneThousand, size: 80 },
    }
    const getChipCounts = (rawAmount: number) => {
        let remainingChips = rawAmount;

        let oneThousands = Math.max(0, Math.floor((remainingChips - 7500) / Math.max(1000, 1250 - rawAmount / 250)));
        remainingChips -= oneThousands * 1000;

        let twohundreds = Math.max(0, Math.floor((remainingChips - 1500) / Math.max(200, 250 - rawAmount / 200)));
        remainingChips -= twohundreds * 200;

        let fifties = Math.max(0, Math.floor((remainingChips - 150) / Math.max(50, 70 - rawAmount / 75)));
        remainingChips -= fifties * 50;

        let tens = Math.floor(remainingChips / 10);
        remainingChips -= tens * 10;

        let ones = remainingChips;

        return {
            1: ones,
            10: tens,
            50: fifties,
            200: twohundreds,
            1000: oneThousands,
        }
    }
    const loadImg = (ref: React.RefObject<HTMLImageElement | null>, src: string) => {
        const img = new Image();
        img.src = src;
        img.onload = () => {
            ref.current = img;
        }
    }
    useEffect(() => {
        loadImg(chipImageOne, '/src/assets/chips/chip1.png');
        loadImg(chipImageTen, '/src/assets/chips/chip10.png');
        loadImg(chipImageFifty, '/src/assets/chips/chip50.png');
        loadImg(chipImageTwoHundred, '/src/assets/chips/chip200.png');
        loadImg(chipImageOneThousand, '/src/assets/chips/chip1000.png');
        chipImageDict.current = imageDict;
    }, []);

    useEffect(() => {
        gameStateRef.current = gameState;
    }, [gameState]);

    useEffect(() => {
        if (config && raiseAmount === 10) {
            setRaiseAmount(config.minRaise);
        }
    }, [config]);

    // chip flying animation handler 
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const resizeCanvas = () => {
            if (canvas.parentElement) {
                canvas.width = canvas.parentElement.clientWidth;
                canvas.height = canvas.parentElement.clientHeight;
            }
        };

        resizeCanvas();
        window.addEventListener('resize', resizeCanvas);

        const render = (timestamp: number) => {
            ctx.clearRect(0, 0, canvas.width, canvas.height);

            const imgDict = chipImageDict.current;
            if (imgDict) {
                activeParticlesRef.current.sort((a, b) => b.type - a.type);

                const particles = activeParticlesRef.current;
                for (let i = particles.length - 1; i >= 0; i--) {
                    const p = particles[i];
                    const elapsed = timestamp - p.startTime;
                    if (elapsed < 0) continue;

                    const progress = Math.min(1, elapsed / p.duration);
                    const ease = 1 - (1 - progress) * (1 - progress);

                    const currentX = p.startX + (p.endX - p.startX) * ease;
                    const currentY = p.startY + (p.endY - p.startY) * ease;
                    const chipImg = imgDict[p.type];

                    if (chipImg) {
                        let currentScale = 1;
                        if (progress < 0.2) {
                            currentScale = (1 / 0.2) * progress;
                        } else if (progress > 0.7) {
                            currentScale = 1 - (1 / 0.3) * (progress - 0.7);
                        }
                        const chipSize = chipImg.size * currentScale;

                        ctx.save();
                        if (chipImg.ref.current) {
                            ctx.drawImage(
                                chipImg.ref.current,
                                currentX - chipSize / 2,
                                currentY - chipSize / 2,
                                chipSize,
                                chipSize
                            );
                        }
                        ctx.restore();
                    }

                    if (progress >= 1) {
                        particles.splice(i, 1);
                    }
                }
            }

            animFrameRef.current = requestAnimationFrame(render);
        };

        animFrameRef.current = requestAnimationFrame(render);

        return () => {
            window.removeEventListener('resize', resizeCanvas);
            if (animFrameRef.current) cancelAnimationFrame(animFrameRef.current);
        };
    }, []);

    useEffect(() => {
        if (selectedSpecialCardIndex !== null && meInGame?.specialCards[selectedSpecialCardIndex]?.visibility !== "visible") {
            setSelectedSpecialCardIndex(null);
            setSelectedSpecialTarget(null);
            setSelectedTargetCardIndex(null);
        }
    }, [meInGame?.specialCards, selectedSpecialCardIndex]);

    const spawnChipStream = (start: string, end: string, rawAmount: number) => {
        const canvas = canvasRef.current;
        if (!canvas || rawAmount <= 0) return 0;

        // string -> position lookup for relevant targets (pot + players)
        const positionMap: Record<string, HTMLElement | null> = {
            'pot': potRef.current,
            ...Object.fromEntries(
                Object.entries(seatRefs.current).map(([id, elem]) => [`player-${id}`, elem])
            )
        };

        const startElem = positionMap[start];
        const endElem = positionMap[end];

        if (!startElem || !endElem) return 0;

        const canvasRect = canvas.getBoundingClientRect();
        const startRect = startElem.getBoundingClientRect();
        const endRect = endElem.getBoundingClientRect();

        const scaleX = canvas.width / (canvasRect.width || 1);
        const scaleY = canvas.height / (canvasRect.height || 1);

        const startX = ((startRect.left + startRect.width / 2) - canvasRect.left) * scaleX;
        const startY = ((startRect.top + startRect.height / 2) - canvasRect.top) * scaleY;

        const endX = ((endRect.left + endRect.width / 2) - canvasRect.left) * scaleX;
        const endY = ((endRect.top + endRect.height / 2) - canvasRect.top) * scaleY;

        const chipCounts = getChipCounts(rawAmount);
        // chip delay - calculated from total number of chips. determines animation time
        const chipDelay = Math.pow(Object.values(chipCounts).reduce((sum, val) => sum + val, 0), 0.7);
        // full formula - uses chipdelay alongside the progress fraction (0-1) to determine exact starting time of chip
        const delayFormula = (frac: number) => { return frac * chipDelay * 70 };
        const now = performance.now();

        const addChips = (type: number, count: number) => {
            for (let i = 0; i < count; i++) {
                const jitterX = (Math.random() - 0.5) * Math.min(chipDelay ** 1.5, 80);
                const jitterY = (Math.random() - 0.5) * Math.min(chipDelay ** 1.5, 80);

                activeParticlesRef.current.push({
                    x: startX + jitterX,
                    y: startY + jitterY,
                    startX: startX + jitterX,
                    startY: startY + jitterY,
                    endX: endX + jitterX / 2,
                    endY: endY + jitterY / 2,
                    // 0.5 added to center particle spawn times
                    startTime: now + delayFormula((i + 0.5) / count),
                    duration: 450 + Math.random() * 150,
                    type
                });
            }
        }

        for (let [type, count] of Object.entries(chipCounts)) {
            addChips(+type, count);
        }

        return delayFormula(1);
    };

    // used for multi-event chains - updates the intermediate values such that animations appear to modify
    // actual values
    const updateIntermediateState = async (playerId: number, player_chip_delta: number) => {
        const newState = JSON.parse(JSON.stringify(gameStateRef.current)) as GameStateData;
        if (!newState) return;

        const player = newState.players.find(p => p.id === playerId);
        if (!player) return;

        player.chips += player_chip_delta;
        newState.overallSum = Math.max(newState.overallSum - player_chip_delta, 0);

        setGameState(newState);
        gameStateRef.current = newState;
    }

    const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
    const nextFrame = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));

    // main event animation handler function
    const processEventQueue = async (): Promise<void> => {
        if (isProcessingQueue.current || eventQueueRef.current.length === 0) {
            return;
        }

        isProcessingQueue.current = true;
        setIsAnimating(true);

        try {
            while (eventQueueRef.current.length > 0) {
                const update = eventQueueRef.current.shift();
                if (!update) continue;

                const { events, nextState } = update;
                const previousState = gameStateRef.current;
                const enteredPreflop = nextState.roundName === "preflop" && previousState?.roundName !== "preflop";
                const enteredShowdown = nextState.roundName === "showdown" && previousState?.roundName !== "showdown";

                const applyState = (state: GameStateData) => {
                    gameStateRef.current = state;
                    setGameState(state);
                };

                let deckCardsDealtInBatch = 0;
                for (const ev of events) {
                    if (ev.action.type === "dealHole" || ev.action.type === "dealCommunity") {
                        deckCardsDealtInBatch += ev.action.count;
                    }
                }

                const clearTableStateForNewHand = async (nextState: GameStateData) => {
                    if (!gameStateRef.current) return;

                    const clearedState: GameStateData = {
                        ...gameStateRef.current,
                        roundName: nextState.roundName || "preflop",
                        communityCards: [],
                        overallSum: 0,
                        deckCards: (nextState.deckCards ?? 52) + deckCardsDealtInBatch,
                        players: gameStateRef.current.players.map(p => ({
                            ...p,
                            holeCards: [],
                            handType: '',
                            totalBet: 0,
                            folded: p.chips === 0,
                        })),
                    };

                    applyState(structuredClone(clearedState));
                    await nextFrame();
                };

                if (enteredPreflop || (previousState && previousState.roundName === "showdown")) {
                    await clearTableStateForNewHand(nextState);
                }

                const animateInitialBlinds = async () => {
                    if (!enteredPreflop || !previousState) {
                        return;
                    }

                    for (const player of nextState.players) {
                        const blindAmount = player.roundBet;

                        if (blindAmount <= 0) continue;

                        const delay = spawnChipStream(`player-${player.id}`, "pot", blindAmount,);

                        await wait(delay + timing.betChipAdded);
                        await updateIntermediateState(player.id, -blindAmount);
                    }
                };



                const triggerInPlaceAnimation = async (
                    element: HTMLElement | null,
                    source: HTMLElement | null,
                    animClass: string,
                    durationMs: number,
                ) => {
                    if (!element) return;

                    const targetRect = element.getBoundingClientRect();
                    let sourceRect = source ? source.getBoundingClientRect() : null;

                    if (!sourceRect || (sourceRect.left === 0 && sourceRect.top === 0 && sourceRect.width === 0)) {
                        const fallbackSource = deckRef.current ?? specialDeckRef.current;
                        if (fallbackSource) {
                            sourceRect = fallbackSource.getBoundingClientRect();
                        }
                    }

                    if (!sourceRect || (sourceRect.left === 0 && sourceRect.top === 0 && sourceRect.width === 0)) {
                        const feltElem = document.querySelector(`.${styles.pokerFelt}`);
                        if (feltElem) {
                            const feltRect = feltElem.getBoundingClientRect();
                            sourceRect = new DOMRect(feltRect.left + feltRect.width / 2 - 31, feltRect.top + feltRect.height / 2 - 45, 62, 90);
                        } else {
                            sourceRect = new DOMRect(window.innerWidth / 2 - 31, window.innerHeight / 2 - 45, 62, 90);
                        }
                    }

                    const dealX = sourceRect.left - targetRect.left;
                    const dealY = sourceRect.top - targetRect.top;

                    element.style.setProperty("--deal-x", `${dealX}px`);
                    element.style.setProperty("--deal-y", `${dealY}px`);

                    element.classList.remove(styles.animatingDeal, styles.animatingDealFlipped, styles.animatingDissolve, styles.animatingFlip);
                    void element.offsetWidth;
                    element.classList.add(animClass);

                    await wait(durationMs);

                    element.classList.remove(animClass);
                    element.style.removeProperty("--deal-x");
                    element.style.removeProperty("--deal-y");
                };

                const playSpecialCardUseAnimation = async (actorId: number, cardValue: string) => {
                    const feltElem = document.querySelector(`.${styles.pokerFelt}`);
                    if (!feltElem) return;
                    const feltRect = feltElem.getBoundingClientRect();

                    const centerBoardElem = document.querySelector(`.${styles.centerBoard}`);
                    const centerRect = centerBoardElem ? centerBoardElem.getBoundingClientRect() : feltRect;
                    const targetX = (centerRect.left + centerRect.width / 2) - feltRect.left - 31;
                    const targetY = (centerRect.top + centerRect.height / 2) - feltRect.top - 45;

                    const currentMyId = myIdRef.current ?? myId;
                    const isSelf = actorId === currentMyId;

                    let startX = targetX;
                    let startY = targetY;
                    let slotElem: HTMLElement | null = null;

                    if (isSelf) {
                        slotElem = document.querySelector(`[data-slot-attr="special-${selectedSpecialCardIndex ?? 0}"]`) ?? document.querySelector(`[data-slot-attr^="special-"]`);
                        if (slotElem) {
                            const slotRect = slotElem.getBoundingClientRect();
                            startX = slotRect.left - feltRect.left;
                            startY = slotRect.top - feltRect.top;
                            slotElem.style.opacity = "0";
                        }
                    } else {
                        const seatElem = document.querySelector(`[data-seat-id="${actorId}"]`);
                        if (seatElem) {
                            const seatRect = seatElem.getBoundingClientRect();
                            startX = (seatRect.left + seatRect.width / 2) - feltRect.left - 31;
                            startY = (seatRect.top + seatRect.height / 2) - feltRect.top - 45;
                        }
                    }

                    const animCard = document.createElement("div");
                    animCard.className = styles.animatedSpecialCardOverlay;
                    animCard.style.setProperty("--start-x", `${startX}px`);
                    animCard.style.setProperty("--start-y", `${startY}px`);
                    animCard.style.setProperty("--end-x", `${targetX}px`);
                    animCard.style.setProperty("--end-y", `${targetY}px`);

                    const info = getSpecialCardInfo(cardValue);
                    animCard.innerHTML = `
                        <div class="${styles.cardInner} ${isSelf ? styles.cardFlipped : ""}">
                            <div class="${styles.cardBackFace} ${styles.specialBackFace}">
                                <img src="/src/assets/special/specialback.png" alt="" draggable="false" />
                            </div>
                            <div class="${styles.cardFront}">
                                ${info.imgSrc ? `<img src="${info.imgSrc}" alt="${info.label}" draggable="false" />` : `<span>${info.label}</span>`}
                            </div>
                        </div>
                    `;

                    feltElem.appendChild(animCard);

                    // controls special card use animations
                    // TODO: refactor such that these are more modular - this is the only place still using
                    // hard coded wait times
                    try {
                        if (isSelf) {
                            animCard.classList.add(styles.animMoveSelf);
                            await wait(550);
                            await wait(300);

                            animCard.classList.remove(styles.animMoveSelf);
                            animCard.classList.add(styles.animDissolveFaceUp);
                            await wait(450);
                        } else {
                            animCard.classList.add(styles.animMoveOther);
                            await wait(550);

                            animCard.classList.remove(styles.animMoveOther);
                            animCard.classList.add(styles.animFlipReveal);
                            await wait(450);
                            await wait(350);

                            animCard.classList.remove(styles.animFlipReveal);
                            animCard.classList.add(styles.animDissolveFlipped);
                            await wait(450);
                        }
                    } finally {
                        if (slotElem) {
                            slotElem.style.opacity = "";
                        }
                        if (animCard.parentNode) {
                            animCard.parentNode.removeChild(animCard);
                        }
                    }
                };

                const animateAction = async (event: MoveEvent, trackedBets: Map<number, number>) => {
                    const action = event.action.type === "action" ? event.action.action : null;

                    if (!action) return;

                    switch (action.type) {
                        case "raise":
                        case "call": {
                            if (event.actorId === null) return;

                            const nextPlayer = nextState.players.find((player) => player.id === event.actorId);

                            if (!nextPlayer) return;

                            const priorBet = trackedBets.get(event.actorId) ?? 0;
                            const targetBet = nextPlayer.totalBet;
                            const betDelta = Math.max(0, targetBet - priorBet);

                            trackedBets.set(event.actorId, targetBet);

                            if (betDelta <= 0) return;

                            const delay = spawnChipStream(`player-${event.actorId}`, "pot", betDelta);
                            await wait(delay + timing.betChipAdded);
                            await updateIntermediateState(event.actorId, -betDelta);

                            return;
                        }

                        case "fold":
                        case "timeout":
                            // fold
                            return;

                        case "endMove":
                            // end turn
                            return;

                        case "playSpecial": {
                            const actorId = event.actorId;
                            const cardValue = action.card;
                            if (actorId !== null && cardValue) {
                                await playSpecialCardUseAnimation(actorId, cardValue);
                            }
                            return;
                        }
                    }
                };

                /**
                 * Animation target resolution fallback
                 */
                const resolveTargetElement = async (refKey: string, selectorFallback?: string): Promise<HTMLDivElement | null> => {
                    let el: HTMLDivElement | null = cardInnerRefs.current[refKey] ?? null;
                    if (!el && selectorFallback) {
                        el = document.querySelector(selectorFallback) as HTMLDivElement | null;
                    }
                    if (!el) {
                        await nextFrame();
                        el = cardInnerRefs.current[refKey] ?? (selectorFallback ? (document.querySelector(selectorFallback) as HTMLDivElement | null) : null);
                    }
                    return el;
                };

                /**
                 * Card deal animation helper
                 */
                const executeCardDealAnimation = async ({
                    activeCards,
                    cardIndex,
                    nextCard,
                    refKey,
                    selectorFallback,
                    source,
                    viewState,
                    applyState,
                }: {
                    activeCards: (CardView | null)[];
                    cardIndex: number;
                    nextCard: CardView;
                    refKey: string;
                    selectorFallback?: string;
                    source: HTMLElement | null;
                    viewState: GameStateData;
                    applyState: (state: GameStateData) => void;
                }) => {
                    const needsFlip = nextCard.visibility === "visible";
                    const hiddenCard: CardView = { visibility: "hidden" };

                    // start with card as face down
                    activeCards.push(hiddenCard);
                    applyState(structuredClone(viewState));
                    await nextFrame();

                    const target = await resolveTargetElement(refKey, selectorFallback);

                    if (target) {
                        // move while face down to the target
                        await triggerInPlaceAnimation(target, source, styles.animatingDeal, timing.deal);

                        // then add information about the card (if it exists) for later
                        activeCards[cardIndex] = nextCard;
                        applyState(structuredClone(viewState));
                        await nextFrame();

                        // then flip the card (if required)
                        if (needsFlip) {
                            await triggerInPlaceAnimation(target, target, styles.animatingFlip, timing.flip);
                        }
                    } else {
                        activeCards[cardIndex] = nextCard;
                        applyState(structuredClone(viewState));
                        await nextFrame();
                    }
                };

                const animateStructuralEvent = async (
                    event: MoveEvent,
                    nextState: GameStateData,
                ): Promise<void> => {
                    if (!gameStateRef.current) return;

                    const applyState = (state: GameStateData) => {
                        gameStateRef.current = state;
                        setGameState(state);
                    };

                    const cloneCurrentState = (): GameStateData =>
                        JSON.parse(JSON.stringify(gameStateRef.current)) as GameStateData;

                    switch (event.action.type) {
                        case "dealHole":
                        case "dealSpecial": {
                            if (event.actorId === null) return;

                            const isSpecial = event.action.type === "dealSpecial";
                            const viewState = cloneCurrentState();
                            const player = viewState.players.find(c => c.id === event.actorId);
                            const nextPlayer = nextState.players.find(c => c.id === event.actorId);
                            if (!player || !nextPlayer) return;

                            const viewCards = isSpecial ? player.specialCards : player.holeCards;
                            const nextCards = isSpecial ? nextPlayer.specialCards : nextPlayer.holeCards;

                            if (nextCards.length < viewCards.length + event.action.count && viewCards.length > 0) {
                                if (isSpecial) {
                                    player.specialCards = [];
                                } else {
                                    player.holeCards = [];
                                }
                                applyState(structuredClone(viewState));
                                await nextFrame();
                            }

                            const activeCards = isSpecial ? player.specialCards : player.holeCards;

                            for (let i = 0; i < event.action.count; i += 1) {
                                const cardIndex = activeCards.length;
                                const nextCard = nextCards[cardIndex];
                                if (!nextCard) break;

                                if (!isSpecial) {
                                    viewState.deckCards = Math.max(0, viewState.deckCards - 1);
                                }

                                const currentMyId = myIdRef.current ?? myId;
                                const toSelf = event.actorId !== null && currentMyId !== null && event.actorId === currentMyId;
                                const source = isSpecial
                                    ? specialDeckRef.current ?? deckRef.current
                                    : deckRef.current;

                                // the special card deal animation varies depending on the target
                                // if it is to an opponent, send the card to the middle of their seat and dissolve
                                if (isSpecial && !toSelf) {
                                    activeCards.push(nextCard);
                                    applyState(structuredClone(viewState));
                                    await nextFrame();

                                    const target = await resolveTargetElement(
                                        `special-seat-${event.actorId}`,
                                        `[data-seat-id="${event.actorId}"] .${styles.cardInner}`
                                    );

                                    if (target) {
                                        await triggerInPlaceAnimation(target, source, styles.animatingDissolve, timing.dissolve);
                                    }
                                // if to client, deal directly to the special card bar
                                } else {
                                    const refKey = isSpecial ? `special-${cardIndex}` : `hole-${event.actorId}-${cardIndex}`;
                                    const selectorFallback = isSpecial
                                        ? `[data-slot-attr="special-${cardIndex}"] .${styles.cardInner}`
                                        : `[data-hole-slot="${event.actorId}:${cardIndex}"] .${styles.cardInner}`;

                                    await executeCardDealAnimation({
                                        activeCards,
                                        cardIndex,
                                        nextCard,
                                        refKey,
                                        selectorFallback,
                                        source,
                                        viewState,
                                        applyState,
                                    });
                                }
                            }

                            return;
                        }

                        case "dealCommunity": {
                            const viewState = cloneCurrentState();

                            if (
                                nextState.communityCards.length <
                                viewState.communityCards.length + event.action.count &&
                                viewState.communityCards.length > 0
                            ) {
                                viewState.communityCards = [];
                                applyState(structuredClone(viewState));
                                await nextFrame();
                            }

                            for (let i = 0; i < event.action.count; i += 1) {
                                const cardIndex = viewState.communityCards.length;
                                const nextCard = nextState.communityCards[cardIndex];
                                if (!nextCard) break;

                                viewState.deckCards = Math.max(0, viewState.deckCards - 1);

                                await executeCardDealAnimation({
                                    activeCards: viewState.communityCards,
                                    cardIndex,
                                    nextCard,
                                    refKey: `community-${cardIndex}`,
                                    selectorFallback: `[data-community-slot="${cardIndex}"] .${styles.cardInner}`,
                                    source: deckRef.current,
                                    viewState,
                                    applyState,
                                });
                            }

                            return;
                        }

                        case "removeHole": {
                            if (event.actorId === null) return;

                            const viewState = cloneCurrentState();
                            const player = viewState.players.find(c => c.id === event.actorId);
                            const index = event.action.index;

                            if (!player || index < 0 || index >= player.holeCards.length) return;

                            player.holeCards.splice(index, 1);
                            applyState(viewState);
                            await nextFrame();
                            return;
                        }

                        case "removeCommunity": {
                            const viewState = cloneCurrentState();
                            const index = event.action.index;

                            if (index < 0 || index >= viewState.communityCards.length) return;

                            viewState.communityCards.splice(index, 1);
                            applyState(viewState);
                            await nextFrame();
                            return;
                        }

                        case "revealCard": {
                            const viewState = cloneCurrentState();
                            const { targetId, cardIndex } = event.action;

                            const player = viewState.players.find(c => c.id === targetId);
                            const nextPlayer = nextState.players.find(c => c.id === targetId);
                            if (!player || !nextPlayer) return;

                            const revealedCard = nextPlayer.holeCards[cardIndex];
                            if (!revealedCard || !player.holeCards[cardIndex]) return;

                            player.holeCards[cardIndex] = revealedCard;
                            applyState(structuredClone(viewState));
                            await nextFrame();

                            const target = cardInnerRefs.current[`hole-${targetId}-${cardIndex}`] ??
                                (holeSlotRefs.current[`${targetId}:${cardIndex}`]?.querySelector(`.${styles.cardInner}`) as HTMLDivElement | null) ?? null;

                            await triggerInPlaceAnimation(target, target, styles.animatingFlip, timing.flip);
                            return;
                        }

                        case "swapBot":
                        case "swapHuman":
                        case "action":
                            return;
                    }
                };

                const animatePotDistribution = async (baseState: GameStateData) => {
                    for (const nextPlayer of nextState.players) {
                        const previousPlayer = baseState.players.find((player) => player.id === nextPlayer.id);

                        if (!previousPlayer) continue;

                        const gain = nextPlayer.chips - previousPlayer.chips;
                        if (gain <= 0) continue;

                        const delay = spawnChipStream("pot", `player-${nextPlayer.id}`, gain);

                        await wait(delay + timing.betChipAdded);
                    }
                };

                if (!previousState) {
                    applyState(nextState);
                    continue;
                }

                await animateInitialBlinds();

                const trackedBets = new Map<number, number>(
                    previousState.players.map((player) => {
                        const nextP = nextState.players.find(p => p.id === player.id);
                        const initialBet = enteredPreflop ? (nextP?.roundBet ?? player.totalBet) : player.totalBet;
                        return [player.id, initialBet];
                    }),
                );

                if (deckCardsDealtInBatch > 0 && gameStateRef.current) {
                    gameStateRef.current.deckCards = (nextState.deckCards ?? 52) + deckCardsDealtInBatch;
                    setGameState({ ...gameStateRef.current });
                }

                for (const event of events) {
                    setActiveMove(event);

                    if (event.action.type === "action") {
                        await animateAction(event, trackedBets);
                    } else {
                        await animateStructuralEvent(event, nextState);
                    }

                    setActiveMove(null);
                }

                if (enteredShowdown) {
                    // showdown has to be done in a complicated manner, as the reveal order and animations need to completely
                    // ignore all other logic surrounding animations through the rest of the file
                    // this is done by generating multiple consecutive pseudo-states to force expected animations

                    // first, card ranks/suits are given values (while not updating anything else), so cards can flip correctly
                    const preFlipState: GameStateData = {
                        ...nextState,
                        overallSum: previousState.overallSum,
                        pots: previousState.pots,
                        players: nextState.players.map(nextPlayer => {
                            const previousPlayer = previousState.players.find(p => p.id === nextPlayer.id);

                            return {
                                ...nextPlayer,
                                chips: previousPlayer?.chips ?? nextPlayer.chips,
                                holeCards: nextPlayer.holeCards.map((nextCard, cardIndex) => {
                                    const prevCard = previousPlayer?.holeCards[cardIndex];
                                    const wasVisible = prevCard?.visibility === "visible";
                                    if (nextCard?.visibility === "visible" && !wasVisible) {
                                        return { ...nextCard, visibility: "hidden" as const };
                                    }
                                    return prevCard ?? nextCard;
                                }),
                            };
                        }),
                    };

                    applyState(structuredClone(preFlipState));
                    await nextFrame();

                    // then execute the flip
                    const flipPromises: Promise<void>[] = [];

                    for (const nextPlayer of nextState.players) {
                        const previousPlayer = previousState.players.find(
                            player => player.id === nextPlayer.id,
                        );

                        for (let cardIndex = 0; cardIndex < nextPlayer.holeCards.length; cardIndex += 1) {
                            const nextCard = nextPlayer.holeCards[cardIndex];
                            const wasVisible =
                                previousPlayer?.holeCards[cardIndex]?.visibility === "visible";

                            if (nextCard?.visibility !== "visible" || wasVisible) continue;

                            const targetElem = cardInnerRefs.current[`hole-${nextPlayer.id}-${cardIndex}`] ??
                                (holeSlotRefs.current[`${nextPlayer.id}:${cardIndex}`]?.querySelector(`.${styles.cardInner}`) as HTMLDivElement | null) ?? null;

                            if (targetElem) {
                                flipPromises.push(
                                    triggerInPlaceAnimation(targetElem, targetElem, styles.animatingFlip, timing.flip)
                                );
                            }
                        }
                    }

                    if (flipPromises.length > 0) {
                        await Promise.all(flipPromises);
                    }

                    // finalize state with visibility "visible" to lock cards face-up
                    const finalRevealState: GameStateData = {
                        ...nextState,
                        overallSum: previousState.overallSum,
                        pots: previousState.pots,
                        players: nextState.players.map(nextPlayer => {
                            const previousPlayer = previousState.players.find(p => p.id === nextPlayer.id);

                            return {
                                ...nextPlayer,
                                chips: previousPlayer?.chips ?? nextPlayer.chips,
                            };
                        }),
                    };

                    applyState(structuredClone(finalRevealState));
                    await nextFrame();

                    await wait(timing.showdown);
                    await animatePotDistribution(previousState);
                }

                // always finish by always updating to the correct final state, to ensure the final state is always accurate
                applyState(nextState);
            }
        } finally {
            setActiveMove(null);
            isProcessingQueue.current = false;
            setIsAnimating(false);

            if (eventQueueRef.current.length > 0) processEventQueue();
        }
    };

    useEffect(() => {
        if (!socket || !isConnected) {
            showToast("Disconnected from server", "error");
            navigate('/');
            return;
        }

        socket.emit("requestGameState");
        socket.emit("getGameConfig");

        const handleClientInfo = (data: { playerId: number }) => {
            myIdRef.current = data.playerId;
            setMyId(data.playerId);
        };
        const handleLobbyUpdate = (data: { players: PlayerLobbyInfo[] }) => setLobbyPlayers(data.players);

        const handleGameUpdate = (data: unknown) => {
            const response = (typeof data === "string" ? JSON.parse(data) : data) as MoveResult;
            eventQueueRef.current.push({ events: response.events ?? [], nextState: response.gameState });
            processEventQueue();
        };

        const handleGameConfig = (data: GameConfig) => setConfig(data);

        const handleInfo = (msg: string) => showToast(msg, "info");
        const handleError = (msg: string) => showToast(msg, "error");
        const handleWarning = (msg: string) => showToast(msg, "warning");

        socket.on("clientInfo", handleClientInfo);
        socket.on("lobbyUpdate", handleLobbyUpdate);
        socket.on("gameUpdate", handleGameUpdate);
        socket.on("info", handleInfo);
        socket.on("error", handleError);
        socket.on("warning", handleWarning);
        socket.on("gameConfig", handleGameConfig);

        return () => {
            socket.off("clientInfo", handleClientInfo);
            socket.off("lobbyUpdate", handleLobbyUpdate);
            socket.off("gameUpdate", handleGameUpdate);
            socket.off("info", handleInfo);
            socket.off("error", handleError);
            socket.off("warning", handleWarning);
        };
    }, [socket, isConnected, navigate, showToast]);

    const startNextRound = () => {
        if (!socket || !isHost) return;
        socket.emit("startGame");
    };

    const leaveGame = () => {
        if (socket) socket.emit("leaveGame");
        navigate('/');
    };

    const selectSpecialTarget = (playerId: number) => {
        if (!selectedSpecialCardValue || playerId === myId) return;

        const info = getSpecialCardInfo(selectedSpecialCardValue);
        if (info.target !== "other" && info.target !== "otherCard") return;

        setSelectedSpecialTarget(playerId);
        setSelectedTargetCardIndex(null);
    };

    const selectTargetCardIndex = (cardIndex: number) => {
        if (!selectedSpecialCardValue) return;

        const info = getSpecialCardInfo(selectedSpecialCardValue);
        if (
            info.target === "communityCard" ||
            info.target === "selfCard" ||
            (info.target === "otherCard" && selectedSpecialTarget !== null)
        ) {
            setSelectedTargetCardIndex(cardIndex);
        }
    };

    const playSelectedSpecialCard = () => {
        if (!socket || !selectedSpecialCardValue) return;

        const info = getSpecialCardInfo(selectedSpecialCardValue);
        let targetId: number | undefined;
        let cardIndex: number | undefined;

        switch (info.target) {
            case "none":
                break;

            case "other":
                if (selectedSpecialTarget == null) {
                    showToast("Select an opponent.", "warning");
                    return;
                }

                targetId = selectedSpecialTarget;
                break;

            case "selfCard":
                if (myId == null || selectedTargetCardIndex == null) {
                    showToast("Select one of your hole cards.", "warning");
                    return;
                }

                targetId = myId;
                cardIndex = selectedTargetCardIndex;
                break;

            case "otherCard":
                if (
                    selectedSpecialTarget == null ||
                    selectedTargetCardIndex == null
                ) {
                    showToast("Select an opponent hole card.", "warning");
                    return;
                }

                targetId = selectedSpecialTarget;
                cardIndex = selectedTargetCardIndex;
                break;

            case "communityCard":
                if (selectedTargetCardIndex == null) {
                    showToast("Select a community card.", "warning");
                    return;
                }

                cardIndex = selectedTargetCardIndex;
                break;
        }

        socket.emit("playerMove", `special ${selectedSpecialCardValue} ${String(targetId ?? 0)} ${String(cardIndex ?? 0)}`);

        cancelSelectedSpecialCard();
    };

    const cancelSelectedSpecialCard = () => {
        setSelectedSpecialCardIndex(null);
        setSelectedSpecialTarget(null);
        setSelectedTargetCardIndex(null);
        setHoveredSpecialIndex(null);
    };

    const discardSelectedSpecialCard = () => {
        if (!socket || selectedSpecialCardIndex == null) return;
        socket.emit("playerMove", `discardspecial ${selectedSpecialCardIndex}`);
        cancelSelectedSpecialCard();
    };

    const isMyTurn = meInGame?.isTurn && gameState?.roundName !== "room" && gameState?.roundName !== "preround" && gameState?.roundName !== "showdown";
    const currentBetToCall = (gameState?.highestBet || 0) - (meInGame?.totalBet || 0);

    const activeModifiers: ActiveModifierInfo[] = (gameState?.modifiers ?? []).map((m: any, idx: number) => parseModifier(m, idx));

    useEffect(() => {
        if (!isMyTurn || gameState?.roundName === "showdown" || gameState?.roundName === "preround" || gameState?.roundName === "room") {
            cancelSelectedSpecialCard();
        }
    }, [gameState?.roundName, isMyTurn]);

    return (
        <div className={styles.gameContainer}>
            <div className={styles.tableHeader}>
                <div>Table Code: {roomId}</div>
                <div className={styles.headerInfo}>
                    <div>Round: {gameState?.roundName ? (gameState.roundName.charAt(0).toUpperCase() + gameState.roundName.slice(1)) : "Waiting"}</div>
                </div>
                <button type="button" className={styles.btnWrapper} onClick={leaveGame}>
                    <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                        Leave Game
                    </PixelBox>
                </button>
            </div>

            {activeModifiers.length > 0 && (
                <div className={styles.activeModifiersContainer}>
                    {activeModifiers.map((mod) => (
                        <div key={mod.id} className={styles.modifierItemWrapper}>
                            <div className={styles.modifierTooltip}>
                                <strong>{mod.name}</strong>
                                <span className={styles.modifierDesc}>{mod.description}</span>
                                <span className={styles.modifierDuration}>{mod.duration}</span>
                            </div>
                            <PixelBox
                                innerClassName={styles.modifierPixelBox}
                                borderColour="var(--input-focus-colour)"
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
            )}

            <div className={styles.pokerFelt}>
                <canvas ref={canvasRef} className={styles.chipCanvas} />

                {gameState?.roundName !== "room" && (
                    <div className={styles.centerBoard}>
                        <div className={styles.deckStack} ref={deckRef}>
                            {(() => {
                                const hiddenCard: CardView = { visibility: "hidden" };
                                const remainingCards = gameState?.deckCards ?? 52;
                                const layerCount = Math.max(1, Math.ceil(remainingCards / 4));

                                return Array.from({ length: layerCount }).map((_, i) => (
                                    <div
                                        key={`deck-layer-${i}`}
                                        className={styles.deckCardLayer}
                                        style={{ transform: `translateY(${-i * 2}px)`, zIndex: i + 1 }}
                                    >
                                        <CardFaces card={hiddenCard} cardType="playing" />
                                    </div>
                                ));
                            })()}
                        </div>

                        <div className={styles.potDisplay} ref={potRef}>
                            Pot: <strong>{gameState?.overallSum || 0}</strong>
                        </div>

                        <div className={styles.specialDeckStack} ref={specialDeckRef}>
                            {(() => {
                                const specialDeckLayerCount = Math.ceil((config?.specialCardLimit ?? 0));
                                const hiddenSpecialCard: CardView = { visibility: "hidden" };

                                return Array.from({ length: Math.max(1, specialDeckLayerCount) }).map((_, i) => (
                                    <div
                                        key={`special-deck-layer-${i}`}
                                        className={styles.specialDeckCard}
                                        style={{ transform: `translateY(${-i * 2}px)`, zIndex: i + 1 }}
                                    >
                                        <CardFaces card={hiddenSpecialCard} cardType="special" />
                                    </div>
                                ));
                            })()}
                        </div>

                        <div className={styles.communityCards}>
                            {Array.from({ length: 5 }).map((_, idx) => {
                                const card = gameState?.communityCards[idx] ?? null;
                                const selectedSpecialInfo = selectedSpecialCardValue ? getSpecialCardInfo(selectedSpecialCardValue) : null;
                                const canTargetCommunity = selectedSpecialInfo?.target === "communityCard" && (card !== null || selectedSpecialCardValue === "DrawCardCommunity");
                                const isSelectedCard = canTargetCommunity && selectedTargetCardIndex === idx;

                                return (
                                    <div
                                        key={`community-slot-${idx}`}
                                        ref={el => { communitySlotRefs.current[idx] = el; }}
                                        className={`
                                            ${styles.cardSlot}
                                            ${canTargetCommunity ? styles.targetableCard : ''}
                                            ${isSelectedCard ? styles.selectedTargetCard : ''}
                                        `}
                                        onClick={() => {
                                            if (canTargetCommunity) {
                                                selectTargetCardIndex(idx);
                                            }
                                        }}
                                    >
                                        {card && (
                                            <PlayingCard
                                                card={card}
                                                innerRef={el => { cardInnerRefs.current[`community-${idx}`] = el; }}
                                            />
                                        )}
                                    </div>
                                );
                            })}
                        </div>
                    </div>
                )}

                {gameState?.players.map((p, seatIndex) => {
                    const totalSeats = gameState.players.length;
                    const angle = (seatIndex / totalSeats) * 2 * Math.PI + Math.PI / 2;
                    const rx = 37;
                    const ry = 33;
                    const left = 50 + rx * Math.cos(angle);
                    const top = 50 + ry * Math.sin(angle);

                    const lobbyInfo = lobbyPlayers.find(lp => lp.id === p.id);
                    const isMe = p.id === myId;
                    const isActing = activeMove?.actorId === p.id;
                    const activePlayerAction = activeMove?.action.type === "action" ? activeMove.action.action : null;
                    const isChecking = isActing && activePlayerAction?.type === "call" && currentBetToCall === 0;

                    const borderColour = (p.isTurn && gameState?.roundName !== "room") ? "var(--input-focus-colour)" : boxBorderColour;

                    const isSelectedTarget = selectedSpecialTarget === p.id;
                    const selectedSpecialInfo = selectedSpecialCardValue ? getSpecialCardInfo(selectedSpecialCardValue) : null;
                    const canTargetPlayer = (selectedSpecialInfo?.target === "other" || selectedSpecialInfo?.target === "otherCard") && p.id !== myId && !p.folded;

                    return (
                        <div
                            key={p.id}
                            data-seat-id={p.id}
                            ref={(el) => { seatRefs.current[p.id] = el; }}
                            className={`
                                ${styles.playerSeat}  
                                ${p.folded || (p.chips === 0 && p.holeCards.length === 0) ? styles.foldedSeat : ''}  
                                ${isChecking ? styles.animatingCheck : ''}  
                                ${canTargetPlayer ? styles.targetableSeat : ''} 
                                ${isSelectedTarget ? styles.selectedTargetSeat : ''}
                            `}
                            style={{ left: `${left}%`, top: `${top}%` }}
                            onClick={() => {
                                if (canTargetPlayer) {
                                    selectSpecialTarget(p.id);
                                }
                            }}
                        >
                            {p.isDealer && gameState?.roundName !== "room" &&
                                <div className={styles.dealerBadgeWrapper}>
                                    <PixelBox innerClassName={styles.dealerBadgeInner} borderColour={borderColour}>
                                        D
                                    </PixelBox>
                                </div>
                            }

                            <PixelBox
                                unclipped
                                innerClassName={styles.seatInner}
                                borderColour={borderColour}
                                backgroundColour={isMe ? "#2a2a38" : "#1a1a24"}
                            >
                                <div className={styles.seatName}>
                                    {lobbyInfo?.name || `Player ${p.id}`} {isMe && "(You)"}
                                </div>
                                <div className={styles.seatChips}>Chips: {p.chips}</div>

                                {gameState?.roundName !== "room" && (
                                    <>
                                        <div className={styles.seatBet}>Bet: {p.totalBet || '0'}</div>

                                        {p.handType ? (
                                            <div className={styles.handTypeTag}>{p.handType}</div>
                                        ) : (
                                            <div className={styles.handPlaceholder}></div>
                                        )}

                                        <div className={styles.holeCards}>
                                            {Array.from({ length: Math.max(2, p.holeCards.length) }, (_, cardIndex) => {
                                                const card = p.holeCards[cardIndex] ?? null;
                                                const canTargetHoleCard = selectedSpecialInfo && (
                                                    (isMe && selectedSpecialInfo.target === "selfCard" && card !== null) ||
                                                    (!isMe && selectedSpecialInfo.target === "otherCard" && selectedSpecialTarget === p.id)
                                                );
                                                const isSelectedHoleCard = canTargetHoleCard && selectedTargetCardIndex === cardIndex;

                                                return (
                                                    <div
                                                        data-hole-slot={`${p.id}:${cardIndex}`}
                                                        ref={element => { holeSlotRefs.current[`${p.id}:${cardIndex}`] = element }}
                                                        key={`player-${p.id}-hole-${cardIndex}`}
                                                        className={`
                                                            ${styles.holeCardSlot}
                                                            ${canTargetHoleCard ? styles.targetableCard : ''}
                                                            ${isSelectedHoleCard ? styles.selectedTargetCard : ''}
                                                        `}
                                                        onClick={(e) => {
                                                            if (canTargetHoleCard) {
                                                                e.stopPropagation();
                                                                selectTargetCardIndex(cardIndex);
                                                            }
                                                        }}
                                                    >
                                                        <PlayingCard
                                                            card={card}
                                                            innerRef={element => {
                                                                cardInnerRefs.current[`hole-${p.id}-${cardIndex}`] = element;
                                                            }}
                                                        />
                                                    </div>
                                                );
                                            })}
                                        </div>

                                        {config && (config.specialCardLimit > 0 || p.specialCards.length > 0) && (
                                            <div className={styles.seatSpecialCards}>
                                                <span className={styles.specialCardCount}>Special x{p.specialCards.length}</span>
                                                <div className={styles.seatSpecialSlot}>
                                                    <SpecialCard
                                                        card={{ visibility: "hidden" }}
                                                        innerRef={element => {
                                                            cardInnerRefs.current[`special-seat-${p.id}`] = element;
                                                        }}
                                                    />
                                                </div>
                                            </div>
                                        )}
                                    </>
                                )}
                            </PixelBox>
                        </div>
                    );
                })}
            </div>

            {gameState?.roundName !== "room" && meInGame && (
                <div className={styles.specialCardBar}>
                    <div className={styles.specialCardHand}>
                        {Array.from({
                            length: Math.max(config?.specialCardLimit ?? 3, meInGame.specialCards.length, 1),
                        }).map((_, index) => {
                            const card = meInGame.specialCards[index] ?? null;
                            const isSelected = selectedSpecialCardIndex === index;

                            return (
                                <div
                                    key={`special-slot-wrapper-${index}`}
                                    className={styles.specialSlotWrapper}
                                    onMouseEnter={() => setHoveredSpecialIndex(index)}
                                    onMouseLeave={() => setHoveredSpecialIndex(null)}
                                >
                                    {card && card.visibility === "visible" && card.value && (isSelected || hoveredSpecialIndex === index) && (
                                        <div
                                            className={[
                                                styles.specialSelectionText,
                                                isSelected ? styles.active : styles.hovered,
                                            ].join(" ")}
                                        >
                                            <strong>{getSpecialCardInfo(card.value).label}</strong>
                                            <span className={styles.specialSelectionDesc}>{getSpecialCardInfo(card.value).description}</span>

                                            {isSelected && (
                                                <>
                                                    {getSpecialCardInfo(card.value).target === "other" &&
                                                        selectedSpecialTarget === null && <span className={styles.specialSelectionInstruction}>Select an opponent</span>}

                                                    {getSpecialCardInfo(card.value).target === "selfCard" &&
                                                        getSpecialCardInfo(card.value).requiresTargetIndex &&
                                                        selectedTargetCardIndex === null && <span className={styles.specialSelectionInstruction}>Select one of your own hole cards</span>}

                                                    {getSpecialCardInfo(card.value).target === "otherCard" &&
                                                        getSpecialCardInfo(card.value).requiresTargetIndex &&
                                                        selectedSpecialTarget !== null &&
                                                        selectedTargetCardIndex === null && <span className={styles.specialSelectionInstruction}>Select an opponent hole card</span>}

                                                    {getSpecialCardInfo(card.value).target === "communityCard" &&
                                                        getSpecialCardInfo(card.value).requiresTargetIndex &&
                                                        selectedTargetCardIndex === null && <span className={styles.specialSelectionInstruction}>Select a community card</span>}
                                                </>
                                            )}
                                        </div>
                                    )}

                                    <SpecialCard
                                        key={`special-slot-${index}`}
                                        slotAttribute={`special-${index}`}
                                        card={card}
                                        slotRef={element => {
                                            specialSlotRefs.current[index] = element;
                                        }}
                                        innerRef={element => {
                                            cardInnerRefs.current[`special-${index}`] = element;
                                        }}
                                        selected={isSelected}
                                        onClick={(card && isMyTurn) ? () => {
                                            setSelectedSpecialCardIndex(
                                                selectedSpecialCardIndex === index ? null : index,
                                            );
                                            setSelectedSpecialTarget(null);
                                            setSelectedTargetCardIndex(null);
                                        } : undefined}
                                    />
                                </div>
                            );
                        })}
                    </div>
                </div>
            )}

            <div className={styles.actionHud}>
                {isMyTurn && gameState?.roundName !== "room" && (
                    <>

                        {selectedSpecialCardValue ? (
                            <div className={styles.turnBar}>

                                <button type="button" className={styles.btnWrapper} disabled={isAnimating} onClick={playSelectedSpecialCard}>
                                    <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                        Play
                                    </PixelBox>
                                </button>

                                <button type="button" className={styles.btnWrapper} disabled={isAnimating} onClick={cancelSelectedSpecialCard}>
                                    <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                        Cancel
                                    </PixelBox>
                                </button>

                                <button type="button" className={styles.btnWrapper} disabled={isAnimating} onClick={discardSelectedSpecialCard}>
                                    <PixelBox innerClassName={`${styles.btnInner} ${styles.btnDanger}`} borderColour={boxBorderColour}>
                                        Discard
                                    </PixelBox>
                                </button>

                                <button type="button" className={styles.btnWrapper} disabled={!meInGame.acted || isAnimating} onClick={() => socket?.emit("playerMove", "endmove")}>
                                    <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                        End Turn
                                    </PixelBox>
                                </button>
                            </div>
                        ) : (
                            <div className={styles.turnBar}>
                                <button type="button" className={styles.btnWrapper} disabled={meInGame.acted || isAnimating} onClick={() => socket?.emit("playerMove", "call")}>
                                    <PixelBox innerClassName={`${styles.btnInner} ${currentBetToCall > 0 ? styles.btnPrimary : ''}`} borderColour={boxBorderColour}>
                                        {currentBetToCall <= 0 ? "Check" : `Call (${currentBetToCall})`}
                                    </PixelBox>
                                </button>

                                <button type="button" className={styles.btnWrapper} disabled={meInGame.acted || isAnimating} onClick={() => socket?.emit("playerMove", `raise ${raiseAmount}`)}>
                                    <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                        Raise
                                    </PixelBox>
                                </button>

                                <NumberSetting
                                    disabled={meInGame.acted}
                                    label=''
                                    range=''
                                    min={config?.minRaise ?? 10}
                                    max={meInGame?.chips ?? 1000}
                                    step={10}
                                    value={raiseAmount}
                                    onChange={(e) => {
                                        if (e === "") {
                                            setRaiseAmount(0);
                                        } else {
                                            setRaiseAmount(typeof e === 'number' ? e : parseInt(e) || 0);
                                        }
                                    }}
                                    styles={styles}
                                    boxBorderColour={boxBorderColour}
                                />

                                <button type="button" className={styles.btnWrapper} disabled={meInGame.acted || isAnimating} onClick={() => socket?.emit("playerMove", "fold")}>
                                    <PixelBox innerClassName={`${styles.btnInner} ${styles.btnDanger}`} borderColour={boxBorderColour}>
                                        Fold
                                    </PixelBox>
                                </button>

                                <button type="button" className={styles.btnWrapper} disabled={!meInGame.acted} onClick={() => socket?.emit("playerMove", "endmove")}>
                                    <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                        End Turn
                                    </PixelBox>
                                </button>
                            </div>
                        )}
                    </>
                )}

                {gameState?.roundName === "room" && (
                    <button type="button" className={styles.btnWrapper} onClick={() => navigate(`/lobby/${roomId}`)}>
                        <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                            Return to Lobby
                        </PixelBox>
                    </button>
                )}

                {(gameState?.roundName === "showdown" || gameState?.roundName === "preround") && (
                    <>
                        {isHost ? (
                            <button type="button" className={styles.btnWrapper} onClick={startNextRound}>
                                <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                    Start Next Hand
                                </PixelBox>
                            </button>
                        ) : (
                            <div className={styles.waitingNotice}>Waiting for Host to start next hand...</div>
                        )}
                    </>
                )}
            </div>
        </div>
    );
}