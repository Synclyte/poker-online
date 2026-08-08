import React, { useState, useEffect, useRef, useMemo } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useSocket } from '../../context/socketcontext';
import { useToast } from '../../context/toastcontext';
import { PixelBox } from '../../components/PixelBox/pixelbox';
import styles from './game.module.css';
import { NumberSetting } from '../../components/NumberSetting/numbersetting';
import { soundManager } from '../../utils/sound';
import { getAssetUrl } from '../../utils/assets';

import {
    CardView,
    GameConfig,
    MoveEvent,
    GameStateData,
    MoveResult,
    PlayerLobbyInfo,
    Particle,
    ChipImageInfo,
    ActiveModifierInfo,
} from './game.types';
import {
    PlayingCard,
    SpecialCard,
    CardFaces,
    getSpecialCardInfo,
    buildSpecialCardInnerHTML,
    parseModifier,
} from '../../components/Card/card';
import { EndGameModal } from '../../components/EndGameModal/endgamemodal';
import { ActiveModifiers } from '../../components/ActiveModifiers/activemodifiers';
import { GameHeader } from '../../components/GameHeader/gameheader';
import { Chat } from '../../components/Chat/chat';
import { CardSlotBar } from '../../components/CardSlotBar/cardslotbar';

const readCssVar = (name: string, fallback: number) => {
    const varName = name.startsWith("--") ? name : `--${name}`;
    const raw = getComputedStyle(document.documentElement).getPropertyValue(varName);
    const parsed = parseFloat(raw);
    return Number.isFinite(parsed) ? parsed : fallback;
};

// animation timings, derived from css
const timing = {
    betChipAdded: readCssVar("--anim-betchip-add-ms", 100),
    flip: readCssVar("--anim-flip-ms", 450),
    deal: readCssVar("--anim-deal-ms", 320),
    dissolve: readCssVar("--anim-dissolve-ms", 400),
    showdown: readCssVar("--anim-showdown-delay-ms", 520),
    specialMove: readCssVar("--anim-special-move-ms", 550),
    specialPause: readCssVar("--anim-special-pause-ms", 300),
    specialRevealPause: readCssVar("--anim-special-reveal-pause-ms", 350),
    check: readCssVar("--anim-check-ms", 400),
    fold: readCssVar("--anim-fold-ms", 300),
};

const computedStyles = getComputedStyle(document.documentElement);
const boxBorderColour = computedStyles.getPropertyValue('--border-colour').trim() || "#dcdcdc";

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

    const [selectedSpecialTarget, setSelectedSpecialTarget] = useState<number | null>(null);
    const [selectedSpecialCardIndex, setSelectedSpecialCardIndex] = useState<number | null>(null);
    const [selectedTargetCardIndex, setSelectedTargetCardIndex] = useState<number | null>(null);
    const [hoveredSpecialIndex, setHoveredSpecialIndex] = useState<number | null>(null);

    const [turnTimeRemaining, setTurnTimeRemaining] = useState<number | null>(null);
    const turnTimerIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
    const turnStartTimeRef = useRef<number | null>(null);
    const activeTurnPlayerIdRef = useRef<number | null>(null);
    const roundNameRef = useRef<string | null>(null);
    const [turnStateTick, setTurnStateTick] = useState<number>(0);

    const specialDeckRef = useRef<HTMLDivElement | null>(null);

    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const activeParticlesRef = useRef<Particle[]>([]);
    const animFrameRef = useRef<number | null>(null);
    const chipImage1 = useRef<HTMLImageElement | null>(null);
    const chipImage10 = useRef<HTMLImageElement | null>(null);
    const chipImage25 = useRef<HTMLImageElement | null>(null);
    const chipImage50 = useRef<HTMLImageElement | null>(null);
    const chipImage250 = useRef<HTMLImageElement | null>(null);
    const chipImage1000 = useRef<HTMLImageElement | null>(null);
    const chipImageDict = useRef<{ [key: number]: ChipImageInfo }>(null);

    const seatRefs = useRef<{ [key: number]: HTMLDivElement | null }>({});

    const eventQueueRef = useRef<{ events: MoveEvent[]; nextState: GameStateData }[]>([]);
    const gameStateRef = useRef<GameStateData | null>(gameState);
    const isProcessingQueue = useRef<boolean>(false);
    const lastPlayedSpecialSlotIndexRef = useRef<number | null>(null);

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
        1: { ref: chipImage1, size: 24 },
        10: { ref: chipImage10, size: 32 },
        25: { ref: chipImage25, size: 40 },
        50: { ref: chipImage50, size: 52 },
        250: { ref: chipImage250, size: 64 },
        1000: { ref: chipImage1000, size: 80 },
    }
    const getChipCounts = (rawAmount: number) => {
        let remainingChips = rawAmount;

        let thousands = Math.max(0, Math.floor((remainingChips - 7500) / Math.max(1000, 1250 - rawAmount / 250)));
        remainingChips -= thousands * 1000;

        let twoHundredFifties = Math.max(0, Math.floor((remainingChips - 1500) / Math.max(250, 320 - rawAmount / 200)));
        remainingChips -= twoHundredFifties * 250;

        let fifties = Math.max(0, Math.floor((remainingChips - 200) / Math.max(50, 70 - rawAmount / 75)));
        remainingChips -= fifties * 50;

        let twentyFives = Math.max(0, Math.floor((remainingChips - 100) / Math.max(25, 40 - rawAmount / 75)));
        remainingChips -= twentyFives * 25;

        let tens = Math.floor(remainingChips / 10);
        remainingChips -= tens * 10;

        let ones = remainingChips;

        return {
            1: ones,
            10: tens,
            25: twentyFives,
            50: fifties,
            250: twoHundredFifties,
            1000: thousands,
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
        loadImg(chipImage1, getAssetUrl('/src/assets/chips/chip1.png'));
        loadImg(chipImage10, getAssetUrl('/src/assets/chips/chip10.png'));
        loadImg(chipImage25, getAssetUrl('/src/assets/chips/chip25.png'));
        loadImg(chipImage50, getAssetUrl('/src/assets/chips/chip50.png'));
        loadImg(chipImage250, getAssetUrl('/src/assets/chips/chip250.png'));
        loadImg(chipImage1000, getAssetUrl('/src/assets/chips/chip1000.png'));
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

    useEffect(() => {
        document.title = "Poker? - In Game";
    }, []);

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

        const totalChips = Object.values(chipCounts).reduce((sum, val) => sum + val, 0);
        const streamDurationMs = delayFormula(1) + 500;
        soundManager.playChipStreamSFX(totalChips, streamDurationMs);

        return delayFormula(1);
    };

    // used for multi-event chains - updates the intermediate values such that animations appear to modify
    // actual values
    const updateIntermediateState = async (
        playerId: number,
        player_chip_delta: number,
        targetRoundBet?: number,
        targetTotalBet?: number,
    ) => {
        const newState = JSON.parse(JSON.stringify(gameStateRef.current)) as GameStateData;
        if (!newState) return;

        const player = newState.players.find(p => p.id === playerId);
        if (!player) return;

        player.chips += player_chip_delta;
        newState.overallSum = Math.max(newState.overallSum - player_chip_delta, 0);

        if (targetRoundBet !== undefined) {
            player.roundBet = targetRoundBet;
        } else if (player_chip_delta < 0) {
            player.roundBet = (player.roundBet || 0) - player_chip_delta;
        }

        if (targetTotalBet !== undefined) {
            player.totalBet = targetTotalBet;
        } else if (player_chip_delta < 0) {
            player.totalBet = (player.totalBet || 0) - player_chip_delta;
        }

        newState.highestBet = Math.max(newState.highestBet || 0, player.roundBet || 0);

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
                const isNewHand = previousState && (previousState.roundName === "preround" || previousState.roundName === "room" || previousState.roundName === "showdown");
                const enteredPreflop = isNewHand || (nextState.roundName === "preflop" && previousState?.roundName !== "preflop");
                const enteredShowdown = nextState.roundName === "showdown" && previousState?.roundName !== "showdown";

                const applyState = (state: GameStateData) => {
                    gameStateRef.current = state;
                    setGameState(state);
                };

                const cloneCurrentState = (): GameStateData =>
                    gameStateRef.current ? (JSON.parse(JSON.stringify(gameStateRef.current)) as GameStateData) : nextState;

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
                        highestBet: 0,
                        deckCards: (nextState.deckCards ?? 52) + deckCardsDealtInBatch,
                        players: gameStateRef.current.players.map(p => {
                            const nextP = nextState.players.find(np => np.id === p.id);
                            return {
                                ...p,
                                holeCards: [],
                                handType: '',
                                totalBet: 0,
                                roundBet: 0,
                                folded: nextP ? nextP.folded : false,
                            };
                        }),
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
                        const previousPlayer = previousState.players.find(p => p.id === player.id);
                        const blindAmount = previousPlayer
                            ? Math.max(0, previousPlayer.chips - player.chips)
                            : Math.max(player.roundBet, player.totalBet);

                        if (blindAmount <= 0) continue;

                        const delay = spawnChipStream(`player-${player.id}`, "pot", blindAmount);

                        await wait(delay + timing.betChipAdded);
                        await updateIntermediateState(player.id, -blindAmount, player.roundBet, player.totalBet);
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
                    const feltStyle = window.getComputedStyle(feltElem);
                    const borderLeft = parseFloat(feltStyle.borderLeftWidth) || 0;
                    const borderTop = parseFloat(feltStyle.borderTopWidth) || 0;

                    const feltOriginX = feltRect.left + borderLeft;
                    const feltOriginY = feltRect.top + borderTop;

                    const currentMyId = myIdRef.current ?? myId;
                    const isSelf = actorId === currentMyId;

                    let slotElem: HTMLElement | null = null;
                    let playedSlotIndex = isSelf ? (lastPlayedSpecialSlotIndexRef.current ?? selectedSpecialCardIndex) : null;
                    if (isSelf) lastPlayedSpecialSlotIndexRef.current = null;

                    if (isSelf && (playedSlotIndex === null || playedSlotIndex === undefined || playedSlotIndex < 0)) {
                        const currentPlayer = gameStateRef.current?.players.find(p => p.id === actorId);
                        if (currentPlayer) {
                            const idx = currentPlayer.specialCards.findIndex(
                                c => c && c.visibility === "visible" && c.value && c.value.toLowerCase() === cardValue.toLowerCase()
                            );
                            if (idx !== -1) playedSlotIndex = idx;
                        }
                    }

                    if (isSelf) {
                        const targetSlotAttr = `special-${playedSlotIndex ?? 0}`;
                        slotElem = document.querySelector(`[data-slot-attr="${targetSlotAttr}"]`) ?? document.querySelector(`[data-slot-attr^="special-"]`);
                    }

                    const animCard = document.createElement("div");
                    animCard.className = styles.animatedSpecialCardOverlay;
                    animCard.innerHTML = buildSpecialCardInnerHTML(cardValue, isSelf);
                    feltElem.appendChild(animCard);

                    // centering calculations
                    const animCardRect = animCard.getBoundingClientRect();
                    const cardWidth = animCardRect.width;
                    const cardHeight = animCardRect.height;

                    const centerBoardElem = document.querySelector(`.${styles.centerBoard}`) ?? feltElem;
                    const centerRect = centerBoardElem.getBoundingClientRect();

                    const centerX = (centerRect.left + centerRect.width / 2) - feltOriginX;
                    const centerY = (centerRect.top + centerRect.height / 2) - feltOriginY;

                    const targetX = centerX - cardWidth / 2;
                    const targetY = centerY - cardHeight / 2;

                    let startX = targetX;
                    let startY = targetY;

                    if (isSelf && slotElem) {
                        const slotRect = slotElem.getBoundingClientRect();
                        const slotCenterX = (slotRect.left + slotRect.width / 2) - feltOriginX;
                        const slotCenterY = (slotRect.top + slotRect.height / 2) - feltOriginY;

                        startX = slotCenterX - cardWidth / 2;
                        startY = slotCenterY - cardHeight / 2;
                        slotElem.style.opacity = "0";
                    } else if (!isSelf) {
                        const seatElem = document.querySelector(`[data-seat-id="${actorId}"]`);
                        if (seatElem) {
                            const seatRect = seatElem.getBoundingClientRect();
                            const seatCenterX = (seatRect.left + seatRect.width / 2) - feltOriginX;
                            const seatCenterY = (seatRect.top + seatRect.height / 2) - feltOriginY;

                            startX = seatCenterX - cardWidth / 2;
                            startY = seatCenterY - cardHeight / 2;
                        }
                    }

                    animCard.style.setProperty("--start-x", `${startX}px`);
                    animCard.style.setProperty("--start-y", `${startY}px`);
                    animCard.style.setProperty("--end-x", `${targetX}px`);
                    animCard.style.setProperty("--end-y", `${targetY}px`);

                    // controls special card use animations
                    try {
                        if (isSelf) {
                            soundManager.playSound("special_dissolve");
                            animCard.classList.add(styles.animMoveSelf);
                            await wait(timing.specialMove);
                            await wait(timing.specialPause);

                            animCard.classList.remove(styles.animMoveSelf);
                            animCard.classList.add(styles.animDissolveFaceUp);
                            await wait(timing.dissolve);
                        } else {
                            soundManager.playSound("special_dissolve");
                            animCard.classList.add(styles.animMoveOther);
                            await wait(timing.specialMove);

                            animCard.classList.remove(styles.animMoveOther);
                            animCard.classList.add(styles.animFlipReveal);
                            await wait(timing.flip);
                            await wait(timing.specialRevealPause);

                            animCard.classList.remove(styles.animFlipReveal);
                            animCard.classList.add(styles.animDissolveFlipped);
                            await wait(timing.dissolve);
                        }
                    } finally {
                        if (gameStateRef.current) {
                            const newState: GameStateData = JSON.parse(JSON.stringify(gameStateRef.current));
                            const player = newState.players.find(p => p.id === actorId);
                            if (player) {
                                const idx = player.specialCards.findIndex(
                                    c => c && c.visibility === "visible" && c.value.toLowerCase() === cardValue.toLowerCase()
                                );
                                if (idx !== -1) {
                                    player.specialCards.splice(idx, 1);
                                    gameStateRef.current = newState;
                                    setGameState(newState);
                                }
                            }
                        }
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

                            if (betDelta <= 0) {
                                soundManager.playSound("call");
                                const seatElem = seatRefs.current[event.actorId];
                                if (seatElem) {
                                    seatElem.classList.remove(styles.animatingCheck);
                                    void seatElem.offsetWidth;
                                    seatElem.classList.add(styles.animatingCheck);
                                    await wait(timing.check);
                                    seatElem.classList.remove(styles.animatingCheck);
                                }
                                return;
                            }

                            if (action.type === "call") {
                                soundManager.playSound("call");
                            }

                            const delay = spawnChipStream(`player-${event.actorId}`, "pot", betDelta);
                            await wait(delay + timing.betChipAdded);
                            await updateIntermediateState(event.actorId, -betDelta, nextPlayer.roundBet, nextPlayer.totalBet);

                            return;
                        }

                        case "fold":
                        case "timeout": {
                            if (event.actorId !== null) {
                                const viewState = cloneCurrentState();
                                const player = viewState.players.find(p => p.id === event.actorId);
                                if (player) {
                                    player.folded = true;
                                    applyState(structuredClone(viewState));
                                    soundManager.playSound("fold");
                                    await wait(timing.fold ?? 300);
                                }
                            }
                            return;
                        }

                        case "playSpecial": {
                            const actorId = event.actorId;
                            const cardValue = action.card;
                            if (actorId !== null && cardValue) {
                                await playSpecialCardUseAnimation(actorId, cardValue);
                            }
                            return;
                        }

                        case "discardSpecial": {
                            const actorId = event.actorId;
                            const cardIndex = action.cardIndex;
                            if (actorId !== null && cardIndex !== undefined) {
                                const currentMyId = myIdRef.current ?? myId;
                                const isSelf = actorId === currentMyId;
                                const refKey = isSelf ? `special-${cardIndex}` : `special-seat-${actorId}`;
                                const selectorFallback = isSelf
                                    ? `[data-slot-attr="special-${cardIndex}"] .${styles.cardInner}`
                                    : `[data-seat-id="${actorId}"] .${styles.cardInner}`;

                                const target = await resolveTargetElement(refKey, selectorFallback);
                                if (target) {
                                    soundManager.playSound("dissolve");
                                    await triggerInPlaceAnimation(target, target, styles.animatingDissolve, timing.dissolve);
                                }
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
                        soundManager.playSound("card_draw");
                        // move while face down to the target
                        await triggerInPlaceAnimation(target, source, styles.animatingDeal, timing.deal);

                        // then flip the card (if required)
                        if (needsFlip) {
                            const flipPromise = triggerInPlaceAnimation(target, target, styles.animatingFlip, timing.flip);
                            await wait(timing.flip / 2);
                            activeCards[cardIndex] = nextCard;
                            applyState(structuredClone(viewState));
                            await flipPromise;
                        } else {
                            activeCards[cardIndex] = nextCard;
                            applyState(structuredClone(viewState));
                            await nextFrame();
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
                                    soundManager.playSound("card_draw");
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

                            const refKey = `hole-${event.actorId}-${index}`;
                            const target = cardInnerRefs.current[refKey] ??
                                (document.querySelector(`[data-hole-slot="${event.actorId}:${index}"] .${styles.cardInner}`) as HTMLDivElement | null);

                            if (target) {
                                soundManager.playSound("dissolve");
                                await triggerInPlaceAnimation(target, target, styles.animatingDissolve, timing.dissolve);
                            }

                            player.holeCards.splice(index, 1);
                            applyState(viewState);
                            await nextFrame();
                            return;
                        }

                        case "removeCommunity": {
                            const viewState = cloneCurrentState();
                            const index = event.action.index;

                            if (index < 0 || index >= viewState.communityCards.length) return;

                            const target = cardInnerRefs.current[`community-${index}`] ??
                                (document.querySelector(`[data-community-slot="${index}"] .${styles.cardInner}`) as HTMLDivElement | null);

                            if (target) {
                                soundManager.playSound("dissolve");
                                await triggerInPlaceAnimation(target, target, styles.animatingDissolve, timing.dissolve);
                            }

                            viewState.communityCards.splice(index, 1);
                            applyState(viewState);
                            await nextFrame();
                            return;
                        }

                        case "replaceHole": {
                            if (event.actorId === null) return;
                            const viewState = cloneCurrentState();
                            const player = viewState.players.find(c => c.id === event.actorId);
                            const nextPlayer = nextState.players.find(c => c.id === event.actorId);
                            const index = event.action.index;

                            if (!player || !nextPlayer || index < 0 || index >= nextPlayer.holeCards.length) return;

                            const nextCard = nextPlayer.holeCards[index];
                            if (!nextCard) return;

                            const refKey = `hole-${event.actorId}-${index}`;
                            const target = cardInnerRefs.current[refKey] ??
                                (document.querySelector(`[data-hole-slot="${event.actorId}:${index}"] .${styles.cardInner}`) as HTMLDivElement | null);

                            if (target) {
                                soundManager.playSound("dissolve");
                                await triggerInPlaceAnimation(target, target, styles.animatingDissolve, timing.dissolve);
                            }

                            player.holeCards[index] = nextCard;
                            applyState(structuredClone(viewState));
                            await nextFrame();

                            if (target && nextCard.visibility === "visible") {
                                soundManager.playSound("card_draw");
                                await triggerInPlaceAnimation(target, target, styles.animatingFlip, timing.flip);
                            }
                            return;
                        }

                        case "replaceCommunity": {
                            const viewState = cloneCurrentState();
                            const index = event.action.index;
                            if (index < 0 || index >= nextState.communityCards.length) return;

                            const nextCard = nextState.communityCards[index];
                            if (!nextCard) return;

                            const target = cardInnerRefs.current[`community-${index}`] ??
                                (document.querySelector(`[data-community-slot="${index}"] .${styles.cardInner}`) as HTMLDivElement | null);

                            if (target) {
                                soundManager.playSound("dissolve");
                                await triggerInPlaceAnimation(target, target, styles.animatingDissolve, timing.dissolve);
                            }

                            viewState.communityCards[index] = nextCard;
                            applyState(structuredClone(viewState));
                            await nextFrame();

                            if (target && nextCard.visibility === "visible") {
                                soundManager.playSound("card_draw");
                                await triggerInPlaceAnimation(target, target, styles.animatingFlip, timing.flip);
                            }
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

                            const target = cardInnerRefs.current[`hole-${targetId}-${cardIndex}`] ??
                                (holeSlotRefs.current[`${targetId}:${cardIndex}`]?.querySelector(`.${styles.cardInner}`) as HTMLDivElement | null) ?? null;

                            if (target) {
                                const flipPromise = triggerInPlaceAnimation(target, target, styles.animatingFlip, timing.flip);
                                await wait(timing.flip / 2);
                                player.holeCards[cardIndex] = revealedCard;
                                applyState(structuredClone(viewState));
                                await flipPromise;
                            } else {
                                player.holeCards[cardIndex] = revealedCard;
                                applyState(structuredClone(viewState));
                                await nextFrame();
                            }
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
                        const blindAmount = Math.max(0, player.chips - (nextP?.chips ?? player.chips));
                        const initialBet = enteredPreflop ? blindAmount : player.totalBet;
                        return [player.id, initialBet];
                    }),
                );

                if (deckCardsDealtInBatch > 0 && gameStateRef.current) {
                    gameStateRef.current.deckCards = (nextState.deckCards ?? 52) + deckCardsDealtInBatch;
                    setGameState({ ...gameStateRef.current });
                }

                for (const event of events) {
                    if (event.action.type === "action") {
                        await animateAction(event, trackedBets);
                    } else {
                        await animateStructuralEvent(event, nextState);
                    }
                }

                if (enteredShowdown) {
                    // showdown has to be done in a complicated manner, as the reveal order and animations need to completely
                    // ignore all other logic surrounding animations through the rest of the file
                    // this is done by generating multiple consecutive pseudo-states to force expected animations

                    // first, card ranks/suits are given values (while not updating anything else), so cards can flip correctly
                    const currentState = gameStateRef.current ?? previousState;
                    const preFlipState: GameStateData = {
                        ...nextState,
                        overallSum: currentState.overallSum,
                        pots: currentState.pots,
                        players: nextState.players.map(nextPlayer => {
                            const currPlayer = currentState.players.find(p => p.id === nextPlayer.id);

                            return {
                                ...nextPlayer,
                                chips: currPlayer?.chips ?? nextPlayer.chips,
                                holeCards: nextPlayer.holeCards.map((nextCard, cardIndex) => {
                                    const prevCard = currPlayer?.holeCards[cardIndex];
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

            if (response?.gameState) {
                const nextTurnPlayer = response.gameState.players.find(p => p.isTurn);
                const nextTurnId = nextTurnPlayer?.id ?? null;
                const nextRound = response.gameState.roundName;

                if (nextTurnId !== activeTurnPlayerIdRef.current || nextRound !== roundNameRef.current || !turnStartTimeRef.current) {
                    activeTurnPlayerIdRef.current = nextTurnId;
                    roundNameRef.current = nextRound;
                    turnStartTimeRef.current = Date.now();
                    setTurnStateTick(t => t + 1);
                }
            }

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

        lastPlayedSpecialSlotIndexRef.current = selectedSpecialCardIndex;

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
    const currentBetToCall = (gameState?.highestBet || 0) - (meInGame?.roundBet || 0);

    const anteMult = gameState?.modifierVars?.anteMult ?? 1;
    const effectiveMinRaise = Math.round((config?.minRaise ?? 10) * anteMult);

    useEffect(() => {
        if (effectiveMinRaise > 0 && raiseAmount < effectiveMinRaise) {
            setRaiseAmount(effectiveMinRaise);
        }
    }, [effectiveMinRaise]);

    const activeModifiers: ActiveModifierInfo[] = useMemo(() => {
        return (gameState?.modifiers ?? []).map((m: any, idx: number) => parseModifier(m, idx));
    }, [gameState?.modifiers]);

    const [displayedModifiers, setDisplayedModifiers] = useState<(ActiveModifierInfo & { isRemoving?: boolean })[]>([]);

    useEffect(() => {
        setDisplayedModifiers(prev => {
            const nextIds = new Set(activeModifiers.map(m => m.id));
            const hasRemovals = prev.some(p => !p.isRemoving && !nextIds.has(p.id));

            if (!hasRemovals) {
                const activeMap = new Map(activeModifiers.map(m => [m.id, m]));
                const merged: (ActiveModifierInfo & { isRemoving?: boolean })[] = [];
                for (const p of prev) {
                    if (p.isRemoving) {
                        merged.push(p);
                    } else if (activeMap.has(p.id)) {
                        merged.push(activeMap.get(p.id)!);
                        activeMap.delete(p.id);
                    }
                }
                for (const newMod of activeMap.values()) {
                    merged.push(newMod);
                }
                return merged;
            }

            setTimeout(() => {
                setDisplayedModifiers(current => current.filter(item => nextIds.has(item.id)));
            }, 400);

            const activeMap = new Map(activeModifiers.map(m => [m.id, m]));
            const merged: (ActiveModifierInfo & { isRemoving?: boolean })[] = [];

            for (const p of prev) {
                if (activeMap.has(p.id)) {
                    merged.push(activeMap.get(p.id)!);
                    activeMap.delete(p.id);
                } else {
                    merged.push({ ...p, isRemoving: true });
                }
            }
            for (const newMod of activeMap.values()) {
                merged.push(newMod);
            }
            return merged;
        });
    }, [activeModifiers]);

    useEffect(() => {
        if (turnTimerIntervalRef.current) {
            clearInterval(turnTimerIntervalRef.current);
            turnTimerIntervalRef.current = null;
        }

        const isRoundActive = roundNameRef.current && roundNameRef.current !== "room" && roundNameRef.current !== "preround" && roundNameRef.current !== "showdown";
        const turnPlayerId = activeTurnPlayerIdRef.current;

        if (!isRoundActive || turnPlayerId === null || turnPlayerId === undefined || !config?.turnTimeout) {
            setTurnTimeRemaining(null);
            return;
        }

        const maxSeconds = config.turnTimeout;

        const updateTimer = () => {
            if (!turnStartTimeRef.current) {
                setTurnTimeRemaining(maxSeconds);
                return;
            }
            const elapsedSeconds = Math.floor((Date.now() - turnStartTimeRef.current) / 1000);
            const remaining = Math.max(0, maxSeconds - elapsedSeconds);
            setTurnTimeRemaining(remaining);

            if (remaining <= 0 && turnTimerIntervalRef.current) {
                clearInterval(turnTimerIntervalRef.current);
                turnTimerIntervalRef.current = null;
            }
        };

        updateTimer();
        turnTimerIntervalRef.current = setInterval(updateTimer, 200);

        return () => {
            if (turnTimerIntervalRef.current) {
                clearInterval(turnTimerIntervalRef.current);
                turnTimerIntervalRef.current = null;
            }
        };
    }, [turnStateTick, config?.turnTimeout]);

    useEffect(() => {
        if (!isMyTurn || gameState?.roundName === "showdown" || gameState?.roundName === "preround" || gameState?.roundName === "room") {
            cancelSelectedSpecialCard();
        }
    }, [gameState?.roundName, isMyTurn]);

    return (
        <div className={styles.gameContainer}>
            <EndGameModal
                gameState={gameState}
                myId={myId}
                isHost={isHost}
                lobbyPlayers={lobbyPlayers}
                boxBorderColour={boxBorderColour}
                startNextRound={startNextRound}
                backToRoom={() => navigate(`/lobby/${roomId}`)}
            />

            <GameHeader
                roomId={roomId}
                gameState={gameState}
                config={config}
                turnTimeRemaining={turnTimeRemaining}
                boxBorderColour={boxBorderColour}
                leaveGame={leaveGame}
            />

            <ActiveModifiers displayedModifiers={displayedModifiers} />

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
                                const specialDeckLayerCount = Math.ceil(config?.specialCardLimit ?? 0);
                                if (specialDeckLayerCount <= 0) return null;
                                const hiddenSpecialCard: CardView = { visibility: "hidden" };

                                return Array.from({ length: specialDeckLayerCount }).map((_, i) => (
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

                        <CardSlotBar
                            cards={gameState?.communityCards ?? []}
                            initialSlotLimit={5}
                            containerClassName={styles.communityCards}
                            renderCard={(card, idx) => {
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
                            }}
                        />
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

                                        <CardSlotBar
                                            cards={p.holeCards}
                                            initialSlotLimit={2}
                                            containerClassName={styles.holeCards}
                                            renderCard={(card, cardIndex) => {
                                                const canTargetHoleCard = selectedSpecialInfo && (
                                                    (isMe && selectedSpecialInfo.target === "selfCard" && card !== null) ||
                                                    (!isMe && selectedSpecialInfo.target === "otherCard" && selectedSpecialTarget === p.id)
                                                );
                                                const isSelectedHoleCard = canTargetHoleCard && selectedTargetCardIndex === cardIndex;

                                                return (
                                                    <div
                                                        data-hole-slot={`${p.id}:${cardIndex}`}
                                                        ref={element => { holeSlotRefs.current[`${p.id}:${cardIndex}`] = element; }}
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
                                            }}
                                        />

                                        {config && (config.specialCardLimit > 0 || p.specialCards.length > 0) && (
                                            <div className={styles.seatSpecialCards}>
                                                <span className={styles.specialCardCount}><span style={{ color: "var(--special-colour)" }}>Special</span> x{p.specialCards.length}</span>
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

            {gameState?.roundName !== "room" && meInGame && (config?.specialCardLimit ?? 0) > 0 && (
                <div className={styles.specialCardBar}>
                    {(() => {
                        const limit = config?.specialCardLimit ?? 0;

                        return (
                            <PixelBox
                                borderColour="transparent"
                                backgroundColour="rgba(26, 26, 36, 0.75)"
                                className={styles.specialPixelBoxBar}
                                innerClassName={styles.specialCardBarInner}
                                unclipped
                            >
                                <CardSlotBar
                                    cards={meInGame.specialCards}
                                    initialSlotLimit={limit}
                                    containerClassName={styles.specialCardHand}
                                    containerStyle={{
                                        width: `calc(${limit} * var(--card-slot-width) + (${limit} - 1) * var(--card-gap))`
                                    }}
                                    renderCard={(card, index) => {
                                        const isSelected = selectedSpecialCardIndex === index;

                                        return (
                                            <div
                                                key={`special-slot-wrapper-${index}`}
                                                className={styles.specialSlotWrapper}
                                                onMouseEnter={() => card ? setHoveredSpecialIndex(index) : undefined}
                                                onMouseLeave={() => setHoveredSpecialIndex(null)}
                                            >
                                                {card && (card as any).visibility === "visible" && (card as any).value && (isSelected || hoveredSpecialIndex === index) && (() => {
                                                    const cardInfo = getSpecialCardInfo(
                                                        (card as any).value,
                                                        config?.blindSize,
                                                        gameState?.modifierVars?.anteMult,
                                                        gameState?.modifierVars?.gambleSuccessChance
                                                    );
                                                    return (
                                                        <div
                                                            className={[
                                                                styles.specialSelectionText,
                                                                isSelected ? styles.active : styles.hovered,
                                                            ].join(" ")}
                                                        >
                                                            <strong>{cardInfo.label}</strong>
                                                            <span className={styles.specialSelectionDesc}>{cardInfo.description}</span>

                                                            {isSelected && (
                                                                <>
                                                                    {cardInfo.target === "other" &&
                                                                        selectedSpecialTarget === null && <span className={styles.specialSelectionInstruction}>Select an opponent</span>}

                                                                    {cardInfo.target === "selfCard" &&
                                                                        cardInfo.requiresTargetIndex &&
                                                                        selectedTargetCardIndex === null && <span className={styles.specialSelectionInstruction}>Select one of your own hole cards</span>}

                                                                    {cardInfo.target === "otherCard" &&
                                                                        cardInfo.requiresTargetIndex &&
                                                                        selectedSpecialTarget !== null &&
                                                                        selectedTargetCardIndex === null && <span className={styles.specialSelectionInstruction}>Select an opponent hole card</span>}

                                                                    {cardInfo.target === "communityCard" &&
                                                                        cardInfo.requiresTargetIndex &&
                                                                        selectedTargetCardIndex === null && <span className={styles.specialSelectionInstruction}>Select a community card</span>}
                                                                </>
                                                            )}
                                                        </div>
                                                    );
                                                })()}

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
                                                    onClick={(card && isMyTurn && !meInGame.acted) ? () => {
                                                        setSelectedSpecialCardIndex(
                                                            selectedSpecialCardIndex === index ? null : index,
                                                        );
                                                        setSelectedSpecialTarget(null);
                                                        setSelectedTargetCardIndex(null);
                                                    } : undefined}
                                                />
                                            </div>
                                        );
                                    }}
                                />
                            </PixelBox>
                        );
                    })()}
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
                                    min={effectiveMinRaise}
                                    max={meInGame?.chips ?? 1000}
                                    step={effectiveMinRaise}
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
            <Chat position="top-left" roomId={roomId} myId={myId} />
        </div>
    );
}