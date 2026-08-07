import buttonClickSfx from '../assets/sfx/button_click.mp3';
import buttonHoverSfx from '../assets/sfx/button_hover.mp3';
import cardDraw1Sfx from '../assets/sfx/card_draw_1.mp3';
import cardDraw2Sfx from '../assets/sfx/card_draw_2.mp3';
import foldSfx from '../assets/sfx/fold.mp3';
import specialDissolveSfx from '../assets/sfx/special_card_dissolve.mp3';
import dissolveSfx from '../assets/sfx/dissolve.mp3';
import toastSuccessSfx from '../assets/sfx/toast_success.mp3';
import toastErrorSfx from '../assets/sfx/toast_warning.mp3';
import callSfx from '../assets/sfx/call.mp3';

export type SoundEffect =
    | "button_click"
    | "button_hover"
    | "card_draw"
    | "fold"
    | "special_dissolve"
    | "dissolve"
    | "toast"
    | "toast_success"
    | "toast_error"
    | "call";

class SoundManager {
    private audioContext: AudioContext | null = null;
    private soundPool: Map<string, HTMLAudioElement[]> = new Map();
    private audioSources: Map<string, string> = new Map();
    private soundVolumeMultipliers: Map<string, number> = new Map([
        ["button_click", 0.30],
        ["button_hover", 0.15],
        ["card_draw_1", 0.18],
        ["card_draw_2", 0.18],
        ["fold", 0.45],
        ["special_dissolve", 0.35],
        ["dissolve", 0.40],
        ["toast", 0.25],
        ["toast_success", 0.25],
        ["toast_error", 0.25],
        ["call", 0.40],
        ["chips", 2.0],
    ]);
    private toggleDrawState: boolean = false;
    private isMuted: boolean = false;
    private volume: number = 0.6;

    constructor() {
        this.audioSources.set("button_click", buttonClickSfx);
        this.audioSources.set("button_hover", buttonHoverSfx);
        this.audioSources.set("card_draw_1", cardDraw1Sfx);
        this.audioSources.set("card_draw_2", cardDraw2Sfx);
        this.audioSources.set("fold", foldSfx);
        this.audioSources.set("special_dissolve", specialDissolveSfx);
        this.audioSources.set("dissolve", dissolveSfx);
        this.audioSources.set("toast", toastSuccessSfx);
        this.audioSources.set("toast_success", toastSuccessSfx);
        this.audioSources.set("toast_error", toastErrorSfx);
        this.audioSources.set("call", callSfx);
    }

    private getAudioContext(): AudioContext | null {
        if (typeof window === "undefined") return null;
        if (!this.audioContext) {
            const AudioCtx = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
            if (AudioCtx) {
                this.audioContext = new AudioCtx();
            }
        }
        if (this.audioContext && this.audioContext.state === "suspended") {
            this.audioContext.resume().catch(() => { });
        }
        return this.audioContext;
    }

    public setSoundVolumeMultiplier(sound: string, multiplier: number) {
        this.soundVolumeMultipliers.set(sound, Math.min(2, Math.max(0, multiplier)));
    }

    public playSound(sound: SoundEffect, volumeScale: number = 1.0) {
        if (this.isMuted) return;

        let soundKey: string = sound;
        if (sound === "card_draw") {
            this.toggleDrawState = !this.toggleDrawState;
            soundKey = this.toggleDrawState ? "card_draw_1" : "card_draw_2";
        }

        const src = this.audioSources.get(soundKey);
        if (!src) return;

        // gets pool for parallel playback of sounds
        try {
            let pool = this.soundPool.get(soundKey);
            if (!pool) {
                pool = [];
                this.soundPool.set(soundKey, pool);
            }

            let audio = pool.find(a => a.paused || a.ended);
            if (!audio) {
                audio = new Audio(src);
                if (pool.length < 10) {
                    pool.push(audio);
                }
            } else {
                audio.currentTime = 0;
            }

            const soundMultiplier = this.soundVolumeMultipliers.get(soundKey) ?? 0.5;
            audio.volume = Math.min(1, Math.max(0, this.volume * soundMultiplier * volumeScale));
            audio.play().catch(() => { });
        } catch (e) { }
    }

    public setChipVolumeMultiplier(multiplier: number) {
        this.soundVolumeMultipliers.set("chips", Math.min(3, Math.max(0, multiplier)));
    }

    /**
     * Procedural generator for chip stream sounds
     * Given a chip count and duration, creates chip clacking sounds matching the chip stream
     */
    public playChipStreamSFX(chipCount: number, durationMs: number) {
        if (this.isMuted) return;

        try {
            const ctx = this.getAudioContext();
            if (!ctx) return;

            const numClacks = Math.max(2, Math.min(16, Math.round(chipCount)));
            const totalDurationSec = Math.max(0.2, Math.min(1.5, durationMs / 1000));
            const intervalSec = totalDurationSec / numClacks;

            const now = ctx.currentTime;
            const chipMult = this.soundVolumeMultipliers.get("chips") ?? 1.0;

            for (let i = 0; i < numClacks; i++) {
                const time = now + i * intervalSec;

                // parameters for chip sound
                const baseFreq = 2600 + (Math.random() * 400 - 200);
                const duration = 0.025;

                // Noise burst for surface click
                const bufferSize = ctx.sampleRate * duration;
                const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
                const data = buffer.getChannelData(0);
                for (let j = 0; j < bufferSize; j++) {
                    data[j] = (Math.random() * 2 - 1) * Math.exp(-j / (bufferSize * 0.2));
                }

                const noiseNode = ctx.createBufferSource();
                noiseNode.buffer = buffer;

                const filter = ctx.createBiquadFilter();
                filter.type = "bandpass";
                filter.frequency.setValueAtTime(baseFreq, time);
                filter.Q.setValueAtTime(6, time);

                const gainNode = ctx.createGain();
                const vol = Math.min(1, Math.max(0, (this.volume * (0.35 + Math.random() * 0.15)) * chipMult));
                if (isNaN(vol) || !isFinite(vol)) continue;

                gainNode.gain.setValueAtTime(vol, time);
                gainNode.gain.exponentialRampToValueAtTime(0.001, time + duration);

                noiseNode.connect(filter);
                filter.connect(gainNode);
                gainNode.connect(ctx.destination);

                noiseNode.start(time);
                noiseNode.stop(time + duration);
            }
        } catch (e) {
            // this can crash the animation queue and make the game unplayable if it fails
        }
    }

    public initGlobalListeners() {
        if (typeof window === "undefined") return;
        if ((window as unknown as { _soundListenersInit?: boolean })._soundListenersInit) return;
        (window as unknown as { _soundListenersInit?: boolean })._soundListenersInit = true;

        document.addEventListener("click", (e) => {
            const target = e.target as HTMLElement | null;
            const btn = target?.closest("button, .btnWrapper, [role='button'], input[type='submit'], input[type='button']");
            if (btn && !btn.hasAttribute("data-no-sound") && !btn.closest("[data-no-sound='true']")) {
                this.playSound("button_click");
            }
        }, true);

        document.addEventListener("mouseover", (e) => {
            const target = e.target as HTMLElement | null;
            const btn = target?.closest("button, .btnWrapper, [role='button'], input[type='submit'], input[type='button']");
            if (btn && !(btn as unknown as { _hasHoveredSFX?: boolean })._hasHoveredSFX) {
                (btn as unknown as { _hasHoveredSFX?: boolean })._hasHoveredSFX = true;
                this.playSound("button_hover", 0.5);
            }
        }, true);

        document.addEventListener("mouseout", (e) => {
            const target = e.target as HTMLElement | null;
            const btn = target?.closest("button, .btnWrapper, [role='button'], input[type='submit'], input[type='button']");
            const related = e.relatedTarget as Node | null;
            if (btn && (!related || !btn.contains(related))) {
                delete (btn as unknown as { _hasHoveredSFX?: boolean })._hasHoveredSFX;
            }
        }, true);
    }

    public toggleMute(): boolean {
        this.isMuted = !this.isMuted;
        return this.isMuted;
    }

    public setVolume(vol: number) {
        this.volume = Math.min(1, Math.max(0, vol));
    }

    public getMuted(): boolean {
        return this.isMuted;
    }
}

export const soundManager = new SoundManager();
