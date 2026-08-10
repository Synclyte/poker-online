import React from 'react';
import styles from '../../pages/Game/game.module.css';
import { GameConfig, GameStateData } from '../../pages/Game/game.types';

interface GameHeaderProps {
    roomId: string | undefined;
    gameState: GameStateData | null;
    config: GameConfig | null;
    turnTimeRemaining: number | null;
}

export const GameHeader: React.FC<GameHeaderProps> = ({
    roomId,
    gameState,
    config,
    turnTimeRemaining,
}) => {
    return (
        <div className={styles.tableHeader}>
            <div className={styles.headerRoomCode}>Room Code: <strong>{roomId?.toUpperCase() ?? "---"}</strong></div>
            <div className={styles.headerInfo}>
                <div className={styles.headerRoundBlock}>
                    <span>Round:</span>
                    <strong>
                        {gameState?.roundName === "gameover"
                            ? "Game Over"
                            : gameState?.roundName
                                ? gameState.roundName.charAt(0).toUpperCase() + gameState.roundName.slice(1)
                                : "Waiting"}
                    </strong>
                </div>
                {config?.roundLimit !== undefined && config.roundLimit > 0 && (
                    <div className={styles.headerHandsBlock}>
                        <span>Hands Played:</span>
                        <strong className={config.roundLimit - (gameState?.gamesPlayed ?? 0) <= 5 ? styles.seatTimerLow : ""}>
                            {gameState?.gamesPlayed ?? 0}/{config.roundLimit}
                        </strong>
                    </div>
                )}
                <div className={styles.headerTimerBlock}>
                    {turnTimeRemaining !== null ? (
                        <>
                            <span>Move Time:</span>
                            <strong className={turnTimeRemaining <= 5 ? styles.seatTimerLow : ""}>{turnTimeRemaining}s</strong>
                        </>
                    ) : (
                        <span style={{ opacity: 0 }}>Move Time: 00s</span>
                    )}
                </div>
            </div>
        </div>
    );
};
