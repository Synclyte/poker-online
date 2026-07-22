import React, { ReactNode } from 'react';
import styles from './PixelBox.module.css';

interface PixelBoxProps extends React.HTMLAttributes<HTMLDivElement> {
    children?: ReactNode;
    borderColour?: string;
    backgroundColour?: string;
    className?: string;
    innerClassName?: string;
}

export function PixelBox({ 
    children, 
    borderColour = '', 
    backgroundColour = '',
    className = '', 
    innerClassName = '',
    ...props 
}: PixelBoxProps) {
    return (
        <div 
            className={`${styles.wrapper} ${className}`}
            style={{ 
                '--pixel-border-color': borderColour,
                '--pixel-bg': backgroundColour
            } as React.CSSProperties}
            {...props}
        >
            <div className={`${styles.inner} ${innerClassName}`}>
                {children}
            </div>
        </div>
    );
}