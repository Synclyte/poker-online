import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useSocket } from '../../context/socketcontext';
import styles from './home.module.css';
import { useToast } from '../../context/toastcontext';
import { PixelBox } from '../../components/PixelBox/pixelbox';
import { NumberSetting } from '../../components/NumberSetting/numbersetting';

type MenuView = "create" | "join-public" | "join-code" | "main";
type Round = "room" | "preround" | "turn" | "river" | "showdown";

function displayRound(round: Round) {
    switch (round) {
        case "room": return "Room";
        case "preround": return "Pre-Round";
        case "turn": return "Turn";
        case "river": return "River";
        case "showdown": return "Showdown";
    }
}

interface PublicLobbyData {
    roomId: string,
    activeConnections: number,
    maxPlayers: number,
    round: Round,
}

const computedStyles = getComputedStyle(document.documentElement);
const boxBorderColour = computedStyles.getPropertyValue('--border-colour').trim();

export function isValidPlayerName(name: string): boolean {
    if (typeof name !== "string") return false;
    for (let i = 0; i < name.length; i++) {
        const code = name.charCodeAt(i);
        if (code < 32 || code > 255 || code === 127) return false;
    }
    const sanitized = name.trim().replace(/ +/g, ' ');
    return sanitized.length >= 3 && sanitized.length <= 12;
}

export function Home() {
    const { socket, isConnected } = useSocket();
    let navigate = useNavigate();
    const { showToast } = useToast();

    const [view, setView] = useState<MenuView>("main");

    const [isCreating, setIsCreating] = useState<boolean>(false);
    const [playerCapacity, setPlayerCapacity] = useState<number>(4);
    const [isPrivate, setIsPrivate] = useState<boolean>(false);

    const [roomCode, setRoomCode] = useState<string>("");
    const [isJoining, setIsJoining] = useState<boolean>(false);
    const [publicLobbies, setPublicLobbies] = useState<PublicLobbyData[]>([]);
    const [playerName, setPlayerName] = useState<string>(() => localStorage.getItem("playerName") || "");

    useEffect(() => {
        if (socket && isConnected) {
            socket.emit("leaveGame");
            localStorage.removeItem("sessionToken");
        }
    }, [socket, isConnected]);

    useEffect(() => {
        if (!socket) return;

        const handleGameCreate = (data: { roomId: string, sessionToken: string }) => {
            localStorage.setItem("sessionToken", data.sessionToken);
            showToast("Successfully created game room", "success");
            navigate(`/lobby/${data.roomId}`);
        }

        const handleJoinGame = (data: { roomId: string, sessionToken: string }) => {
            localStorage.setItem("sessionToken", data.sessionToken);
            showToast("Successfully joined game room", "success");
            navigate(`/lobby/${data.roomId}`);
        }

        const handleGetPublicLobbies = (data: { roomId: string, activeConnections: number, maxPlayers: number, round: Round }[]) => {
            setPublicLobbies(data);
        }

        const handleError = (message: string) => {
            showToast(message, "error");
            setIsJoining(false);
            setIsCreating(false);
        }

        socket.on("gameCreated", handleGameCreate);
        socket.on("joinSuccess", handleJoinGame);
        socket.on("error", handleError);
        socket.on("publicLobbies", handleGetPublicLobbies);

        return () => {
            socket.off("gameCreated", handleGameCreate);
            socket.off("joinSuccess", handleJoinGame);
            socket.off("error", handleError);
            socket.off("publicLobbies", handleGetPublicLobbies);
        }
    }, [socket, navigate, showToast]);

    const handleGameCreateForm = (e: React.SubmitEvent<HTMLFormElement>) => {
        e.preventDefault();

        if (!socket || !isConnected) return showToast("Failed to create game - not connected to server", "error");

        if (playerCapacity < 2 || playerCapacity > 8) {
            return showToast("Player capacity must be between 2 and 8", "error");
        }

        setIsCreating(true);
        const config = { isPrivate: isPrivate, maxPlayers: playerCapacity, playerName: playerName.trim() || undefined };
        socket.emit("createGame", JSON.stringify(config));
    }

    const joinGameByCode = (e: React.SubmitEvent<HTMLFormElement>) => {
        e.preventDefault();
        joinGame();
    }

    const joinGame = (specificCode?: string) => {
        if (!socket || !isConnected) return showToast("Failed to join game - not connected to server", "error");

        const targetCode = specificCode || roomCode;
        if (!targetCode) return;

        setIsJoining(true);
        socket.emit("joinGame", { roomId: targetCode.toLowerCase(), playerName: playerName.trim() || undefined });
    }

    const back = () => {
        setView("main");
        setRoomCode("");
        setIsJoining(false);
        setIsCreating(false);
    }

    const getPublicLobbies = () => {
        if (!socket || !isConnected) return;
        return socket.emit("getPublicLobbies");
    }

    return (
        <>
            <div className={styles.container}>
                {/*
            <div className={styles.canvasPlaceholder}>

            </div>
            */}

                <div className={styles.uiWrapperOuter}>
                    <PixelBox
                        innerClassName={styles.uiWrapperInner}
                        borderColour={boxBorderColour}
                        backgroundColour="#2b2b36"
                    >
                        <h1 className={styles.title}>Poker?</h1>

                        {view === 'main' && (
                            <div className={styles.menuStack}>
                                {(() => {
                                    const isNameValid = isValidPlayerName(playerName);
                                    return (
                                        <div className={styles.formGroup} style={{ alignSelf: 'center', width: '100%', maxWidth: '240px', alignItems: 'center', marginBottom: '0.25rem' }}>
                                            <label style={{ fontSize: '1.4rem', color: isNameValid ? 'var(--input-focus-colour)' : '#ff5252', fontFamily: "'Jersey 10', sans-serif", textShadow: 'none' }}>
                                                Player Name {!isNameValid && "(3-12 chars)"}
                                            </label>
                                            <div className={styles.inputOuter} style={{ width: '100%' }}>
                                                <PixelBox innerClassName={styles.inputInner} borderColour={isNameValid ? boxBorderColour : '#ff5252'}>
                                                    <input
                                                        type="text"
                                                        maxLength={16}
                                                        className={styles.formControl}
                                                        style={{ color: isNameValid ? 'var(--text-colour)' : '#ff5252' }}
                                                        value={playerName}
                                                        placeholder="Player"
                                                        onChange={(e) => {
                                                            const val = e.target.value;
                                                            setPlayerName(val);
                                                            if (isValidPlayerName(val)) {
                                                                localStorage.setItem("playerName", val);
                                                            }
                                                        }}
                                                    />
                                                </PixelBox>
                                            </div>
                                        </div>
                                    );
                                })()}

                                <button className={styles.btnWrapper} onClick={() => { setView('join-public'); getPublicLobbies() }}>
                                    <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                        Find Public Game
                                    </PixelBox>
                                </button>
                                <button className={styles.btnWrapper} onClick={() => setView('join-code')}>
                                    <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                        Join with Code
                                    </PixelBox>
                                </button>
                                <button className={styles.btnWrapper} onClick={() => setView('create')}>
                                    <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                        Create Game
                                    </PixelBox>
                                </button>
                            </div>
                        )}

                        {view === 'create' && (
                            <form className={styles.menuStack} onSubmit={handleGameCreateForm} noValidate>
                                <div className={styles.formGroup}>
                                    <label>Lobby Type</label>
                                    <div className={styles.inputOuter}>
                                        <PixelBox innerClassName={styles.inputInner} borderColour={boxBorderColour}>
                                            <select
                                                className={styles.formControl}
                                                value={isPrivate ? "private" : "public"}
                                                onChange={(e) => setIsPrivate(e.target.value === "private")}
                                            >
                                                <option value="public">Public</option>
                                                <option value="private">Private</option>
                                            </select>
                                        </PixelBox>
                                    </div>
                                </div>

                                <div className={styles.formGroup}>
                                    <NumberSetting label={"Player Capacity"} range={"2-8"} value={playerCapacity}
                                        min={2} max={8} step={1} onChange={(val) => { if (val) setPlayerCapacity(val) }}
                                        styles={styles} boxBorderColour={boxBorderColour} />
                                </div>

                                <button
                                    type="submit"
                                    className={styles.btnWrapper}
                                    disabled={isCreating}
                                >
                                    <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                        {isCreating ? "Creating..." : "Host Lobby"}
                                    </PixelBox>
                                </button>
                                <button
                                    type="button"
                                    className={styles.btnWrapper}
                                    onClick={back}
                                >
                                    <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                        Back
                                    </PixelBox>
                                </button>
                            </form>
                        )}

                        {view === 'join-public' && (
                            <div className={styles.menuStack}>
                                <div className={styles.roomListOuter}>
                                    <PixelBox innerClassName={styles.roomListInner} borderColour="#505060">
                                        {(publicLobbies.length > 0) ? publicLobbies.map((room) => (
                                            <div key={room.roomId} className={styles.roomItem}>
                                                <div className={styles.roomInfo}>
                                                    <span>Room: {room.roomId} ({room.activeConnections}/{room.maxPlayers})</span>
                                                    <span>Status: {displayRound(room.round)}</span>
                                                </div>
                                                <button
                                                    className={styles.btnWrapper}
                                                    onClick={() => joinGame(room.roomId)}
                                                    disabled={isJoining}
                                                >
                                                    <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                                        Join
                                                    </PixelBox>
                                                </button>
                                            </div>
                                        )) : (
                                            <div className={styles.emptyState}>No public lobbies found</div>
                                        )}
                                    </PixelBox>
                                </div>
                                <button type="button" className={styles.btnWrapper} onClick={back}>
                                    <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                        Back
                                    </PixelBox>
                                </button>
                            </div>
                        )}

                        {view === 'join-code' && (
                            <form className={styles.menuStack} onSubmit={joinGameByCode} noValidate>
                                <div className={styles.formGroup}>
                                    <label>Enter Room Code</label>
                                    <div className={styles.inputOuter}>
                                        <PixelBox innerClassName={styles.inputInner} borderColour={boxBorderColour}>
                                            <input
                                                type="text"
                                                maxLength={3}
                                                className={`${styles.formControl} ${styles.roomCodeInput}`}
                                                value={roomCode}
                                                onChange={(e) => setRoomCode(e.target.value)}
                                            />
                                        </PixelBox>
                                    </div>
                                </div>
                                <button type="submit" className={styles.btnWrapper} disabled={isJoining || roomCode.length !== 3}>
                                    <PixelBox innerClassName={`${styles.btnInner} ${styles.btnPrimary}`} borderColour={boxBorderColour}>
                                        {isJoining ? "Joining..." : "Join Game"}
                                    </PixelBox>
                                </button>
                                <button type="button" className={styles.btnWrapper} onClick={back}>
                                    <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                        Back
                                    </PixelBox>
                                </button>
                            </form>
                        )}
                    </PixelBox>
                </div>
            </div>

            <div className={styles.version}>
                Version 1
            </div>
        </>
    );
}