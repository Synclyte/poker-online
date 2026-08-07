import React, { useState, useEffect, useRef } from 'react';
import { useSocket } from '../../context/SocketContext';
import { PixelBox } from '../PixelBox/pixelbox';
import styles from './Chat.module.css';

export interface ChatMessage {
    id: string;
    senderId: number | null;
    senderName: string;
    text: string;
    timestamp: number;
}

interface ChatProps {
    roomId: string | undefined;
    myId: number | null;
    boxBorderColour?: string;
    collapsible?: boolean;
    initialCollapsed?: boolean;
    position?: "floating" | "top-left" | "inline";
    className?: string;
    showHeader?: boolean;
}

export const Chat: React.FC<ChatProps> = ({
    roomId,
    myId,
    boxBorderColour = 'var(--border-colour)',
    collapsible = true,
    initialCollapsed = false,
    position = "inline",
    className = '',
    showHeader = true,
}) => {
    const { socket, isConnected } = useSocket();
    const [messages, setMessages] = useState<ChatMessage[]>([]);
    const [inputText, setInputText] = useState('');
    const [isCollapsed, setIsCollapsed] = useState(initialCollapsed);
    const [unreadCount, setUnreadCount] = useState(0);

    const messageListRef = useRef<HTMLDivElement | null>(null);

    const scrollToBottom = () => {
        if (messageListRef.current) {
            messageListRef.current.scrollTop = messageListRef.current.scrollHeight;
        }
    };

    useEffect(() => {
        if (!isCollapsed) {
            scrollToBottom();
            setUnreadCount(0);
        }
    }, [messages, isCollapsed]);

    useEffect(() => {
        if (!socket || !isConnected || !roomId) return;

        socket.emit("getChatHistory");

        const handleChatHistory = (history: ChatMessage[]) => {
            setMessages(history || []);
        };

        const handleChatMessage = (msg: ChatMessage) => {
            setMessages((prev) => [...prev, msg]);
            if (isCollapsed) {
                setUnreadCount((c) => c + 1);
            }
        };

        socket.on("chatHistory", handleChatHistory);
        socket.on("chatMessage", handleChatMessage);

        return () => {
            socket.off("chatHistory", handleChatHistory);
            socket.off("chatMessage", handleChatMessage);
        };
    }, [socket, isConnected, roomId, isCollapsed]);

    const handleSend = () => {
        if (!socket || !isConnected || !inputText.trim()) return;
        socket.emit("sendChatMessage", inputText);
        setInputText('');
    };

    const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === "Enter") {
            e.preventDefault();
            handleSend();
        }
    };

    const toggleCollapse = () => {
        if (!collapsible) return;
        setIsCollapsed((prev) => {
            if (prev) setUnreadCount(0);
            return !prev;
        });
    };

    const positionClass = position === "top-left"
        ? styles.chatWrapperTopLeft
        : position === "floating"
            ? styles.chatWrapperFloating
            : styles.chatWrapperInline;

    return (
        <div
            className={[
                styles.chatWrapper,
                positionClass,
                className,
            ].filter(Boolean).join(" ")}
        >
            <PixelBox
                innerClassName={styles.chatMainBox}
                borderColour="transparent"
                backgroundColour="rgba(18, 18, 26, 0.95)"
            >
                {showHeader && (
                    <div
                        className={[
                            styles.chatHeader,
                            isCollapsed ? styles.chatHeaderCollapsed : "",
                        ].filter(Boolean).join(" ")}
                        onClick={toggleCollapse}
                    >
                        <div className={styles.chatTitleGroup}>
                            <span className={styles.chatTitle}>Game Chat</span>
                            {isCollapsed && unreadCount > 0 && (
                                <span className={styles.unreadBadge}>{unreadCount}</span>
                            )}
                        </div>
                        {collapsible && (
                            <button type="button" className={styles.toggleBtn} aria-label="Toggle chat">
                                <svg
                                    className={[styles.collapseArrow, isCollapsed ? styles.arrowUp : ""].filter(Boolean).join(" ")}
                                    width="10"
                                    height="6"
                                    viewBox="0 0 10 6"
                                >
                                    <path d="M0 0h10v2H8v2H6v2H4V4H2V2H0z" fill="currentColor" />
                                </svg>
                            </button>
                        )}
                    </div>
                )}

                {!isCollapsed && (
                    <div className={styles.chatBody}>
                        <div className={styles.messageList} ref={messageListRef}>
                            {messages.map((msg) => {
                                const isSelf = msg.senderId !== null && msg.senderId === myId;

                                return (
                                    <div key={msg.id} className={styles.messageItem}>
                                        <span
                                            className={[
                                                styles.senderName,
                                                isSelf ? styles.selfName : '',
                                            ].filter(Boolean).join(' ')}
                                        >
                                            {msg.senderName}:
                                        </span>
                                        <span className={styles.msgText}>{msg.text}</span>
                                    </div>
                                );
                            })}
                        </div>

                        <div className={styles.inputBar}>
                            <input
                                type="text"
                                className={styles.chatInput}
                                placeholder="Type a message..."
                                value={inputText}
                                onChange={(e) => setInputText(e.target.value)}
                                onKeyDown={handleKeyDown}
                                maxLength={200}
                            />
                        </div>
                    </div>
                )}
            </PixelBox>
        </div>
    );
};
