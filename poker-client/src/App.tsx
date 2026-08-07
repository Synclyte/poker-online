import { useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { SocketProvider } from './context/SocketContext';
import { ToastProvider } from './context/ToastContext';
import { soundManager } from './utils/sound';

import { Home } from './pages/Home/home';
import { Lobby } from './pages/Lobby/lobby';
import { Game } from './pages/Game/game';

export default function App() {
    useEffect(() => {
        soundManager.initGlobalListeners();
    }, []);

    return (
        <ToastProvider>
        <SocketProvider>
        <Router>
        <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/lobby/:roomId" element={<Lobby />} />
            <Route path="/game/:roomId" element={<Game />} />
        </Routes>
        </Router>
        </SocketProvider>
        </ToastProvider>
    );
}