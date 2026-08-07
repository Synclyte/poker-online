import React from 'react';
import { PixelBox } from '../PixelBox/pixelbox';
import styles from '../../pages/Game/Game.module.css';
import { GameStateData, PlayerLobbyInfo } from '../../pages/Game/game.types';

interface EndGameModalProps {
    gameState: GameStateData | null;
    myId: number | null;
    isHost: boolean;
    lobbyPlayers: PlayerLobbyInfo[];
    boxBorderColour: string;
    startNextRound: () => void;
    backToRoom: () => void;
}

export const EndGameModal: React.FC<EndGameModalProps> = ({
    gameState,
    myId,
    isHost,
    lobbyPlayers,
    boxBorderColour,
    startNextRound,
    backToRoom,
}) => {
    const isGameOver = (gameState?.roundName === "room" || gameState?.roundName === "gameover") && (gameState?.gamesPlayed ?? 0) > 0;
    if (!isGameOver) return null;

    const maxChips = Math.max(...(gameState?.players.map(p => p.chips) ?? [0]));
    const winners = gameState?.players.filter(p => p.chips === maxChips && maxChips > 0) ?? [];

    return (
        <div className={styles.endGameOverlay}>
            <PixelBox
                className={styles.endGamePixelBox}
                innerClassName={styles.endGameInner}
                borderColour={boxBorderColour}
                backgroundColour="#1a1a24"
            >
                <h2 className={styles.endGameTitle}>GAME OVER</h2>
                <div className={styles.endGameWinnerText}>
                    {winners.length === 1 ? (
                        <>
                            <span className={styles.winnerName}>
                                {winners[0].id === myId
                                    ? "YOU WIN!"
                                    : `${lobbyPlayers.find(lp => lp.id === winners[0].id)?.name || `Player ${winners[0].id}`} Wins!`}
                            </span>
                            <div className={styles.winnerChips}>Final Chips: {winners[0].chips}</div>
                        </>
                    ) : winners.length > 1 ? (
                        <>
                            <span className={styles.winnerName}>Tie</span>
                            <div className={styles.winnerChips}>
                                Winners: {winners.map(w => lobbyPlayers.find(lp => lp.id === w.id)?.name || `Player ${w.id}`).join(", ")} ({maxChips} chips)
                            </div>
                        </>
                    ) : (
                        <span className={styles.winnerName}>Game Finished</span>
                    )}
                </div>

                <div className={styles.endGameActions}>
                    {isHost && (
                        <button type="button" className={styles.btnWrapper} onClick={startNextRound}>
                            <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                                Start New Game
                            </PixelBox>
                        </button>
                    )}
                    <button type="button" className={styles.btnWrapper} onClick={backToRoom}>
                        <PixelBox innerClassName={styles.btnInner} borderColour={boxBorderColour}>
                            Back to Room
                        </PixelBox>
                    </button>
                </div>
            </PixelBox>
        </div>
    );
};
