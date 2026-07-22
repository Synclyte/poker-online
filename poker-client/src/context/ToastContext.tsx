import React, { createContext, useContext, useState, useCallback, ReactNode, useEffect } from 'react';
import styles from "./toast.module.css";
import { PixelBox } from '../components/PixelBox/pixelbox';

type ToastType = "error" | "info" | "success" | "warning";
const defaultToastDurations = {
    "error": 5000,
    "info": 3000,
    "success": 5000,
    "warning": 3000,
};

const toastColours = {
    "error": '#e53935',
    "info": '#1e88e5',
    "success": '#43a047',
    "warning": '#fb8c00',
}

interface Toast {
    id: string,
    message: string,
    type: ToastType,
    duration: number | undefined,
};

interface ToastContextInterface {
    showToast: (message: string, type: ToastType, duration?: number | undefined) => void;
}

const ToastContext = createContext<ToastContextInterface | undefined>(undefined);

export const useToast = () => {
    const context = useContext(ToastContext);
    if (!context) throw new Error("Toast not wrapped correctly");

    return context;
}

const ToastItem = ({ toast, onRemove }: { toast: Toast, onRemove: (id: string) => void}) => {
    const [isBeingRemoved, setIsBeingRemoved] = useState<boolean>(false);

    const triggerRemoval = useCallback(() => {
        setIsBeingRemoved(true);
        setTimeout(() => onRemove(toast.id), 300);
    }, [toast.id, onRemove]);

    useEffect(() => {
        const timer = setTimeout(triggerRemoval, toast.duration || defaultToastDurations[toast.type]);
        return () => clearTimeout(timer);
    }, [toast.duration, toast.type, triggerRemoval]);

    return (
        <PixelBox 
            className={`${styles.toast} ${isBeingRemoved ? styles.exiting : ''}`}
            innerClassName={`${styles.toastInner}`}
            backgroundColour={`${toastColours[toast.type]}`}
        >
            <span>{toast.message}</span>
            <button className={styles.closeBtn} onClick={triggerRemoval}>x</button>
        </PixelBox>
    );    
}

export const ToastProvider: React.FC<{ children: ReactNode }> = ( {children} ) => {
    const [toasts, setToasts] = useState<Toast[]>([]);

    const showToast = useCallback((message: string, type: ToastType, duration: number | undefined) => {
        const id = Math.random().toString(36).substring(2, 9);
        setToasts((prev) => [...prev, { id, message, type, duration }]);
    }, []);

    const removeToast = useCallback((id: string) => {
        setToasts((prev) => prev.filter((toast) => toast.id !== id))
    }, []);

    return (
        <ToastContext.Provider value={{ showToast }}>
            {children}

            <div className={styles.toastContainer}>
                {toasts.map((toast) => (
                    <ToastItem key={toast.id} toast={toast} onRemove={removeToast} />
                ))}
            </div>
        </ToastContext.Provider>
    );
}