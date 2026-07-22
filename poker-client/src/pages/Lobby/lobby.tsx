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
    deckType: string;
    turnTimeout: number | '';
    isPrivate: boolean;
}

const computedStyles = getComputedStyle(document.documentElement);
const boxBorderColour = computedStyles.getPropertyValue('--border-colour').trim();

export function Lobby() {
    const { roomId } = useParams<{ roomId: string }>();
    const { socket, isConnected } = useSocket();
    const { showToast } = useToast();
    const navigate = useNavigate();

    const [myId, setMyId] = useState<number | null>(null);
    const myIdRef = useRef<number | null>(null);
    myIdRef.current = myId;

    const [players, setPlayers] = useState<Player[]>([]);
    const [config, setConfig] = useState<LobbyConfig>({
        maxPlayers: 4,
        blindSize: 10,
        minRaise: 20,
        startingChips: 1000,
        deckType: "standard",
        turnTimeout: 30,
        isPrivate: false
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

        const handleLobbyUpdate = (data: { players: Player[], config: LobbyConfig }) => {
            setPlayers(data.players);
            
            // ignore backend config updates if the user is the host - allows
            // host to input values without aggressive forced server changes
            const me = data.players.find(p => p.id === myIdRef.current);
            if (!me?.isHost) {
                setConfig(data.config);
            }
        };

        const handleClientInfo = (data: { playerId: number }) => {
            setMyId(data.playerId);
        };

        const handleKicked = () => {
            showToast("Kicked from lobby", "warning");
            navigate('/');
        };

        const handleGameStarted = () => {
            showToast("Game starting...", "success");
        };

        socket.on("lobbyUpdate", handleLobbyUpdate);
        socket.on("clientInfo", handleClientInfo);
        socket.on("kicked", handleKicked);
        socket.on("gameStarted", handleGameStarted);

        return () => {
            socket.off("lobbyUpdate", handleLobbyUpdate);
            socket.off("clientInfo", handleClientInfo);
            socket.off("kicked", handleKicked);
            socket.off("gameStarted", handleGameStarted);
        };
    }, [socket, isConnected, navigate, showToast, roomId]);

    const updateConfig = (key: keyof LobbyConfig, value: string | number | boolean) => {
        if (!isHost || !socket) return;
        const newConfig = { ...config, [key]: value };
        setConfig(newConfig);
        socket.emit("updateConfig", { roomId, config: newConfig });
    };

    const addBot = () => {
        if (!isHost || !socket) return;
        if (players.length >= (Number(config.maxPlayers) || 0)) {
            return showToast("Lobby is full", "warning");
        }
        socket.emit("addBot", { roomId, botType: botToAdd });
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
        socket.emit("startGame", { roomId });
    };

    const leaveLobby = () => {
        if (socket) socket.emit("leaveGame");
        navigate('/');
    };

    return (
        <div className={styles.container}>
            <div className={styles.canvasPlaceholder}>
                
            </div>

            <PixelBox 
                className={styles.lobbyOuter} 
                innerClassName={styles.lobbyInner}
                borderColour={boxBorderColour}
                backgroundColour="#2b2b36"
            >
                <div className={styles.header}>
                    <h1 className={styles.title}>Game Room</h1>
                    <div className={styles.roomCode}>Room Code: <strong>{roomId?.toUpperCase()}</strong></div>
                </div>

                <div className={styles.splitLayout}>
                    
                    <div className={styles.formStack}>
                        <h2 className={styles.panelTitle}>Settings</h2>

                        <div className={styles.formGroupRow}>
                            <div className={styles.formGroup} style={{ flex: 1 }}>
                                <label>Deck Type</label>
                                <div className={styles.inputOuter}>
                                    <PixelBox innerClassName={styles.inputInner} borderColour={boxBorderColour}>
                                        <select 
                                            className={styles.formControl}
                                            value={config.deckType}
                                            disabled={!isHost}
                                            onChange={(e) => updateConfig("deckType", e.target.value)}
                                        >
                                            <option value="standard">Standard (52)</option>
                                            <option value="restricted">Restricted (40)</option>
                                            <option value="double">Double Deck (104)</option>
                                            <option value="half">Half Deck (26)</option>
                                            <option value="1joker">1 Joker (53)</option>
                                            <option value="2jokers">2 Jokers (54)</option>
                                        </select>
                                    </PixelBox>
                                </div>
                            </div>

                            <NumberSetting 
                                label="Players" range="2-8" 
                                value={config.maxPlayers} min={2} max={8} 
                                disabled={!isHost} onChange={(val) => updateConfig("maxPlayers", val)} 
                                styles={styles} boxBorderColour={boxBorderColour}
                            />
                        </div>

                        <div className={styles.formGroupRow}>
                            <NumberSetting 
                                label="Starting Chips" range="100-10000" 
                                value={config.startingChips} min={100} max={10000} step={100} 
                                disabled={!isHost} onChange={(val) => updateConfig("startingChips", val)} 
                                styles={styles} boxBorderColour={boxBorderColour}
                            />
                            
                            <NumberSetting 
                                label="Turn Timeout" range="10-120s" 
                                value={config.turnTimeout} min={10} max={120} 
                                disabled={!isHost} onChange={(val) => updateConfig("turnTimeout", val)} 
                                styles={styles} boxBorderColour={boxBorderColour}
                            />
                        </div>

                        <div className={styles.formGroupRow}>
                            <NumberSetting 
                                label="Small Blind" range="0-200" 
                                value={config.blindSize} min={0} max={200} step={5} 
                                disabled={!isHost} onChange={(val) => updateConfig("blindSize", val)} 
                                styles={styles} boxBorderColour={boxBorderColour}
                            />

                            <NumberSetting 
                                label="Min Raise" range="1-200" 
                                value={config.minRaise} min={1} max={200} step={5} 
                                disabled={!isHost} onChange={(val) => updateConfig("minRaise", val)} 
                                styles={styles} boxBorderColour={boxBorderColour}
                            />
                        </div>

                        <div className={styles.formGroup}>
                            <label>Lobby Privacy</label>
                            <div className={styles.inputOuter}>
                                <PixelBox innerClassName={styles.inputInner} borderColour={boxBorderColour}>
                                    <select 
                                        className={styles.formControl}
                                        value={config.isPrivate ? "private" : "public"}
                                        disabled={!isHost}
                                        onChange={(e) => updateConfig("isPrivate", e.target.value === "private")}
                                    >
                                        <option value="public">Public</option>
                                        <option value="private">Private</option>
                                    </select>
                                </PixelBox>
                            </div>
                        </div>

                        {isHost && (
                            <div className={styles.formGroupRow} style={{ marginTop: '1rem' }}>
                                <div className={styles.formGroup} style={{ flex: 2 }}>
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
                                <div style={{ flex: 1 }}>
                                    <button type="button" className={styles.btnWrapper} onClick={addBot}>
                                        <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                            Add Bot
                                        </PixelBox>
                                    </button>
                                </div>
                            </div>
                        )}
                    </div>

                    <div className={styles.formStack}>
                        <h2 className={styles.panelTitle}>Players ({players.length}/{Math.max(Math.min(Number(config.maxPlayers) || 0, 8), 2)})</h2>
                        
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
                                                <PixelBox innerClassName={`${styles.btnInner} ${styles.btnDanger}`} borderColour={boxBorderColour} style={{ fontSize: "2.5rem" }}>
                                                    x
                                                </PixelBox>
                                            </button>
                                        </div>
                                    )}
                                </div>
                            ))}
                        </div>

                        <div className={styles.formGroupRow} style={{ marginTop: 'auto' }}>
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
            </PixelBox>
        </div>
    );
}