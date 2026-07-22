import React, { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { io, Socket } from 'socket.io-client';

const SOCKET_URL = 'http://localhost:3000';

interface SocketContextType {
    socket: Socket | null;
    isConnected: boolean;
}

const SocketContext = createContext<SocketContextType>({
    socket: null,
    isConnected: false,
});

export const useSocket = () => useContext(SocketContext);

interface SocketProviderProps {
    children: ReactNode;
}

export const SocketProvider: React.FC<SocketProviderProps> = ({ children }) => {
    const [socket, setSocket] = useState<Socket | null>(null);
    const [isConnected, setIsConnected] = useState(false);

    useEffect(() => {
        const clientSocket = io(SOCKET_URL, {
            transports: ['websocket'], 
        });

        setSocket(clientSocket);

        // when connecting, check whether client had an active game
        // if they did, reconnect them to the game
        clientSocket.on('connect', () => {
            setIsConnected(true);

            const sessionToken = localStorage.getItem('sessionToken');
            if (sessionToken) {
                clientSocket.emit('reconnectGame', sessionToken);
            }
        });

        clientSocket.on('disconnect', () => {
            setIsConnected(false);
        });

        return () => {
            clientSocket.disconnect();
        };
    }, []);

    return (
        <SocketContext.Provider value={{ socket, isConnected }}>
            {children}
        </SocketContext.Provider>
    );
};