import React, { useState, useEffect, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useSocket } from '../../context/SocketContext';
import { useToast } from '../../context/ToastContext';
import { PixelBox } from '../../components/PixelBox/pixelbox';
import styles from './Game.module.css';
import { NumberSetting } from '../../components/NumberSetting/numbersetting';

interface MoveEvent {
    player_id: number;
    action: string;
}

interface PlayerState {
    id: number;
    chips: number;
    total_bet: number;
    round_bet: number;
    folded: boolean;
    acted: boolean;
    is_turn: boolean;
    is_dealer: boolean;
    hole_cards: string[];
    special_cards: string[];
    hand_type: string;
}

interface GameStateData {
    round_name: string;
    community_cards: string[];
    pots: number[];
    round_bet_sum: number;
    overall_sum: number;
    highest_bet: number;
    deck_cards: number;
    winning_hand_type: string;
    players: PlayerState[];
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

// animation timings
const betChipAddedTime = 150;
const flipTime = 420;
const dealTime = 400;
const showdownFlipDelay = flipTime + 250;

const computedStyles = getComputedStyle(document.documentElement);
const boxBorderColour = computedStyles.getPropertyValue('--border-colour').trim() || "#dcdcdc";

/** Takes a card string in format `rank:suit` (i.e. 6:d) and converts 
 * the card into a component representation
*/
function parseCardString(cardStr: string) {
    if (!cardStr || cardStr === "HIDDEN" || cardStr === "Unmarked") {
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

// Playing card component
// Contains a deal animation (flying from card stack) and flip animation
interface PlayingCardProps {
    cardStr: string | null;
    deckRef: React.RefObject<HTMLDivElement | null>;
}

/** Playing card component representation.
 *  Handles movement & flip animations, alongside rendering the card itself
 */
const PlayingCard: React.FC<PlayingCardProps> = ({ cardStr, deckRef }) => {
    const slotRef = useRef<HTMLDivElement | null>(null);
    const [dealOffset, setDealOffset] = useState<{ x: string; y: string } | null>(null);
    const [isFlipped, setIsFlipped] = useState<boolean>(false);
    
    const [isDealing, setIsDealing] = useState<boolean>(true);
    const isInitiallyDealt = useRef<boolean>(false);

    const isJoker = cardStr?.startsWith("*") || false;
    const parsed = parseCardString(cardStr || "");
    const shouldBeRevealed = Boolean(cardStr && !parsed.isHidden);

    useEffect(() => {
        if (!cardStr) return;

        if (!isInitiallyDealt.current) {
            isInitiallyDealt.current = true;

            if (slotRef.current && deckRef.current) {
                const slotRect = slotRef.current.getBoundingClientRect();
                const deckRect = deckRef.current.getBoundingClientRect();

                const deltaX = `${(deckRect.left + deckRect.width / 2) - (slotRect.left + slotRect.width / 2)}px`;
                const deltaY = `${(deckRect.top + deckRect.height / 2) - (slotRect.top + slotRect.height / 2)}px`;

                setDealOffset({ x: deltaX, y: deltaY });
            } else {
                setDealOffset({ x: "0px", y: "-150px" });
            }

            const dealTimer = setTimeout(() => {
                setIsDealing(false);
            }, dealTime);

            return () => clearTimeout(dealTimer);
        }
    }, [cardStr, deckRef]);

    useEffect(() => {
        if (shouldBeRevealed) {
            const delay = isDealing ? flipTime : 0;
            const timer = setTimeout(() => {
                setIsFlipped(true);
            }, delay);
            return () => clearTimeout(timer);
        } else {
            setIsFlipped(false);
        }
    }, [shouldBeRevealed, isDealing]);

    if (!cardStr) {
        return <div className={styles.cardSlot} />;
    }

    return (
        <div ref={slotRef} className={styles.cardSlot}>
            {cardStr && dealOffset && (
                <div 
                    className={dealOffset ? styles.dealtCard : ''}
                    style={{
                        '--deal-x': dealOffset?.x || '0px',
                        '--deal-y': dealOffset?.y || '0px',
                        width: '100%',
                        height: '100%'
                    } as React.CSSProperties}
                >
                    <div className={`${styles.cardInner} ${isFlipped ? styles.cardFlipped : ''}`}>
                        <div className={styles.cardBackFace} />
                        {!isJoker && (
                            <div className={styles.cardFront} style={{ color: parsed.color }}>
                                <span>{parsed.rank}</span>
                                <img src={`/src/assets/${parsed.suitSymbol}.png`} alt={parsed.suitSymbol} />
                            </div>
                        )}
                        {isJoker && (
                            <div className={`${styles.cardFront} ${styles.joker}`} style={{ color: '#000000' }}>
                                <span style={{ transform: 'rotate(90deg)', left: '-10px', top: '15px', position: 'absolute'}}>Joker</span>
                                <span style={{ transform: 'rotate(270deg)', right: '-10px', bottom: '15px', position: 'absolute'}}>Joker</span>
                                <img src={`/src/assets/heartsmall.png`} />
                                <img src={`/src/assets/spadesmall.png`} />
                                <img src={`/src/assets/diamondsmall.png`} />
                                <img src={`/src/assets/clubsmall.png`} />
                            </div>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
};

export function Game() {
    const { roomId } = useParams<{ roomId: string }>();
    const { socket, isConnected } = useSocket();
    const { showToast } = useToast();
    const navigate = useNavigate();

    const [myId, setMyId] = useState<number | null>(null);
    const [gameState, setGameState] = useState<GameStateData | null>(null);
    const [lobbyPlayers, setLobbyPlayers] = useState<PlayerLobbyInfo[]>([]);
    const [raiseAmount, setRaiseAmount] = useState<number>(10);

    const [activeMove, setActiveMove] = useState<MoveEvent | null>(null);

    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const activeParticlesRef = useRef<Particle[]>([]);
    const animFrameRef = useRef<number | null>(null);
    const chipImageOne = useRef<HTMLImageElement | null>(null);
    const chipImageTen = useRef<HTMLImageElement | null>(null);
    const chipImageFifty = useRef<HTMLImageElement | null>(null);
    const chipImageTwoHundred = useRef<HTMLImageElement | null>(null);
    const chipImageOneThousand = useRef<HTMLImageElement | null>(null);
    const chipImageDict = useRef<{ [key: number]: ChipImageInfo}>(null);

    const seatRefs = useRef<{ [key: number]: HTMLDivElement | null }>({});

    const eventQueueRef = useRef<{ events: MoveEvent[]; nextState: GameStateData }[]>([]);
    const gameStateRef = useRef<GameStateData | null>(gameState);
    const isProcessingQueue = useRef<boolean>(false);

    const deckRef = useRef<HTMLDivElement | null>(null);
    const potRef = useRef<HTMLDivElement | null>(null);

    const meInGame = gameState?.players.find(p => p.id === myId);
    const meInLobby = lobbyPlayers.find(p => p.id === myId);
    const isHost = meInLobby?.isHost || false;

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
        loadImg(chipImageOne, '/src/assets/chip1.png');
        loadImg(chipImageTen, '/src/assets/chip10.png');
        loadImg(chipImageFifty, '/src/assets/chip50.png');
        loadImg(chipImageTwoHundred, '/src/assets/chip200.png');
        loadImg(chipImageOneThousand, '/src/assets/chip1000.png');
        chipImageDict.current = imageDict;
    }, []);

    useEffect(() => {
        gameStateRef.current = gameState;
    }, [gameState]);

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

            // draw chip particle
            const imgDict = chipImageDict.current;
            if (!imgDict) return;

            // ensures higher denomination chips render over lower denominations
            activeParticlesRef.current.sort((a, b) => b.type - a.type);

            const particles = activeParticlesRef.current;
            for (let i = particles.length - 1; i >= 0; i--) {
                const p = particles[i];
                const elapsed = timestamp - p.startTime;
                if (elapsed < 0) continue;

                const progress = Math.min(1, elapsed / p.duration);
                // ease out quadratic function
                const ease = 1 - (1 - progress) * (1 - progress);

                // calculates positioning using easing
                const currentX = p.startX + (p.endX - p.startX) * ease;
                const currentY = p.startY + (p.endY - p.startY) * ease;
                const chipImg = imgDict[p.type];

                // progress-based chip scaling - chips initially grow, maintain size, and shrink as they
                // are absorbed. absorbing given more time due to ease function
                let currentScale = 1;
                if (progress < 0.2) {
                    currentScale = (1 / 0.2) * progress;
                } else if (progress > 0.7) {
                    currentScale = 1 - (1 / 0.3) * (progress - 0.7);
                }
                const chipSize = chipImg.size * currentScale;

                ctx.save();
                
                if (chipImg && chipImg.ref.current) {
                    ctx.drawImage(
                        chipImg.ref.current,
                        currentX - chipSize / 2,
                        currentY - chipSize / 2,
                        chipSize,
                        chipSize
                    )
                }

                if (progress >= 1) {
                    particles.splice(i, 1);
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

        const startX = (startRect.left + startRect.width / 2) - canvasRect.left;
        const startY = (startRect.top + startRect.height / 2) - canvasRect.top;

        const endX = (endRect.left + endRect.width / 2) - canvasRect.left;
        const endY = (endRect.top + endRect.height / 2) - canvasRect.top;

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
    const updateIntermediateState = async (player_id: number, player_chip_delta: number) => {
        const newState = JSON.parse(JSON.stringify(gameStateRef.current)) as GameStateData;
        if (!newState) return;

        const player = newState.players.find(p => p.id === player_id);
        if (!player) return;

        player.chips += player_chip_delta;
        newState.overall_sum = Math.max(newState.overall_sum - player_chip_delta, 0);

        setGameState(newState);
        gameStateRef.current = newState;
    }

    // main event animation handler function
    const processEventQueue = async () => {
        if (isProcessingQueue.current || eventQueueRef.current.length === 0) return;
        isProcessingQueue.current = true;

        while (eventQueueRef.current.length > 0) {
            const { events, nextState } = eventQueueRef.current.shift()!;
            const previousState = gameStateRef.current;
            const isShowdown = nextState.round_name === 'showdown' && previousState?.round_name !== 'showdown';

            // produces animations for initial blind posting
            const handleInitialBlinds = async () => {
                if (!(nextState.round_name === 'preflop' && (!previousState || previousState.round_name !== 'preflop'))) {
                    return;
                }

                for (const p of nextState.players) {
                    const blindAmount = p.total_bet || p.round_bet || 0;
                    if (blindAmount > 0) {
                        const delay = spawnChipStream(`player-${p.id}`, 'pot', blindAmount);
                        await new Promise((resolve) => setTimeout(resolve, delay + betChipAddedTime));
                        updateIntermediateState(p.id, -blindAmount);
                    }
                }
            }

            // produces animations for pot being distributed during showdown
            const handlePotDistributions = async () => {
                if (!isShowdown || !previousState) {
                    return;
                }

                for (const np of nextState.players) {
                    const prevPlayer = previousState.players.find(pp => pp.id === np.id);
                    if (prevPlayer) {
                        const gain = np.chips - prevPlayer.chips;
                        if (gain > 0) {
                            const delay = spawnChipStream('pot', `player-${np.id}`, gain);
                            await new Promise((resolve) => setTimeout(resolve, delay + betChipAddedTime));
                        }
                    }
                };
            };

            // first checks if blinds should be handled - this should always precede bot bet handling
            await handleInitialBlinds();

            // if there are normal events, handle them immediately after
            if (events.length > 0) {
                const trackingBets: { [playerId: number]: number } = {};
                previousState?.players?.forEach(p => {
                    trackingBets[p.id] = p.total_bet || 0;
                });

                // sequentially handles raise/call events, updating intermediate states
                for (const event of events) {
                    setActiveMove(event);
                    const actUpper = typeof event.action === "string" ? event.action.toUpperCase() : "";

                    const nextPlayer = nextState.players?.find(p => p.id === event.player_id);
                    if (nextPlayer) {
                        const startBet = trackingBets[event.player_id] ?? (nextPlayer.total_bet || 0);
                        const targetBet = nextPlayer.total_bet || 0;
                        const betDelta = Math.max(0, targetBet - startBet);
                        const isBetAction = actUpper.includes("RAISE") || actUpper.includes("CALL") || actUpper.includes("BET");

                        if (isBetAction && betDelta > 0) {
                            const delay = spawnChipStream(`player-${event.player_id}`, 'pot', betDelta);
                            trackingBets[event.player_id] = targetBet;

                            await new Promise((resolve) => setTimeout(resolve, delay + betChipAddedTime));
                            updateIntermediateState(event.player_id, -betDelta);
                        }
                    }

                    setActiveMove(null);
                }
            }

            // finally, handle the showdown. this is always the final event in a round
            if (isShowdown) {
                // artificial intermediate state to show only cards while staggering
                // chip modification
                const baseState = previousState || nextState;
                const cardRevealState: GameStateData = {
                    ...nextState,
                    overall_sum: baseState.overall_sum,
                    pots: baseState.pots,
                    players: nextState.players.map(np => {
                        const prevP = baseState.players.find(pp => pp.id === np.id);
                        return {
                            ...np,
                            chips: prevP ? prevP.chips : np.chips
                        };
                    })
                };

                // update to pseudo-state to reveal cards
                setGameState(cardRevealState);
                gameStateRef.current = cardRevealState;

                // then wait for flips and hand out pots
                await new Promise((resolve) => setTimeout(resolve, showdownFlipDelay));
                await handlePotDistributions();
            }
            setGameState(nextState);
            gameStateRef.current = nextState;
        }

        isProcessingQueue.current = false;
    };

    useEffect(() => {
        if (!socket || !isConnected) {
            showToast("Disconnected from server", "error");
            navigate('/');
            return;
        }

        socket.emit("requestGameState");

        const handleClientInfo = (data: { playerId: number }) => setMyId(data.playerId);
        const handleLobbyUpdate = (data: { players: PlayerLobbyInfo[] }) => setLobbyPlayers(data.players);

        const handleGameUpdate = (data: any) => {
            let events: MoveEvent[] = [];
            let nextState: GameStateData;

            const parsedData = typeof data === "string" ? JSON.parse(data) : data;

            if (parsedData.game_state) {
                events = parsedData.events || [];
                nextState = typeof parsedData.game_state === "string" 
                    ? JSON.parse(parsedData.game_state) 
                    : parsedData.game_state;
            } else {
                events = parsedData.events || [];
                nextState = parsedData;
            }

            eventQueueRef.current.push({ events, nextState });
            processEventQueue();
        };

        const handleInfo = (msg: string) => showToast(msg, "info");
        const handleError = (msg: string) => showToast(msg, "error");

        socket.on("clientInfo", handleClientInfo);
        socket.on("lobbyUpdate", handleLobbyUpdate);
        socket.on("gameUpdate", handleGameUpdate);
        socket.on("gameStart", handleGameUpdate);
        socket.on("info", handleInfo);
        socket.on("error", handleError);

        return () => {
            socket.off("clientInfo", handleClientInfo);
            socket.off("lobbyUpdate", handleLobbyUpdate);
            socket.off("gameUpdate", handleGameUpdate);
            socket.off("gameStart", handleGameUpdate);
            socket.off("info", handleInfo);
            socket.off("error", handleError);
        };
    }, [socket, isConnected, navigate, showToast]);

    const startNextRound = () => {
        if (!socket || !isHost) return;
        socket.emit("startGame");
    };

    const handleAction = (action: string, amount: number = 0) => {
        if (!socket) return;
        const safeAmount = meInGame ? Math.min(amount, meInGame.chips) : amount;
        socket.emit("playerMove", { action, amount: safeAmount });
    };

    const leaveGame = () => {
        if (socket) socket.emit("leaveGame");
        navigate('/');
    };

    const isMyTurn = meInGame?.is_turn && gameState?.round_name !== "room" && gameState?.round_name !== "preround" && gameState?.round_name !== "showdown";
    const currentBetToCall = (gameState?.highest_bet || 0) - (meInGame?.total_bet || 0);

    return (
        <div className={styles.gameContainer}>
            <div className={styles.tableHeader}>
                <div className={styles.headerInfo}>
                    <span>Room: <strong>{roomId?.toUpperCase()}</strong></span>
                    <span>Round: <strong>{gameState?.round_name ? gameState.round_name.toUpperCase() : "LOBBY"}</strong></span>
                </div>
                <button type="button" className={styles.btnWrapper} onClick={leaveGame}>
                    <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                        Leave Game
                    </PixelBox>
                </button>
            </div>

            <div className={styles.pokerFelt}>
                <canvas ref={canvasRef} className={styles.chipCanvas} />

                {gameState?.round_name !== "room" && (
                    <div className={styles.centerBoard}>
                        <div className={styles.deckStack}>
                            {/* handles making the deck stack - height varies with card count */}
                            {(() => {
                                const remainingCards = gameState?.deck_cards ?? 52;
                                const layerCount = Math.max(1, Math.ceil(remainingCards / 4));

                                return Array.from({ length: layerCount }).map((_, i) => {
                                    const offsetPx = -i * 2;
                                    const isTopCard = i === layerCount - 1;

                                    return (
                                        <div
                                            key={`deck-layer-${i}`}
                                            ref={isTopCard ? deckRef : null}
                                            className={styles.deckCardLayer}
                                            style={{
                                                transform: `translateY(${offsetPx}px)`,
                                                zIndex: i + 1,
                                            }}
                                        />
                                    );
                                });
                            })()}
                        </div>

                        <div className={styles.potDisplay} ref={potRef}>
                            Pot: <strong>{gameState?.overall_sum || 0}</strong>
                        </div>

                        {/* renders community cards / placeholders */}
                        <div className={styles.communityCards}>
                            {([0, 1, 2, 3, 4]).map((idx) => (
                                <PlayingCard 
                                    key={`community-${idx}-${gameState?.community_cards[idx] || 'empty'}`} 
                                    cardStr={gameState?.community_cards[idx] || null} 
                                    deckRef={deckRef}
                                />
                            ))}
                        </div>
                    </div>
                )}

                {gameState?.players.map((p, index) => {
                    // ensures players are spaced evenly around the table
                    const totalPlayers = gameState.players.length;
                    const angle = (index / totalPlayers) * 2 * Math.PI + Math.PI / 2;
                    const rx = 34;
                    const ry = 32;
                    const left = 50 + rx * Math.cos(angle);
                    const top = 50 + ry * Math.sin(angle);

                    const lobbyInfo = lobbyPlayers.find(lp => lp.id === p.id);
                    const isMe = p.id === myId;
                    const isActing = activeMove?.player_id === p.id;
                    const isChecking = isActing && activeMove?.action.toUpperCase().includes("CALL") && currentBetToCall === 0;
                    const borderColour = (p.is_turn && gameState?.round_name !== "room") ? "var(--input-focus-colour)" : boxBorderColour;

                    return (
                        <div 
                            key={p.id} 
                            ref={(el) => { seatRefs.current[p.id] = el; }}
                            className={`
                                ${styles.playerSeat} 
                                ${p.folded ? styles.foldedSeat : ''} 
                                ${isChecking ? styles.animatingCheck : ''}
                            `}
                            style={{ left: `${left}%`, top: `${top}%` }}
                        >
                            {p.is_dealer && gameState?.round_name !== "room" && 
                                <div className={styles.dealerBadgeWrapper}>
                                    <PixelBox innerClassName={styles.dealerBadgeInner} borderColour={borderColour}>
                                        D
                                    </PixelBox>
                                </div>
                            }
                            
                            <PixelBox 
                                innerClassName={styles.seatInner} 
                                borderColour={borderColour}
                                backgroundColour={isMe ? "#2a2a38" : "#1a1a24"}
                            >
                                <div className={styles.seatName}>
                                    {lobbyInfo?.name || `Player ${p.id}`} {isMe && "(You)"}
                                </div>
                                <div className={styles.seatChips}>Chips: {p.chips}</div>
                                
                                {gameState?.round_name !== "room" && (
                                    <>
                                        <div className={styles.seatBet}>Bet: {p.total_bet || '0'}</div>

                                        {p.hand_type && (
                                            <div className={styles.handTypeTag}>{p.hand_type}</div>
                                        )}

                                        {!p.hand_type && (
                                            <div className={styles.handPlaceholder}></div>
                                        )}

                                        <div className={styles.holeCards}>
                                            {[0, 1].map((cardIndex) => {
                                                const cardStr = p.hole_cards && p.hole_cards[cardIndex] ? p.hole_cards[cardIndex] : null;

                                                return (
                                                    <PlayingCard
                                                        key={`player-${p.id}-hole-${cardIndex}`}
                                                        cardStr={cardStr}
                                                        deckRef={deckRef}
                                                    />
                                                );
                                            })}
                                        </div>
                                    </>
                                )}
                            </PixelBox>
                        </div>
                    );
                })}
            </div>

            <div className={styles.actionHud}>
                {isMyTurn && gameState?.round_name !== "room" && (
                    <div className={styles.turnBar}>
                        <button type="button" className={styles.btnWrapper} onClick={() => handleAction("fold")}>
                            <PixelBox innerClassName={`${styles.btnInner} ${styles.btnDanger}`} borderColour={boxBorderColour}>
                                Fold
                            </PixelBox>
                        </button>

                        <button type="button" className={styles.btnWrapper} onClick={() => handleAction("call")}>
                            <PixelBox innerClassName={`${styles.btnInner} ${currentBetToCall > 0 ? styles.btnPrimary : ''}`} borderColour={boxBorderColour}>
                                {currentBetToCall <= 0 ? "Check" : `Call (${currentBetToCall})`}
                            </PixelBox>
                        </button>

                        <NumberSetting
                            label='' 
                            range=''
                            min={10}
                            max={meInGame?.chips || 1000}
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

                        <button type="button" className={styles.btnWrapper} onClick={() => handleAction("raise", raiseAmount)}>
                            <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                Raise
                            </PixelBox>
                        </button>
                    </div>
                )}

                {gameState?.round_name === "room" && (
                    <button type="button" className={styles.btnWrapper} onClick={() => navigate(`/lobby/${roomId}`)}>
                        <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                            Return to Lobby
                        </PixelBox>
                    </button>
                )}

                {(gameState?.round_name === "showdown" || gameState?.round_name === "preround") && (
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