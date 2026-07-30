import express from 'express';
import { Server, Socket } from 'socket.io';
import { createServer } from 'http';

import { Game } from './pkg/poker_server.js';

const app = express();
const httpServer = createServer(app);
const io = new Server(httpServer, { cors: { origin: '*'}});

interface LobbyConfig {
    maxPlayers: number;
    blindSize: number;
    minRaise: number;
    startingChips: number;
    deckType: string;
    turnTimeout: number;
    isPrivate: boolean;
}

interface Player {
    id: number;
    name: string;
    isHost: boolean;
    isBot: boolean;
    botType?: string;
}

interface RoomData {
    game: Game;
    config: LobbyConfig;
    players: Player[];
    activeConnections: number;
    roomHostSession: string;
    sessionTokens: string[];
    roomId: string;
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

function generateSessionToken() {
    return Math.random().toString(36).substring(2, 8) + Date.now().toString(36);
}

const clamp = (value: number, min: number, max: number) => {
    return Math.min(Math.max(value, min), max);
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
    if (room.game.get_round() !== "room") return;

    const bots = room.players.filter(p => p.isBot).map(p => p.botType!);
    const humanCount = room.players.filter(p => !p.isBot).length;

    try {
        const blind_size = clampValue(room.config.blindSize, "blindSize") ?? room.config.blindSize;
        const min_raise = clampValue(room.config.minRaise, "minRaise") ?? room.config.minRaise;
        const starting_chips = clampValue(room.config.startingChips, "startingChips") ?? room.config.startingChips;

        const args = JSON.stringify({
            player_count: humanCount,
            bots: bots,
            rng_seed: Date.now(), 
            blind_size,
            min_raise,
            starting_chips,
            deck_type: room.config.deckType,
            max_players: room.config.maxPlayers
        });

        return room.game.update_config(args);
    } catch (e) {
        io.to(room.roomId).emit("error", "An unexpected error occurred while attempting to update game configuration");
        return;
    }
}

/**
 * Forces the end of the next turn if the player does not make a move within the specified time limit
 * TODO: send time limit in socket event so client is able to see when turns expire
 */
function turnTimeout(roomId: string, game: Game, limit: number) {
    clearTimeout(timers.get(roomId));

    const activeRounds = ["preflop", "flop", "turn", "river"];
    if (!activeRounds.includes(game.get_round())) {
        timers.delete(roomId);
        return;
    }

    // forces the current player to send a "timeout" move after the time limit expires
    const timerId = setTimeout(() => {
        if (!activeRounds.includes(game.get_round())) {
            timers.delete(roomId);
            return;
        }

        let response = game.player_move(game.get_current_turn_player(), "timeout", 0);
        let parsedResponse = JSON.parse(response);

        io.to(roomId).emit("moveTimeout", parsedResponse);

        const room = roomMap.get(roomId);
        if (room) broadcastGameState(room, parsedResponse.events || []);

        if (!parsedResponse.error && activeRounds.includes(game.get_round())) {
            turnTimeout(roomId, game, limit);
        } else {
            timers.delete(roomId);
        }
    }, limit);

    timers.set(roomId, timerId);
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
                clearTimeout(timers.get(roomId));
                timers.delete(roomId);
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
            io.to(roomId).emit("hostPromote", { hostId: newHostPlayer.id });
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
function broadcastGameState(room: RoomData, events: any[] = []) {
    for (const token of room.sessionTokens) {
        const session = sessionMap.get(token);
        if (session) {
            try {
                const playerState = JSON.parse(room.game.get_game_state(session.playerId));
                io.to(session.socketId).emit("gameUpdate", {
                    events,
                    game_state: playerState
                });
            } catch (e) {
                io.to(room.roomId).emit("error", "An unexpected error occurred while attempting to broadcast game state");
            }
        }
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
                    
                    if (currentRound !== "room" && currentRound !== "preround") {
                        let response = JSON.parse(room.game.toggle_id_with_bot(session.playerId, true));
                        if (!response.error) {
                            broadcastGameState(room, response.events || []);
                        }
                    } else {
                        room.players = room.players.filter(p => p.id !== session.playerId);
                        updateGameConfig(room);
                    }

                    room.sessionTokens = room.sessionTokens.filter((token) => token !== sessionToken);
                    
                    socket.to(session.roomId).emit("info", `Player ${session.playerId} disconnected`);
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
        try {
            const parsedConfig = JSON.parse(configStr);
            const roomId = Math.random().toString(36).substring(2, 5);
            const sessionToken = generateSessionToken();

            const hostPlayer: Player = {
                id: 0,
                name: "Player 0",
                isHost: true,
                isBot: false
            };

            const room: RoomData = {
                game: new Game(),
                config: {
                    maxPlayers: parsedConfig.maxPlayers || 4,
                    blindSize: parsedConfig.blindSize || 20,
                    minRaise: parsedConfig.minRaise || 10,
                    startingChips: parsedConfig.startingChips || 1000,
                    deckType: parsedConfig.deckType || "standard",
                    turnTimeout: parsedConfig.turnTimeout || 40,
                    isPrivate: parsedConfig.isPrivate || false
                },
                players: [hostPlayer],
                activeConnections: 1,
                roomHostSession: sessionToken,
                sessionTokens: [sessionToken],
                roomId
            };

            try {
                room.game.init_panic_hook();
            } catch (e) {}

            roomMap.set(roomId, room);
            socketSessionMap.set(socket.id, sessionToken);
            sessionMap.set(sessionToken, { roomId, playerId: 0, socketId: socket.id });

            updateGameConfig(room);

            socket.join(roomId);
            socket.emit("gameCreated", { roomId });
            broadcastLobbyUpdate(room);

            console.log(`Created game, id: ${roomId}`);
        } catch (e) {
            socket.emit("error", "Failed to create game due to bad configuration");
        }
    });

    socket.on("addBot", (data: { roomId: string, botType: string }) => {
        withGameContext(socket, true, (room) => {
            if (room.players.length >= room.config.maxPlayers) {
                return socket.emit("warning", "Cannot add bot as lobby is already full");
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
            try {
                room.config.maxPlayers = clampValue(data.config.maxPlayers, "maxPlayers") ?? room.config.maxPlayers;
                room.config.isPrivate = data.config.isPrivate ?? room.config.isPrivate;
                room.config.turnTimeout = clampValue(data.config.turnTimeout, "turnTimeout") ?? room.config.turnTimeout;
                room.config.blindSize = clampValue(data.config.blindSize, "blindSize") ?? room.config.blindSize;
                room.config.minRaise = clampValue(data.config.minRaise, "minRaise") ?? room.config.minRaise;
                room.config.startingChips = clampValue(data.config.startingChips, "startingChips") ?? room.config.startingChips;
                room.config.deckType = data.config.deckType || room.config.deckType;

                updateGameConfig(room);
                broadcastLobbyUpdate(room);
            } catch (e) {
                socket.emit("error", "Configuration could not be parsed");
            }
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

    socket.on("joinGame", (roomId: string) => {
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
                    room.game.force_remove_player(replacedBot.id);
                } catch (e) {}
            }
        }

        adjustConnections(room, 1);
        
        const newId = room.players.length > 0 ? Math.max(...room.players.map(p => p.id)) + 1 : 0;
        // todo: allow players to choose names
        room.players.push({
            id: newId,
            name: `Player ${newId}`,
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
        sessionMap.set(sessionToken, { roomId, playerId: newId, socketId: socket.id });
        room.sessionTokens.push(sessionToken);

        // edge case logic for rejoining an empty lobby
        if (humanCount === 0) room.roomHostSession = sessionToken;

        socket.emit("joinSuccess", {roomId, sessionToken});
        socket.join(roomId);

        socket.to(roomId).emit("info", `Player ${newId} joined the game`);

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
        
        room.sessionTokens = room.sessionTokens.filter((token) => token !== sessionToken);
        room.players = room.players.filter(p => p.id !== session.playerId);
        
        room.game.queue_remove_player(session.playerId);
        updateGameConfig(room);

        socket.leave(session.roomId);
        sessionMap.delete(sessionToken);
        socketSessionMap.delete(socket.id);

        if (room.roomHostSession === sessionToken) assignNewHost(session.roomId, room);

        socket.to(session.roomId).emit("info", `Player ${session.playerId} left the game`);
        try {
            broadcastGameState(room);
        } catch (e) {}

        broadcastLobbyUpdate(room);
    });

    socket.on("requestLobbyInfo", () => {
        const sessionToken = socketSessionMap.get(socket.id);
        if (!sessionToken) return;
        const session = sessionMap.get(sessionToken);
        if (!session) return;
        const room = roomMap.get(session.roomId);
        if (!room) return;

        socket.emit("lobbyUpdate", { players: room.players, config: room.config });
        socket.emit("clientInfo", { playerId: session.playerId });
    });

    socket.on("requestGameState", () => {
        withGameContext(socket, false, (room, session) => {
            socket.emit("clientInfo", { playerId: session.playerId });
            socket.emit("lobbyUpdate", { players: room.players, config: room.config, round: room.game.get_round() });
            try {
                const playerState = JSON.parse(room.game.get_game_state(session.playerId));
                socket.emit("gameUpdate", playerState);
            } catch (e) {}
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
                let response = room.game.toggle_id_with_bot(session.playerId, false);
                io.to(session.roomId).emit("gameUpdate", response);
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
                if (initialiseResult.error) return socket.emit("error", initialiseResult.error);

                io.to(session.roomId).emit("gameStart", initialiseResult);
                broadcastGameState(room, initialiseResult.events || []);
                turnTimeout(session.roomId, room.game, room.config.turnTimeout * 1000);
            } catch (e) {
                io.to(room.roomId).emit("error", "An unexpected error occurred while attempting to initialise the room");
            }
        });
    });

    socket.on("startGame", () => {
        withGameContext(socket, true, (room, session) => {
            try {
                // preround -> preflop
                const startResult = JSON.parse(room.game.start());
                if (startResult.error) return socket.emit("error", startResult.error);

                io.to(session.roomId).emit("gameStarted");
                broadcastGameState(room, startResult.events || []);
                broadcastLobbyUpdate(room);
                turnTimeout(session.roomId, room.game, room.config.turnTimeout * 1000);
            } catch (e) {
                io.to(room.roomId).emit("An unexpected error occurred while attempting to start the game");
            }
        });
    });

    socket.on("playerMove", (moveData: { action: string; amount: number }) => {
        withGameContext(socket, false, (room, session) => {
            try {
                const moveResultStr = room.game.player_move(session.playerId, moveData.action, moveData.amount);
                const moveResult = JSON.parse(moveResultStr);

                if (moveResult.error) {
                    return socket.emit("error", moveResult.error);
                }

                broadcastGameState(room, moveResult.events || []);
                turnTimeout(room.roomId, room.game, room.config.turnTimeout * 1000);
            } catch (err) {
                console.error("Move error:", err);
                socket.emit("error", "Invalid move");
            }
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

const PORT = 3000;
httpServer.listen(PORT, () => {
    console.log(`Poker started on port ${PORT}`);
});