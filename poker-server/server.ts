import express from 'express';
import { Server, Socket } from 'socket.io';
import { createServer } from 'http';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let GameAPI: any;
let absolutePkgPath = '';

try {
    absolutePkgPath = fs.existsSync(path.join(__dirname, 'pkg/poker_engine.js'))
        ? path.join(__dirname, 'pkg/poker_engine.js')
        : fs.existsSync(path.join(__dirname, '../pkg/poker_engine.js'))
            ? path.join(__dirname, '../pkg/poker_engine.js')
            : fs.existsSync(path.join(__dirname, 'pkg/poker_server.js'))
                ? path.join(__dirname, 'pkg/poker_server.js')
                : path.join(__dirname, '../pkg/poker_server.js');

    const pkgModule = require(absolutePkgPath);
    GameAPI = pkgModule.GameAPI;
} catch (err) { }

type GameAPI = any;

const app = express();
app.set('trust proxy', 1);

const httpServer = createServer(app);
const io = new Server(httpServer, {
    cors: { origin: '*' },
    transports: ['websocket', 'polling']
});

// host frontend
const publicPath = fs.existsSync(path.join(__dirname, 'public'))
    ? path.join(__dirname, 'public')
    : fs.existsSync(path.join(__dirname, '../public'))
        ? path.join(__dirname, '../public')
        : path.join(__dirname, '../poker-client/dist');

console.log(`Serving frontend from: ${publicPath} (exists: ${fs.existsSync(publicPath)})`);

if (fs.existsSync(publicPath)) {
    app.use(express.static(publicPath));
    app.use((req, res) => {
        res.sendFile(path.join(publicPath, 'index.html'));
    });
} else {
    app.get('/', (req, res) => {
        res.status(200).send("Failed to serve frontend.");
    });
}

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
    maxBetMultiplier: number;
}

interface EngineConfig {
    players: ConfigPlayer[];
    rngSeed: number;
    blindSize: number;
    minRaise: number;
    startingChips: number;
    specialCardLimit: number;
    deckType: string;
    maxPlayers: number;
    roundLimit: number;
    maxBetMultiplier: number;
}

interface Player {
    id: number;
    name: string;
    isHost: boolean;
    isBot: boolean;
    botType?: string;
}

interface MoveEvent {
    actorId: number | null;
    private: boolean;
    action: unknown;
}

interface ErrorResponse {
    responseType: "warning" | "error";
    message: string;
}

interface MoveResult {
    events: MoveEvent[],
    gameState: unknown,
}

type ConfigPlayer =
    | { type: "human"; id: number }
    | { type: "bot"; id: number; aiType: AIType };

type AIType = "Risky" | "Safe" | "Smart" | "Random";

interface ChatMessage {
    id: string;
    senderId: number | null;
    senderName: string;
    text: string;
    timestamp: number;
    isSystem?: boolean;
}

interface RoomData {
    game: GameAPI;
    config: GameConfig;
    players: Player[];
    activeConnections: number;
    roomHostSession: string;
    sessionTokens: string[];
    roomId: string;
    chatMessages: ChatMessage[];
    roomCloseTimer?: NodeJS.Timeout | undefined;
}

interface SessionData {
    roomId: string;
    playerId: number;
    socketId: string;
    reconnectTimer?: NodeJS.Timeout | undefined;
    disconnectTimer?: NodeJS.Timeout | undefined;
}

const playerLeaveTimeout = 2 * 60 * 1000;
const earlyLeaveTimeout = 5000;
const roomMap = new Map<string, RoomData>();
const sessionMap = new Map<string, SessionData>();
const socketSessionMap = new Map<string, string>();
const timers = new Map<string, NodeJS.Timeout>();

interface RoomTurnTimer {
    turnPlayerId: number;
    turnSeq: number;
    status: "waiting" | "running";
    safetyTimer?: NodeJS.Timeout | undefined;
    timeoutTimer?: NodeJS.Timeout | undefined;
    deadline?: number | undefined;
    limit: number;
}

const roomTurnTimers = new Map<string, RoomTurnTimer>();
const MAX_ANIMATION_BUDGET_MS = 4000;

function generateSessionToken() {
    return Math.random().toString(36).substring(2, 8) + Date.now().toString(36);
}

const clamp = (value: number, min: number, max: number) => {
    return Math.min(Math.max(value, min), max);
}

const isError = (value: ErrorResponse | MoveResult): value is ErrorResponse => {
    return "responseType" in value
}

/**
 * Calls a provided function after checking a given socket has permission to do so
 */
function withGameContext(
    socket: Socket,
    requireHost: boolean,
    callback: (room: RoomData, session: SessionData, sessionToken: string) => void
) {
    const sessionToken = socketSessionMap.get(socket.id);
    if (!sessionToken) return socket.emit("error", "Socket has no session");

    const session = sessionMap.get(sessionToken);
    if (!session) return socket.emit("error", "Session token has no session");

    const room = roomMap.get(session.roomId);
    if (!room) return socket.emit("error", "No room associated with session");

    if (requireHost && room.roomHostSession !== sessionToken) {
        return socket.emit("warning", "Cannot perform this action as non-host");
    }

    callback(room, session, sessionToken);
}

function sanitizeAndValidatePlayerName(inputName: string): string | null {
    if (typeof inputName !== "string") return null;

    for (let i = 0; i < inputName.length; i++) {
        const code = inputName.charCodeAt(i);
        if (code < 32 || code > 255 || code === 127) {
            return null;
        }
    }

    const sanitized = inputName.trim().replace(/ +/g, ' ');

    if (sanitized.length < 3 || sanitized.length > 12) {
        return null;
    }

    return sanitized;
}

/**
 * Handles modification to connection count and associated cleanup 
 */
function adjustConnections(room: RoomData, amount: number) {
    room.activeConnections += amount;

    if (room.activeConnections <= 0) {
        handleRoomRemoval(room.roomId);
    } else if (room.roomCloseTimer) {
        clearTimeout(room.roomCloseTimer);
        room.roomCloseTimer = undefined;
    }
}

// internal mapping to ensure excessive/unrealistic configurations are rejected
type ConfigDict = { [key: string]: [number, number]; }
const configBounds: ConfigDict = {
    "maxPlayers": [2, 8],
    "turnTimeout": [10, 120],
    "blindSize": [0, 200],
    "minRaise": [1, 200],
    "startingChips": [100, 10000],
    "specialCardLimit": [0, 10],
    "roundLimit": [1, 100],
    "maxBetMultiplier": [5, 55],
}

const clampValue = (value: any, boundName: string) => {
    if (typeof value !== "number" || isNaN(value)) return undefined;
    const bounds = configBounds[boundName];
    if (!bounds) return value;
    return clamp(value, bounds[0], bounds[1]);
}

/**
 * Updates the Rust game config by synchronising it with the existing room config
 */
function updateGameConfig(room: RoomData) {
    if (room.game.get_round() !== "room" && room.game.get_round() !== "gameover") return;

    const blindSize = clampValue(room.config.blindSize, "blindSize") ?? room.config.blindSize;
    const minRaise = clampValue(room.config.minRaise, "minRaise") ?? room.config.minRaise;
    const startingChips = clampValue(room.config.startingChips, "startingChips") ?? room.config.startingChips;
    const rawMaxBetMultiplier = clampValue(room.config.maxBetMultiplier, "maxBetMultiplier") ?? room.config.maxBetMultiplier;
    const maxBetBound = configBounds["maxBetMultiplier"]?.[1] ?? 55;
    const maxBetMultiplier = (rawMaxBetMultiplier >= maxBetBound || rawMaxBetMultiplier === 0) ? 0 : rawMaxBetMultiplier;

    const newConfig: EngineConfig = {
        players: buildPlayerList(room),
        rngSeed: Date.now(),
        blindSize,
        minRaise,
        startingChips,
        specialCardLimit: room.config.specialCardLimit,
        deckType: room.config.deckType,
        maxPlayers: room.config.maxPlayers,
        roundLimit: room.config.roundLimit,
        maxBetMultiplier,
    }

    try {
        const rawResponse = room.game.update_config(JSON.stringify(newConfig));
        if (rawResponse === "") return;
        const response = JSON.parse(rawResponse) as ErrorResponse;
        if (response && isError(response)) console.error("Config update warning:", response.message);
    } catch (e) {
        console.error("Failed to update engine config:", e);
    }
}

function clearRoomTurnTimer(roomId: string) {
    clearTimeout(timers.get(roomId));
    timers.delete(roomId);

    const timer = roomTurnTimers.get(roomId);
    if (timer) {
        if (timer.safetyTimer) clearTimeout(timer.safetyTimer);
        if (timer.timeoutTimer) clearTimeout(timer.timeoutTimer);
        roomTurnTimers.delete(roomId);
    }
}

/**
 * Initiates the turn transition. Starts a safety ceiling timer waiting for clientReady.
 */
function startTurnTransition(roomId: string, game: GameAPI, limit: number) {
    clearRoomTurnTimer(roomId);

    const activeRounds = ["preflop", "flop", "turn", "river"];
    if (!activeRounds.includes(game.get_round())) {
        return;
    }

    const turnPlayerId = game.get_current_turn_player();
    if (turnPlayerId === null || turnPlayerId === undefined) {
        return;
    }

    const prevSeq = roomTurnTimers.get(roomId)?.turnSeq ?? 0;
    const turnSeq = prevSeq + 1;

    const roomTimer: RoomTurnTimer = {
        turnPlayerId,
        turnSeq,
        status: "waiting",
        limit,
    };

    roomTimer.safetyTimer = setTimeout(() => {
        startTurnCountdown(roomId, game, turnSeq);
    }, MAX_ANIMATION_BUDGET_MS);

    roomTurnTimers.set(roomId, roomTimer);
}

/**
 * Starts the authoritative turn countdown once clientReady is received or safety ceiling fires.
 */
function startTurnCountdown(roomId: string, game: GameAPI, turnSeq: number) {
    const timer = roomTurnTimers.get(roomId);
    if (!timer || timer.turnSeq !== turnSeq || timer.status !== "waiting") {
        return;
    }

    const activeRounds = ["preflop", "flop", "turn", "river"];
    if (!activeRounds.includes(game.get_round())) {
        clearRoomTurnTimer(roomId);
        return;
    }

    if (timer.safetyTimer) {
        clearTimeout(timer.safetyTimer);
        timer.safetyTimer = undefined;
    }

    timer.status = "running";
    const deadline = Date.now() + timer.limit;
    timer.deadline = deadline;

    timer.timeoutTimer = setTimeout(() => {
        executeTurnTimeout(roomId, game, turnSeq);
    }, timer.limit);

    io.to(roomId).emit("turnTimerStarted", {
        turnPlayerId: timer.turnPlayerId,
        duration: Math.round(timer.limit / 1000),
        deadline,
    });
}

function executeTurnTimeout(roomId: string, game: GameAPI, turnSeq: number) {
    const timer = roomTurnTimers.get(roomId);
    if (!timer || timer.turnSeq !== turnSeq) {
        return;
    }

    const activeRounds = ["preflop", "flop", "turn", "river"];
    if (!activeRounds.includes(game.get_round())) {
        clearRoomTurnTimer(roomId);
        return;
    }

    let response = JSON.parse(game.player_move(timer.turnPlayerId, "timeout"));

    if (isError(response)) {
        io.to(roomId).emit(
            response.responseType,
            response.message
        );
        clearRoomTurnTimer(roomId);
        return;
    }

    io.to(roomId).emit("moveTimeout", response);

    const room = roomMap.get(roomId);
    if (room) {
        broadcastGameState(room, response.events);
    }

    if (activeRounds.includes(game.get_round())) {
        startTurnTransition(roomId, game, timer.limit);
    } else {
        clearRoomTurnTimer(roomId);
    }
}

/**
 * Removes the current room if there are no more valid socket connections after a reconnect grace period has elapsed
 */
function handleRoomRemoval(roomId: string) {
    const room = roomMap.get(roomId);
    if (!room) return;

    // if no human players are connected to the room, remove it
    if (room.activeConnections <= 0) {
        const closeTime = (room.game.get_round() === "room") ? earlyLeaveTimeout : playerLeaveTimeout;
        room.roomCloseTimer = setTimeout(() => {
            if (room.activeConnections <= 0) {
                roomMap.delete(roomId);
                clearRoomTurnTimer(roomId);
            }
        }, closeTime);
        console.log(`Room ${roomId} flagged for removal in ${(closeTime / 1000).toFixed(1)} seconds`);
    }
}

/**
 * Called after host disconnect
 * Assigns the longest staying player as the new room host
 */
function assignNewHost(roomId: string, room: RoomData) {
    const humanPlayers = room.players.filter(p => !p.isBot);

    if (humanPlayers.length > 0) {
        // get new host, selected as the player who has been in the lobby the longest
        const newHostPlayer = humanPlayers[0];
        if (!newHostPlayer) return;
        // get player session from player
        const sessionEntry = Array.from(sessionMap.entries()).find(
            ([_, s]) => s.roomId === roomId && s.playerId === newHostPlayer.id
        );

        // then update relevant host records to reassign host
        if (sessionEntry) {
            room.roomHostSession = sessionEntry[0];
            room.players.forEach(p => p.isHost = (p.id === newHostPlayer.id));
            io.to(roomId).emit("hostPromote", { playerId: newHostPlayer.id, hostId: newHostPlayer.id });
        }
    }
}

/**
 * Emits a "lobbyUpdate" event, containing updated information about the lobby players or config
 */
function broadcastLobbyUpdate(room: RoomData) {
    io.to(room.roomId).emit("lobbyUpdate", {
        players: room.players,
        config: room.config,
        round: room.game.get_round(),
    });
}

/**
 * Provides all connected room sockets with the current game state
 */
function broadcastGameState(room: RoomData, events: MoveEvent[] = []) {
    for (const token of room.sessionTokens) {
        const session = sessionMap.get(token);
        if (!session) continue;

        try {
            const response = JSON.parse(room.game.get_game_state(session.playerId)) as MoveResult | ErrorResponse;
            if (isError(response)) {
                io.to(session.socketId).emit(
                    response.responseType,
                    response.message
                );
            } else {
                io.to(session.socketId).emit("gameUpdate", {
                    events: viewableEvents(session.playerId, events),
                    gameState: response.gameState
                });
            }
        } catch (e) {
            io.to(room.roomId).emit("error", "An unexpected error occurred while attempting to broadcast game state");
        }
    }
}

function viewableEvents(viewerId: number, events: MoveEvent[] = []): MoveEvent[] {
    return events.filter(
        (event) => !event.private || event.actorId === viewerId,
    );
}

function buildPlayerList(room: RoomData): ConfigPlayer[] {
    return room.players.map((player) => {
        if (player.isBot) {
            return { type: "bot", id: player.id, aiType: player.botType as AIType || "Smart" };
        } else {
            return { type: "human", id: player.id };
        }
    });
}

function getCompleteConfig(config: any): GameConfig | undefined {
    try {
        const parsedConfig = typeof config === "string" ? JSON.parse(config) : config;

        const gameConfig: GameConfig = {
            maxPlayers: parsedConfig.maxPlayers ?? 4,
            blindSize: typeof parsedConfig.blindSize === "number" ? parsedConfig.blindSize : 20,
            minRaise: parsedConfig.minRaise ?? 10,
            startingChips: parsedConfig.startingChips ?? 1000,
            specialCardLimit: typeof parsedConfig.specialCardLimit === "number" ? parsedConfig.specialCardLimit : 3,
            deckType: parsedConfig.deckType ?? "standard",
            turnTimeout: parsedConfig.turnTimeout ?? 40,
            isPrivate: parsedConfig.isPrivate ?? false,
            roundLimit: parsedConfig.roundLimit ?? 30,
            maxBetMultiplier: typeof parsedConfig.maxBetMultiplier === "number" ? parsedConfig.maxBetMultiplier : 15,
        };

        return gameConfig;
    } catch (e) {
        return undefined;
    }
}

io.on("connection", (socket: Socket) => {
    socket.on("disconnect", () => {
        withGameContext(socket, false, (room, session, sessionToken) => {
            adjustConnections(room, -1);

            // replaces player with bot and notifies room on player disconnect
            session.disconnectTimer = setTimeout(() => {
                try {
                    const currentRound = room.game.get_round();

                    const discPlayer = room.players.find(p => p.id === session.playerId);
                    const discPlayerName = discPlayer?.name || (session.playerId !== undefined ? `Player ${session.playerId}` : "A player");

                    if (currentRound !== "room" && currentRound !== "preround") {
                        let response = JSON.parse(room.game.toggle_id_with_bot(session.playerId, true));
                        if (isError(response)) {
                            return socket.emit(
                                response.responseType,
                                response.message
                            );

                        }
                        broadcastGameState(room, response.events);
                    } else {
                        room.players = room.players.filter(p => p.id !== session.playerId);
                        updateGameConfig(room);
                    }

                    room.sessionTokens = room.sessionTokens.filter((token) => token !== sessionToken);

                    socket.to(session.roomId).emit("info", `${discPlayerName} disconnected`);
                    broadcastLobbyUpdate(room);

                    if (room.roomHostSession === sessionToken) assignNewHost(session.roomId, room);

                    const closeTime = (currentRound === "room") ? earlyLeaveTimeout : playerLeaveTimeout;
                    setTimeout(() => {
                        room.game.queue_remove_player(session.playerId);
                        sessionMap.delete(sessionToken);
                    }, closeTime);

                } catch (e) {
                    io.to(room.roomId).emit("error", "An unexpected error occurred while attempting to toggle player with bot")
                    return;
                }
                // disconnect buffered to ensure minor connection drops do not cause disconnect
            }, 3000);

            socketSessionMap.delete(socket.id);
        });
    });

    socket.on("createGame", (configStr: string) => {
        const parsedConfig = getCompleteConfig(configStr);
        if (!parsedConfig) return socket.emit("error", "Failed to create game - could not parse config");
        const roomId = Math.random().toString(36).substring(2, 5);
        const sessionToken = generateSessionToken();

        let initialName = "Player 0";
        try {
            const rawObj = JSON.parse(configStr);
            if (rawObj && typeof rawObj.playerName === "string") {
                const validated = sanitizeAndValidatePlayerName(rawObj.playerName);
                if (validated) initialName = validated;
            }
        } catch (e) { }

        const hostPlayer: Player = {
            id: 0,
            name: initialName,
            isHost: true,
            isBot: false
        };

        const room: RoomData = {
            game: new GameAPI(),
            config: parsedConfig,
            players: [hostPlayer],
            activeConnections: 1,
            roomHostSession: sessionToken,
            sessionTokens: [sessionToken],
            roomId,
            chatMessages: [],
        };

        try {
            room.game.init_panic_hook();
            updateGameConfig(room);

            roomMap.set(roomId, room);
            socketSessionMap.set(socket.id, sessionToken);
            sessionMap.set(sessionToken, { roomId, playerId: 0, socketId: socket.id });

            socket.join(roomId);
            socket.emit("gameCreated", { roomId, sessionToken });
            broadcastLobbyUpdate(room);
            console.log(`Created game, id: ${roomId}`);
        } catch (error) {
            roomMap.delete(roomId);
            socketSessionMap.delete(socket.id);
            sessionMap.delete(sessionToken);

            console.error("Game creation configuration failed", { roomId, error, parsedConfig });

            socket.emit("error", "Failed to create game due to bad configuration");
        }
    });

    socket.on("addBot", (data: { roomId: string, botType: AIType }) => {
        withGameContext(socket, true, (room) => {
            if (room.players.length >= room.config.maxPlayers) {
                return socket.emit("warning", "Cannot add bot as lobby is already full");
            } else if (!data.botType) {
                return socket.emit("error", "Invalid bot");
            }

            const newId = room.players.length > 0 ? Math.max(...room.players.map(p => p.id)) + 1 : 0;
            room.players.push({
                id: newId,
                name: `${data.botType} Bot`,
                isHost: false,
                isBot: true,
                botType: data.botType
            });

            try {
                updateGameConfig(room);
                socket.emit("info", `Added new ${data.botType} bot to lobby`);
                broadcastLobbyUpdate(room);
            } catch (e) {
                // ensures failed adds always roll back - realigns server state with game state
                room.players.pop();
                socket.emit("error", "Failed to add bot to game");
            }
        });
    });

    socket.on("updateConfig", (data: { roomId: string, config: any }) => {
        withGameContext(socket, true, (room) => {
            const gameConfig = getCompleteConfig(data.config);
            if (!gameConfig) return socket.emit("error", "Configuration could not be parsed");

            room.config.maxPlayers = clampValue(gameConfig.maxPlayers, "maxPlayers") ?? room.config.maxPlayers;
            room.config.isPrivate = gameConfig.isPrivate ?? true;
            room.config.turnTimeout = clampValue(gameConfig.turnTimeout, "turnTimeout") ?? 30;
            room.config.blindSize = clampValue(gameConfig.blindSize, "blindSize") ?? room.config.blindSize;
            room.config.minRaise = clampValue(gameConfig.minRaise, "minRaise") ?? room.config.minRaise;
            room.config.startingChips = clampValue(gameConfig.startingChips, "startingChips") ?? room.config.startingChips;
            room.config.deckType = gameConfig.deckType ?? room.config.deckType ?? "standard";
            room.config.specialCardLimit = clampValue(gameConfig.specialCardLimit, "specialCardLimit") ?? room.config.specialCardLimit;
            room.config.roundLimit = clampValue(gameConfig.roundLimit, "roundLimit") ?? room.config.roundLimit;
            room.config.maxBetMultiplier = clampValue(gameConfig.maxBetMultiplier, "maxBetMultiplier") ?? room.config.maxBetMultiplier;

            updateGameConfig(room);
            broadcastLobbyUpdate(room);
        });
    });

    socket.on("kickPlayer", (data: { roomId: string, playerId: string }) => {
        // host only
        withGameContext(socket, true, (room) => {
            const pid = parseInt(data.playerId);
            const kickedPlayer = room.players.find(p => p.id === pid);
            if (!kickedPlayer) return;

            room.players = room.players.filter(p => p.id !== pid);
            // instantly removes target player
            room.game.force_remove_player(pid);

            // finds target player session
            const targetEntry = Array.from(sessionMap.entries()).find(
                ([_, s]) => s.roomId === room.roomId && s.playerId === pid
            );

            // if a player session exists, remove it from game data
            if (targetEntry) {
                const [targetSessionToken, targetSession] = targetEntry;

                room.sessionTokens = room.sessionTokens.filter(t => t !== targetSessionToken);
                adjustConnections(room, -1);

                const targetSocket = io.sockets.sockets.get(targetSession.socketId);
                if (targetSocket) {
                    targetSocket.leave(room.roomId);
                    socketSessionMap.delete(targetSocket.id);
                }
                sessionMap.delete(targetSessionToken);

                io.to(targetSession.socketId).emit("kicked");
                io.to(room.roomId).emit("info", `${kickedPlayer.name} was kicked from the game`);
            } else {
                io.to(room.roomId).emit("info", `${kickedPlayer.name} was removed`);
            }

            updateGameConfig(room);
            broadcastLobbyUpdate(room);
        });
    });

    socket.on("promoteHost", (data: { roomId: string, playerId: string }) => {
        withGameContext(socket, true, (room) => {
            const pid = parseInt(data.playerId);
            const targetEntry = Array.from(sessionMap.entries()).find(
                ([_, s]) => s.roomId === room.roomId && s.playerId === pid
            );

            if (targetEntry) {
                room.roomHostSession = targetEntry[0];
                room.players.forEach(p => p.isHost = (p.id === pid));

                broadcastLobbyUpdate(room);
                io.to(room.roomId).emit("hostPromote", { playerId: pid });
            } else {
                socket.emit("error", "Target player could not be given host");
            }
        });
    });

    socket.on("sendChatMessage", (text: string) => {
        withGameContext(socket, false, (room, session) => {
            if (typeof text !== "string" || !text.trim()) return;
            const cleanedText = text.trim().substring(0, 250);
            const senderPlayer = room.players.find(p => p.id === session.playerId);
            const senderName = senderPlayer ? senderPlayer.name : `Player ${session.playerId}`;

            const msg: ChatMessage = {
                id: `msg-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
                senderId: session.playerId,
                senderName,
                text: cleanedText,
                timestamp: Date.now(),
                isSystem: false,
            };

            if (!room.chatMessages) room.chatMessages = [];
            room.chatMessages.push(msg);
            if (room.chatMessages.length > 100) room.chatMessages.shift();

            io.to(room.roomId).emit("chatMessage", msg);
        });
    });

    socket.on("getChatHistory", () => {
        withGameContext(socket, false, (room) => {
            socket.emit("chatHistory", room.chatMessages || []);
        });
    });

    socket.on("updatePlayerName", (newName: string) => {
        withGameContext(socket, false, (room, session) => {
            const validated = sanitizeAndValidatePlayerName(newName);
            if (!validated) {
                return socket.emit("error", "Name must be 3-12 extended ASCII characters");
            }
            const player = room.players.find(p => p.id === session.playerId);
            if (player) {
                player.name = validated;
                broadcastLobbyUpdate(room);
            }
        });
    });

    socket.on("joinGame", (data: string | { roomId: string, playerName?: string }) => {
        const roomId = typeof data === "string" ? data : data?.roomId;
        const requestedName = typeof data === "object" && typeof data.playerName === "string"
            ? sanitizeAndValidatePlayerName(data.playerName) || undefined
            : undefined;

        const room = roomMap.get(roomId);
        if (!room) return socket.emit("error", "Room does not exist");

        const humanCount = room.players.filter(p => !p.isBot).length;
        if (humanCount >= room.config.maxPlayers) return socket.emit("error", "Lobby is full");

        if (room.players.length >= room.config.maxPlayers) {
            const replacedIndex = room.players.map(p => p.isBot).lastIndexOf(true);
            if (replacedIndex !== -1) {
                const replacedBot = room.players[replacedIndex];
                if (!replacedBot) return;

                room.players.splice(replacedIndex, 1);
                try {
                    const oldPlayers = room.players;
                    const oldActiveConnections = room.activeConnections;

                    try {
                        const replacedIndex = room.players.map((p) => p.isBot).lastIndexOf(true);

                        if (replacedIndex !== -1) room.players = room.players.filter((_, index) => index !== replacedIndex);
                        const newId = room.players.length > 0 ? Math.max(...room.players.map((p) => p.id)) + 1 : 0;

                        room.players = [
                            ...room.players,
                            {
                                id: newId,
                                name: `Player ${newId}`,
                                isHost: humanCount === 0,
                                isBot: false,
                            },
                        ];

                        updateGameConfig(room);
                        adjustConnections(room, 1);

                        const sessionToken = generateSessionToken();
                        socketSessionMap.set(socket.id, sessionToken);
                        sessionMap.set(sessionToken, { roomId, playerId: newId, socketId: socket.id });

                        room.sessionTokens.push(sessionToken);
                        socket.join(roomId);
                        socket.emit("joinSuccess", { roomId, sessionToken });
                        broadcastLobbyUpdate(room);
                    } catch (error) {
                        room.players = oldPlayers;
                        room.activeConnections = oldActiveConnections;

                        try {
                            updateGameConfig(room);
                        } catch (restoreError) {
                            console.error("Failed to restore game after join failure", restoreError);
                        }

                        console.error("Join configuration failed", { roomId, error });
                        socket.emit("error", "Failed to communicate join with server");
                    }
                } catch (e) { }
            }
        }

        adjustConnections(room, 1);

        const newId = room.players.length > 0 ? Math.max(...room.players.map(p => p.id)) + 1 : 0;
        room.players.push({
            id: newId,
            name: requestedName || `Player ${newId}`,
            isHost: humanCount === 0,
            isBot: false
        });

        try {
            updateGameConfig(room);
        } catch (e) {
            adjustConnections(room, -1);
            room.players.pop();
            return socket.emit("error", "Failed to communicate join with server");
        }

        const sessionToken = generateSessionToken();
        socketSessionMap.set(socket.id, sessionToken);
        sessionMap.set(sessionToken, {
            roomId,
            playerId: newId,
            socketId: socket.id
        });
        room.sessionTokens.push(sessionToken);

        // edge case logic for rejoining an empty lobby
        if (humanCount === 0) room.roomHostSession = sessionToken;

        socket.emit("joinSuccess", { roomId, sessionToken });
        socket.join(roomId);

        const joinedPlayer = room.players.find(p => p.id === newId);
        const joinedPlayerName = joinedPlayer?.name || `Player ${newId}`;
        socket.to(roomId).emit("info", `${joinedPlayerName} joined the game`);

        broadcastLobbyUpdate(room);
    });

    socket.on("leaveGame", () => {
        const sessionToken = socketSessionMap.get(socket.id);
        if (!sessionToken) return;

        const session = sessionMap.get(sessionToken);
        if (!session) return;

        const room = roomMap.get(session.roomId);
        if (!room) return;

        adjustConnections(room, -1);

        const leavingPlayer = room.players.find(p => p.id === session.playerId);
        const leavingPlayerName = leavingPlayer?.name || (session.playerId !== undefined ? `Player ${session.playerId}` : "A player");

        room.sessionTokens = room.sessionTokens.filter((token) => token !== sessionToken);
        room.players = room.players.filter(p => p.id !== session.playerId);

        room.game.queue_remove_player(session.playerId);
        updateGameConfig(room);

        socket.leave(session.roomId);
        sessionMap.delete(sessionToken);
        socketSessionMap.delete(socket.id);

        if (room.roomHostSession === sessionToken) assignNewHost(session.roomId, room);

        socket.to(session.roomId).emit("info", `${leavingPlayerName} left the game`);
        try {
            broadcastGameState(room);
        } catch (e) { }

        broadcastLobbyUpdate(room);
    });

    socket.on("requestLobbyInfo", () => {
        const sessionToken = socketSessionMap.get(socket.id);
        if (!sessionToken) return;
        const session = sessionMap.get(sessionToken);
        if (!session) return;
        const room = roomMap.get(session.roomId);
        if (!room) return;

        socket.emit("lobbyUpdate", { players: room.players, config: room.config, round: room.game.get_round() });
        socket.emit("clientInfo", { playerId: session.playerId });
    });

    socket.on("requestGameState", () => {
        withGameContext(socket, false, (room, session) => {
            socket.emit("clientInfo", { playerId: session.playerId });

            socket.emit("lobbyUpdate", {
                players: room.players,
                config: room.config,
                round: room.game.get_round()
            });
            try {
                const playerState = JSON.parse(room.game.get_game_state(session.playerId)) as MoveResult | ErrorResponse;

                if (isError(playerState)) {
                    socket.emit(playerState.responseType, playerState.message);
                    return;
                }

                socket.emit("gameUpdate", playerState);

                const timer = roomTurnTimers.get(session.roomId);
                if (timer && timer.status === "running" && timer.deadline && timer.deadline > Date.now()) {
                    socket.emit("turnTimerStarted", {
                        turnPlayerId: timer.turnPlayerId,
                        duration: Math.max(0, Math.ceil((timer.deadline - Date.now()) / 1000)),
                        deadline: timer.deadline,
                    });
                }
            } catch (e) { }
        });
    });

    socket.on("reconnectGame", (sessionToken: string) => {
        const session = sessionMap.get(sessionToken);
        if (!session) return;
        const room = roomMap.get(session.roomId);
        if (!room) return;

        // if there is an existing reconnect timer on the session, clear it
        if (session.reconnectTimer) {
            clearTimeout(session.reconnectTimer);
            session.reconnectTimer = undefined;
        }

        // and allow the player to rejoin the room
        adjustConnections(room, 1);

        session.socketId = socket.id;
        socketSessionMap.set(socket.id, sessionToken);
        socket.join(session.roomId);

        if (session.disconnectTimer) {
            clearTimeout(session.disconnectTimer);
            session.disconnectTimer = undefined;
            room.sessionTokens.push(sessionToken);
        } else {
            try {
                let response = JSON.parse(room.game.toggle_id_with_bot(session.playerId, false)) as MoveResult | ErrorResponse;

                if (isError(response)) {
                    socket.emit(response.responseType, response.message);
                    return;
                }

                broadcastGameState(room, response.events);
            } catch (e) {
                socket.emit("error", "An unexpected error occurred while attempting to reconnect to the previous game session");
            }
        }

        socket.emit("playerReconnect", {
            roomId: session.roomId,
            playerId: session.playerId,
            state: JSON.parse(room.game.get_game_state(session.playerId)),
        });
        broadcastLobbyUpdate(room);
    });

    socket.on("initialiseGame", () => {
        withGameContext(socket, true, (room, session) => {
            try {
                // lobby -> preround
                const initialiseResult = JSON.parse(room.game.initialise());
                if (isError(initialiseResult)) {
                    return io.to(session.socketId).emit(
                        initialiseResult.responseType,
                        initialiseResult.message
                    );
                }

                io.to(session.roomId).emit("gameStart", initialiseResult);
                broadcastGameState(room, initialiseResult.events);
            } catch (e) {
                io.to(room.roomId).emit("error", "An unexpected error occurred while attempting to initialise the room");
            }
        });
    });

    socket.on("getGameConfig", () => {
        withGameContext(socket, false, (room) => {
            socket.emit("gameConfig", room.config);
        });
    });

    socket.on("startGame", () => {
        withGameContext(socket, true, (room, session) => {
            try {
                const currentRound = room.game.get_round();
                if (currentRound === "room" || currentRound === "gameover") {
                    const initResult = JSON.parse(room.game.initialise());
                    if (isError(initResult)) return socket.emit(initResult.responseType, initResult.message);
                }

                // preround -> preflop
                const startResult = JSON.parse(room.game.start());
                if (isError(startResult)) return socket.emit(startResult.responseType, startResult.message);

                io.to(session.roomId).emit("gameStarted");
                broadcastGameState(room, startResult.events);
                broadcastLobbyUpdate(room);
                startTurnTransition(session.roomId, room.game, room.config.turnTimeout * 1000);
            } catch (e) {
                io.to(room.roomId).emit("An unexpected error occurred while attempting to start the game");
            }
        });
    });

    socket.on("playerMove", (moveData: string) => {
        withGameContext(socket, false, (room, session) => {
            try {
                const moveResult = JSON.parse(room.game.player_move(
                    session.playerId,
                    moveData
                )) as MoveResult | ErrorResponse;

                if (isError(moveResult)) return socket.emit(moveResult.responseType, moveResult.message);

                broadcastGameState(room, moveResult.events);
                startTurnTransition(
                    room.roomId,
                    room.game,
                    room.config.turnTimeout * 1000
                );
            } catch (err) {
                socket.emit("error", "Invalid move");
            }
        });
    });

    socket.on("clientReady", () => {
        withGameContext(socket, false, (room, session) => {
            const timer = roomTurnTimers.get(session.roomId);
            if (!timer) return;
            if (timer.status !== "waiting") return;
            if (timer.turnPlayerId !== session.playerId) return;
            startTurnCountdown(session.roomId, room.game, timer.turnSeq);
        });
    });

    socket.on("getPublicLobbies", () => {
        const joinableRounds = ["room", "preround"];

        // shows all lobbies which are public, in a joinable round, and not full
        const publicLobbies = Array.from(roomMap.values())
            .filter((room) => !room.config.isPrivate && joinableRounds.includes(room.game.get_round()) && room.activeConnections < room.config.maxPlayers && room.activeConnections > 0)
            .map((room) => ({
                roomId: room.roomId,
                activeConnections: room.activeConnections,
                maxPlayers: room.config.maxPlayers,
                round: room.game.get_round()
            }));

        socket.emit("publicLobbies", publicLobbies);
    });
});

const PORT = Number(process.env.PORT || 3000);
const HOST = process.env.HOST || '0.0.0.0';

httpServer.listen(PORT, HOST, () => {
    console.log(`Poker started on ${HOST}:${PORT}`);
});