import React, { ReactNode } from 'react';
import styles from './pixelbox.module.css';

interface PixelBoxProps extends React.HTMLAttributes<HTMLDivElement> {
    children?: ReactNode;
    borderColour?: string;
    backgroundColour?: string;
    className?: string;
    innerClassName?: string;
    unclipped?: boolean;
}

export function PixelBox({
    children,
    borderColour = '',
    backgroundColour = '',
    className = '',
    innerClassName = '',
    unclipped = false,
    ...props
}: PixelBoxProps) {
    if (unclipped) {
        return (
            <div
                className={`${styles.wrapper} ${styles.unclippedWrapper} ${className}`}
                style={{
                    '--pixel-border-color': borderColour,
                    '--pixel-bg': backgroundColour
                } as React.CSSProperties}
                {...props}
            >
                <div className={styles.borderBg} />
                <div className={styles.innerBg} />
                <div className={`${styles.inner} ${styles.unclippedInner} ${innerClassName}`}>
                    {children}
                </div>
            </div>
        );
    }

    const isTransparent = borderColour === "transparent" || borderColour === "none";

    return (
        <div
            className={`${styles.wrapper} ${className}`}
            style={{
                '--pixel-border-color': borderColour,
                '--pixel-bg': backgroundColour
            } as React.CSSProperties}
            {...props}
        >
            {!isTransparent && <div className={styles.borderBg} />}
            {!isTransparent && <div className={styles.innerBg} />}
            <div className={`${styles.inner} ${isTransparent ? styles.transparentInner : ''} ${innerClassName}`}>
                {children}
            </div>
        </div>
    );
}