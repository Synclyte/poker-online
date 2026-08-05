import React, { useState, useEffect, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useSocket } from '../../context/SocketContext';
import { useToast } from '../../context/ToastContext';
import { PixelBox } from '../../components/PixelBox/pixelbox';
import { NumberSetting } from '../../components/NumberSetting/numbersetting';
import styles from './Lobby.module.css';

interface Player {
    id: number;
    name: string;
    isHost: boolean;
    isBot: boolean;
    botType?: string;
}

interface LobbyConfig {
    maxPlayers: number | '';
    blindSize: number | '';
    minRaise: number | '';
    startingChips: number | '';
    specialCardLimit: number | '';
    deckType: string;
    turnTimeout: number | '';
    isPrivate: boolean;
    roundLimit: number | '';
}
type AIType = "Risky" | "Safe" | "Smart" | "Random";

export const DECK_OPTIONS = [
    { value: "standard", label: "Standard (52)" },
    { value: "restricted", label: "Restricted (40)" },
    { value: "double", label: "Double Deck (104)" },
    { value: "half", label: "Half Deck (26)" },
    { value: "onejokerstandard", label: "1 Joker (53)" },
    { value: "twojokerstandard", label: "2 Jokers (54)" },
] as const;

export type DeckType = typeof DECK_OPTIONS[number]['value'];
type MainView = 'lobby' | 'configure';
type LeftPanelMode = 'overview' | 'add-bot';

const computedStyles = getComputedStyle(document.documentElement);
const boxBorderColour = computedStyles.getPropertyValue('--border-colour').trim() || "#dcdcdc";

export function Lobby() {
    const { roomId } = useParams<{ roomId: string }>();
    const { socket, isConnected } = useSocket();
    const { showToast } = useToast();
    const navigate = useNavigate();

    const [myId, setMyId] = useState<number | null>(null);
    const myIdRef = useRef<number | null>(null);
    myIdRef.current = myId;

    const [view, setView] = useState<MainView>('lobby');
    const [leftPanelMode, setLeftPanelMode] = useState<LeftPanelMode>('overview');

    const [players, setPlayers] = useState<Player[]>([]);
    const [config, setConfig] = useState<LobbyConfig>({
        maxPlayers: 4,
        blindSize: 10,
        minRaise: 20,
        startingChips: 1000,
        specialCardLimit: 3,
        deckType: "standard",
        turnTimeout: 30,
        isPrivate: false,
        roundLimit: 30,
    });

    const [botToAdd, setBotToAdd] = useState<string>("Smart");
    const isHost = players.find(p => p.id === myId)?.isHost || false;

    useEffect(() => {
        if (!socket || !isConnected) {
            showToast("Disconnected from server", "error");
            navigate('/');
            return;
        }

        socket.emit("requestLobbyInfo");

        const handleLobbyUpdate = (data: { players: Player[], config: LobbyConfig, round?: string }) => {
            setPlayers(data.players);

            const me = data.players.find(p => p.id === myIdRef.current);
            if (!me?.isHost) {
                setConfig(data.config);
            }
        };

        const handleClientInfo = (data: { playerId: number }) => setMyId(data.playerId);

        const handleKicked = () => {
            showToast("Kicked from lobby", "warning");
            navigate('/');
        };

        const handleGameStart = () => {
            showToast("Game starting...", "success");
            navigate(`/game/${roomId}`);
        };

        const handleSuccess = (data: string) => showToast(`${data}`, "success");
        const handleError = (data: string) => showToast(`${data}`, "error");
        const handleInfo = (data: string) => showToast(`${data}`, "info");

        const handleHostPromote = (data: { playerId: number }) => {
            if (data.playerId === myIdRef.current) {
                showToast("You have been promoted to host", "info");
            } else {
                const player = players.find(p => p.id === data.playerId);
                if (!player) showToast(`Player ${data.playerId} was promoted to host`, "info");
                else showToast(`${player.name} was promoted to host`, "info");
            }
        };

        socket.on("lobbyUpdate", handleLobbyUpdate);
        socket.on("clientInfo", handleClientInfo);
        socket.on("kicked", handleKicked);
        socket.on("gameStart", handleGameStart);
        socket.on("info", handleInfo);
        socket.on("success", handleSuccess);
        socket.on("error", handleError);
        socket.on("hostPromote", handleHostPromote);

        return () => {
            socket.off("lobbyUpdate", handleLobbyUpdate);
            socket.off("clientInfo", handleClientInfo);
            socket.off("kicked", handleKicked);
            socket.off("gameStart", handleGameStart);
            socket.off("info", handleInfo);
            socket.off("success", handleSuccess);
            socket.off("error", handleError);
            socket.off("hostPromote", handleHostPromote);
        };
    }, [socket, isConnected, navigate, showToast]);

    const updateConfig = (key: keyof LobbyConfig, value: string | number | boolean) => {
        if (!isHost || !socket) return;

        const newConfig = { ...config, [key]: value };
        setConfig(newConfig);
        socket.emit("updateConfig", { roomId, config: JSON.stringify(newConfig) });
    };

    const addBot = () => {
        if (!isHost || !socket) return;
        if (players.length >= (Number(config.maxPlayers) || 0)) {
            return showToast("Lobby is full", "warning");
        }
        socket.emit("addBot", { roomId, botType: botToAdd as AIType });
    };

    const kickPlayer = (playerId: number) => {
        if (!isHost || !socket) return;
        socket.emit("kickPlayer", { roomId, playerId: playerId.toString() });
    };

    const promoteHost = (playerId: number) => {
        if (!isHost || !socket) return;
        socket.emit("promoteHost", { roomId, playerId: playerId.toString() });
    };

    const startGame = () => {
        if (!isHost || !socket) return;
        if (players.length < 2) return showToast("Need at least 2 players to start", "warning");
        socket.emit("initialiseGame", { roomId });
    };

    const leaveLobby = () => {
        if (socket) socket.emit("leaveGame");
        navigate('/');
    };

    return (
        <div className={styles.container}>
            <div className={styles.canvasPlaceholder} />

            <PixelBox
                className={styles.lobbyOuter}
                innerClassName={styles.lobbyInner}
                borderColour={boxBorderColour}
                backgroundColour="#2b2b36"
            >
                <div className={styles.header}>
                    <h1 className={styles.title}>
                        {view === 'configure' ? "Game Configuration" : "Game Room"}
                    </h1>
                    <div className={styles.roomCode}>Room Code: <strong>{roomId?.toUpperCase()}</strong></div>
                </div>

                {view === 'lobby' && (
                    <div className={styles.splitLayout}>
                        <div className={styles.formStack}>
                            <h2 className={styles.panelTitle}>
                                {leftPanelMode === 'overview' ? "Settings" : "Add Bot"}
                            </h2>

                            {leftPanelMode === 'overview' ? (
                                <>
                                    <div className={styles.overviewScrollable}>
                                        <div className={styles.summaryRow}>
                                            <span className={styles.summaryKey}>Privacy:</span>
                                            <span className={styles.summaryVal}>{config.isPrivate ? "PRIVATE" : "PUBLIC"}</span>
                                        </div>
                                        <div className={styles.summaryRow}>
                                            <span className={styles.summaryKey}>Max Players:</span>
                                            <span className={styles.summaryVal}>{config.maxPlayers || 0}</span>
                                        </div>
                                        <div className={styles.summaryRow}>
                                            <span className={styles.summaryKey}>Deck Type:</span>
                                            <span className={styles.summaryVal}>{config.deckType.toUpperCase()}</span>
                                        </div>
                                        <div className={styles.summaryRow}>
                                            <span className={styles.summaryKey}>Starting Chips:</span>
                                            <span className={styles.summaryVal}>{config.startingChips || 0}</span>
                                        </div>
                                        <div className={styles.summaryRow}>
                                            <span className={styles.summaryKey}>Special Card Limit:</span>
                                            <span className={styles.summaryVal}>{config.specialCardLimit || 0}</span>
                                        </div>
                                        <div className={styles.summaryRow}>
                                            <span className={styles.summaryKey}>Small Blind:</span>
                                            <span className={styles.summaryVal}>{config.blindSize || 0}</span>
                                        </div>
                                        <div className={styles.summaryRow}>
                                            <span className={styles.summaryKey}>Min Raise:</span>
                                            <span className={styles.summaryVal}>{config.minRaise || 0}</span>
                                        </div>
                                        <div className={styles.summaryRow}>
                                            <span className={styles.summaryKey}>Turn Timeout:</span>
                                            <span className={styles.summaryVal}>{config.turnTimeout || 0}s</span>
                                        </div>
                                        <div className={styles.summaryRow}>
                                            <span className={styles.summaryKey}>Round Limit:</span>
                                            <span className={styles.summaryVal}>{config.roundLimit || 0}</span>
                                        </div>
                                    </div>

                                    {isHost && (
                                        <div className={styles.formGroupRow}>
                                            <button type="button" className={styles.btnWrapper} onClick={() => setLeftPanelMode('add-bot')}>
                                                <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                                    Add Bots
                                                </PixelBox>
                                            </button>
                                            <button type="button" className={styles.btnWrapper} onClick={() => setView('configure')}>
                                                <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                                    Configure
                                                </PixelBox>
                                            </button>
                                        </div>
                                    )}
                                </>
                            ) : (
                                <>
                                    <div className={styles.columnScrollContainer}>
                                        <div className={styles.columnGroup}>
                                            <span className={styles.columnHeader}>Bot Settings</span>
                                            <div className={styles.formGroup}>
                                                <label>Bot Type</label>
                                                <div className={styles.inputOuter}>
                                                    <PixelBox innerClassName={styles.inputInner} borderColour={boxBorderColour}>
                                                        <select
                                                            className={styles.formControl}
                                                            value={botToAdd}
                                                            onChange={(e) => setBotToAdd(e.target.value)}
                                                        >
                                                            <option value="Smart">Smart Bot</option>
                                                            <option value="Random">Random Bot</option>
                                                            <option value="Risky">Risky Bot</option>
                                                            <option value="Safe">Safe Bot</option>
                                                        </select>
                                                    </PixelBox>
                                                </div>
                                            </div>
                                        </div>
                                    </div>

                                    <div className={styles.formGroupRow}>
                                        <button type="button" className={styles.btnWrapper} onClick={() => setLeftPanelMode('overview')}>
                                            <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                                Back
                                            </PixelBox>
                                        </button>
                                        <button type="button" className={styles.btnWrapper} onClick={addBot}>
                                            <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                                Add Bot
                                            </PixelBox>
                                        </button>
                                    </div>
                                </>
                            )}
                        </div>

                        <div className={styles.formStack}>
                            <h2 className={styles.panelTitle}>
                                Players ({players.length}/{Math.max(Math.min(Number(config.maxPlayers) || 0, 8), 2)})
                            </h2>

                            <div className={styles.playerList}>
                                {players.map((player) => (
                                    <div key={player.id} className={styles.playerItem}>
                                        <div className={styles.playerInfo}>
                                            <span className={styles.playerName}>
                                                {player.name} {player.id === myId && "(You)"}
                                            </span>
                                            <span className={styles.playerRole}>
                                                {player.isHost ? "Host" : player.isBot ? "Bot" : "Player"}
                                            </span>
                                        </div>

                                        {isHost && player.id !== myId && (
                                            <div className={styles.playerActions}>
                                                {!player.isBot && (
                                                    <button className={styles.btnWrapper} title="Promote to Host" onClick={() => promoteHost(player.id)}>
                                                        <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                                            +
                                                        </PixelBox>
                                                    </button>
                                                )}
                                                <button className={styles.btnWrapper} title="Kick Player" onClick={() => kickPlayer(player.id)}>
                                                    <PixelBox innerClassName={`${styles.btnInner} ${styles.btnDanger}`} borderColour={boxBorderColour}>
                                                        x
                                                    </PixelBox>
                                                </button>
                                            </div>
                                        )}
                                    </div>
                                ))}
                            </div>

                            <div className={styles.formGroupRow}>
                                <button type="button" className={styles.btnWrapper} onClick={leaveLobby}>
                                    <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                        Leave Lobby
                                    </PixelBox>
                                </button>

                                {isHost && (
                                    <button type="button" className={styles.btnWrapper} onClick={startGame}>
                                        <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                            Start Game
                                        </PixelBox>
                                    </button>
                                )}
                            </div>
                        </div>
                    </div>
                )}

                {view === 'configure' && (
                    <div className={styles.configContainer}>
                        <h2 className={styles.panelTitle}>Options</h2>
                        <div className={styles.configColumns}>
                            <div className={styles.columnScrollContainer}>
                                <div className={styles.columnGroup}>
                                    <span className={styles.columnHeader}>Rules</span>

                                    <div className={styles.formGroup}>
                                        <label>Privacy</label>
                                        <div className={styles.inputOuter}>
                                            <PixelBox innerClassName={styles.inputInner} borderColour={boxBorderColour}>
                                                <select
                                                    className={styles.formControl}
                                                    value={config.isPrivate ? "private" : "public"}
                                                    onChange={(e) => updateConfig("isPrivate", e.target.value === "private")}
                                                >
                                                    <option value="public">Public</option>
                                                    <option value="private">Private</option>
                                                </select>
                                            </PixelBox>
                                        </div>
                                    </div>


                                    <NumberSetting
                                        label="Player Capacity" range="2-8"
                                        value={config.maxPlayers} min={2} max={8}
                                        onChange={(val) => updateConfig("maxPlayers", val)}
                                        styles={styles} boxBorderColour={boxBorderColour}
                                    />

                                    <div className={styles.formGroup}>
                                        <label>Deck Type</label>
                                        <div className={styles.inputOuter}>
                                            <PixelBox innerClassName={styles.inputInner} borderColour={boxBorderColour}>
                                                <select
                                                    className={styles.formControl}
                                                    value={config.deckType}
                                                    onChange={(e) => updateConfig("deckType", e.target.value as DeckType)}
                                                >
                                                    {DECK_OPTIONS.map((opt) => (
                                                        <option key={opt.value} value={opt.value}>
                                                            {opt.label}
                                                        </option>
                                                    ))}
                                                </select>
                                            </PixelBox>
                                        </div>
                                    </div>

                                    <NumberSetting
                                        label="Turn Timeout (s)" range="10-120"
                                        value={config.turnTimeout} min={10} max={120}
                                        onChange={(val) => updateConfig("turnTimeout", val)}
                                        styles={styles} boxBorderColour={boxBorderColour}
                                    />
                                </div>
                            </div>

                            <div className={styles.columnScrollContainer}>
                                <div className={styles.columnGroup}>
                                    <span className={styles.columnHeader}>Stakes</span>

                                    <NumberSetting
                                        label="Starting Chips" range="100-10000"
                                        value={config.startingChips} min={100} max={10000} step={100}
                                        onChange={(val) => updateConfig("startingChips", val)}
                                        styles={styles} boxBorderColour={boxBorderColour}
                                    />

                                    <NumberSetting
                                        label="Small Blind" range="0-200"
                                        value={config.blindSize} min={0} max={200} step={5}
                                        onChange={(val) => updateConfig("blindSize", val)}
                                        styles={styles} boxBorderColour={boxBorderColour}
                                    />

                                    <NumberSetting
                                        label="Minimum Raise" range="1-200"
                                        value={config.minRaise} min={1} max={200} step={5}
                                        onChange={(val) => updateConfig("minRaise", val)}
                                        styles={styles} boxBorderColour={boxBorderColour}
                                    />

                                    <NumberSetting
                                        label="Round Limit" range="5-100"
                                        value={config.roundLimit} min={5} max={100} step={1}
                                        onChange={(val) => updateConfig("roundLimit", val)}
                                        styles={styles} boxBorderColour={boxBorderColour}
                                    />

                                    <NumberSetting
                                        label="Special Card Limit" range="0-10"
                                        value={config.specialCardLimit} min={0} max={10} step={1}
                                        onChange={(val) => updateConfig("specialCardLimit", val)}
                                        styles={styles} boxBorderColour={boxBorderColour}
                                    />
                                </div>
                            </div>
                        </div>

                        <div className={styles.formGroupRow} style={{ justifyContent: 'flex-start' }}>
                            <div style={{ width: '180px' }}>
                                <button type="button" className={styles.btnWrapper} onClick={() => setView('lobby')}>
                                    <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                        Back
                                    </PixelBox>
                                </button>
                            </div>
                        </div>
                    </div>
                )}
            </PixelBox>
        </div>
    );
}