import express from 'express';
import { Server, Socket } from 'socket.io';
import { createServer } from 'http';

import { Game } from './pkg/poker_server.js';

const app = express();
const httpServer = createServer(app);
const io = new Server(httpServer, { cors: { origin: '*'}});

const gameMap = new Map<string, Game>();
const socketPlayerMap = new Map<string, { roomId: string, playerId: number, isHost: boolean }>();

io.on("connection", (socket: Socket) => {
    socket.on("disconnecting", () => {
        for (const room in socket.rooms) {
            if (room !== socket.id) {
                io.emit("playerDisconnect", socketPlayerMap.get(socket.id));
            }
        }
        let current_game = gameMap.get(socket.id);
        let player_id = socketPlayerMap.get(socket.id)?.playerId;

        if (!player_id) {
            console.log("failed to remove player with socket id: " + socket.id + " from game");
            return;
        } else if (!current_game) {
            console.log("failed to find game associated with socket id: " + socket.id);
            return;
        }

        let response: JSON = JSON.parse(current_game.toggle_id_with_bot(player_id, true));
        if ("error" in response) {
            console.log(response.error);
            return;
        }

        setTimeout(() => {
            current_game.queue_remove_player(player_id);
        }, 3 * 60 * 1000) 
    });

    socket.on("disconnect", () => {

    });

    socket.on("createGame", (config: string) => {

    });

    socket.on("joinGame", () => {

    });

    socket.on("playerMove", (moveData: string) => {

    });
});