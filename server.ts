import express from 'express';
import { Server, Socket } from 'socket.io';
import { createServer } from 'http';

import * as game from '../pkg/poker-server';

const app = express();
const httpServer = createServer(app);
const io = new Server(httpServer);
