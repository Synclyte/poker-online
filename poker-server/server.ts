import express from 'express';
import { Server, Socket } from 'socket.io';
import { createServer } from 'http';

import { Game } from './pkg/poker_server.js';

const app = express();
const httpServer = createServer(app);
const io = new Server(httpServer, { cors: { origin: '*'}});

interface RoomData {
    game: Game,
    turnTimeout: number,
    activeConnections: number,
    roomHostId: number,
    roomCloseTimer?: NodeJS.Timeout | undefined,
}

interface SessionData {
    roomId: string,
    playerId: number,
    socketId: string,
    reconnectTimer?: NodeJS.Timeout | undefined,
    disconnectTimer?: NodeJS.Timeout | undefined,
}

const playerLeaveTimeout = 3 * 60 * 1000;
const roomMap = new Map<string, RoomData>();
const sessionMap = new Map<string, SessionData>();
const socketSessionMap = new Map<string, string>();
const timers = new Map<string, NodeJS.Timeout>();

function generateSessionToken() {
    return Math.random().toString(36).substring(2, 8) + Date.now().toString(36);
}

function turnTimeout(roomId: string, game: Game, limit: number) {
    clearTimeout(timers.get(roomId));

    const timerId = setTimeout(() => {
        let response = game.player_move(game.get_current_turn_player(), "timeout", 0);
        let parsedResponse = JSON.parse(response);

        io.to(roomId).emit("moveTimeout", parsedResponse);

        if (!parsedResponse.error) turnTimeout(roomId, game, limit); 
    }, limit);

    timers.set(roomId, timerId);
}

function handleRoomRemoval(roomId: string) {
    const room = roomMap.get(roomId);
    if (!room) return;

    if (room.activeConnections <= 0) {
        room.roomCloseTimer = setTimeout(() => {
            if (room.activeConnections <= 0) {
                roomMap.delete(roomId);
                clearTimeout(timers.get(roomId));
                timers.delete(roomId);
            }
        }, playerLeaveTimeout);
    }
}

io.on("connection", (socket: Socket) => {
    socket.on("disconnect", () => {
        const sessionToken = socketSessionMap.get(socket.id);
        if (!sessionToken) return;

        const session = sessionMap.get(sessionToken);
        if (!session) return;

        const room = roomMap.get(session.roomId);
        if (!room) return;

        room.activeConnections -= 1;
        handleRoomRemoval(session.roomId);

        session.disconnectTimer = setTimeout(() => {
            let response: JSON = JSON.parse(room.game.toggle_id_with_bot(session.playerId, true));
            if (!("error" in response)) {
                io.to(session.roomId).emit("gameUpdate", response);
            }

            io.to(session.roomId).emit("playerDisconnect", { playerId: session.playerId });

            setTimeout(() => {
                room.game.queue_remove_player(session.playerId);
                sessionMap.delete(sessionToken);
            }, 3 * 60 * 1000);
        }, 3000)

        socketSessionMap.delete(socket.id);
    });

    socket.on("createGame", (configStr: string) => {
        try {
            const parsedConfig = JSON.parse(configStr);
            const turnTimeout = parsedConfig.turnTimeout || 40_000;

            const defaultArgs: string = JSON.stringify({
                player_count: 1,
                bots: parsedConfig.bots || [],
                rng_seed: Date.now(),
                blind_size: parsedConfig.blind_size || 20,
                min_raise: parsedConfig.min_raise || 10,
                starting_chips: parsedConfig.starting_chips || 1000,
                deck_type: parsedConfig.deck_type || "standard",
                max_players: parsedConfig.max_players || 4
            });

            let game = new Game(defaultArgs);
            let roomId = Math.random().toString(36).substring(2, 5);
            while (roomMap.has(roomId)) {
                roomId = Math.random().toString(36).substring(2, 5);
            }

            roomMap.set(roomId, {
                game,
                turnTimeout,
                activeConnections: 1,
                roomHostId: 0,
            });

            const sessionToken = generateSessionToken();
            socketSessionMap.set(socket.id, sessionToken);
            sessionMap.set(sessionToken, { roomId, playerId: 0, socketId: socket.id });

            socket.join(roomId);
            socket.emit("gameCreated", { roomId });
        } catch (e) {
            socket.emit("error", "Could not create game - bad configuration");
        }
    });

    socket.on("joinGame", (roomId: string) => {
        const room = roomMap.get(roomId);
        if (!room) {
            return socket.emit("error", "Room does not exist");
        }

        const response = JSON.parse(room.game.try_add_player("human"));
        if (response.error) {
            console.log(response.error);
            return;
        }

        room.activeConnections += 1;
        if (room.roomCloseTimer) {
            clearTimeout(room.roomCloseTimer);
            room.roomCloseTimer = undefined;
        }
        
        const sessionToken = generateSessionToken();
        socketSessionMap.set(socket.id, sessionToken);
        sessionMap.set(sessionToken, { roomId, playerId: +response.id, socketId: socket.id });
        
        socket.join(roomId);
        io.to(roomId).emit("playerJoin", { playerId: +response.id, state: JSON.parse(room.game.get_game_state(1e6))});
    });

    socket.on("reconnectGame", (sessionToken: string) => {
        const session = sessionMap.get(sessionToken);
        if (!session) return socket.emit("error", "Session does not exist");

        const room = roomMap.get(session.roomId);
        if (!room) return socket.emit("error", "Room associated with session does not exist");

        if (session.reconnectTimer) {
            clearTimeout(session.reconnectTimer);
            session.reconnectTimer = undefined;
        }

        room.activeConnections += 1;
        if (room.roomCloseTimer) {
            clearTimeout(room.roomCloseTimer);
            room.roomCloseTimer = undefined;
        }

        session.socketId = socket.id;
        socketSessionMap.set(socket.id, sessionToken);
        socket.join(session.roomId);

        if (session.disconnectTimer) {
            clearTimeout(session.disconnectTimer);
            session.disconnectTimer = undefined;

            socket.emit("playerReconnect", {
                roomId: session.roomId,
                playerId: session.playerId,
                state: JSON.parse(room.game.get_game_state(session.playerId)),
            });
        } else {
            if (session.reconnectTimer) {
                clearTimeout(session.reconnectTimer);
                session.reconnectTimer = undefined;
            }

            let response = room.game.toggle_id_with_bot(session.playerId, false);

            socket.emit("playerReconnect", {
                roomId: session.roomId,
                playerId: session.playerId,
                state: JSON.parse(room.game.get_game_state(session.playerId)),
            });

            io.to(session.roomId).emit("gameUpdate", response);
        }        
    });

    socket.on("initialiseGame", () => {
        const sessionToken = socketSessionMap.get(socket.id);
        if (!sessionToken) return socket.emit("error", "Socket has no session");

        const session = sessionMap.get(sessionToken);
        if (!session) return socket.emit("error", "Session token has no session");

        const room = roomMap.get(session.roomId);
        if (!room) return socket.emit("error", "No room associated with session");

        if (room.roomHostId != session.playerId) return socket.emit("error", "Cannot reset game as non-host");

        const initialiseResult = JSON.parse(room.game.initialise());
        if (initialiseResult.error) return socket.emit("error", initialiseResult.error);

        io.to(session.roomId).emit("gameStart", initialiseResult);
        turnTimeout(session.roomId, room.game, room.turnTimeout);
    });

    socket.on("startGame", () => {
        const sessionToken = socketSessionMap.get(socket.id);
        if (!sessionToken) return socket.emit("error", "Socket has no session");

        const session = sessionMap.get(sessionToken);
        if (!session) return socket.emit("error", "Session token has no session");

        const room = roomMap.get(session.roomId);
        if (!room) return socket.emit("error", "No room associated with session");

        if (room.roomHostId != session.playerId) return socket.emit("error", "Cannot start game as non-host");

        const startResult = JSON.parse(room.game.start());
        if (startResult.error) return socket.emit("error", startResult.error);

        io.to(session.roomId).emit("gameStart", startResult);
        turnTimeout(session.roomId, room.game, room.turnTimeout);
    });

    socket.on("playerMove", (moveData: { action: string, amount: number }) => {
        const sessionToken = socketSessionMap.get(socket.id);
        if (!sessionToken) return socket.emit("error", "Session does not exist");

        const session = sessionMap.get(sessionToken);
        if (!session) return socket.emit("error", "Session not associated with game");

        const room = roomMap.get(session.roomId);
        if (!room) return socket.emit("error", "Associated game no longer exists");

        const moveResult = room.game.player_move(session.playerId, moveData.action, moveData.amount);
        const parsedResult = JSON.parse(moveResult);

        if (parsedResult.error) {
            socket.emit("error", parsedResult.error);
        } else {
            io.to(session.roomId).emit("gameUpdate", parsedResult);
            turnTimeout(session.roomId, room.game, room.turnTimeout);
        }
    });
});