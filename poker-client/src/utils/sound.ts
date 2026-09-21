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
import countdownSfx from '../assets/sfx/countdown.mp3';

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
    | "call"
    | "countdown";

class SoundManager {
    private audioContext: AudioContext | null = null;
    private soundPool: Map<string, HTMLAudioElement[]> = new Map();
    private audioSources: Map<string, string> = new Map();
    private soundVolumeMultipliers: Map<string, number> = new Map([
        ["button_click", 0.50],
        ["button_hover", 0.30],
        ["card_draw_1", 0.34],
        ["card_draw_2", 0.34],
        ["fold", 0.85],
        ["special_dissolve", 0.65],
        ["dissolve", 0.75,],
        ["toast", 0.5],
        ["toast_success", 0.5],
        ["toast_error", 0.5],
        ["call", 0.8],
        ["chips", 4.0],
        ["countdown", 0.4],
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
        this.audioSources.set("countdown", countdownSfx);

        if (typeof window !== "undefined") {
            try {
                const countdownAudio = new Audio(countdownSfx);
                countdownAudio.preload = "auto";
                this.soundPool.set("countdown", [countdownAudio]);
            } catch { }
        }

        if (typeof window !== "undefined" && window.localStorage) {
            const savedVol = localStorage.getItem("globalVolume");
            if (savedVol !== null) {
                const parsed = parseFloat(savedVol);
                if (Number.isFinite(parsed)) {
                    this.volume = Math.min(1, Math.max(0, parsed));
                }
            }
            const savedMute = localStorage.getItem("globalMuted");
            if (savedMute === "true") {
                this.isMuted = true;
            }
        }
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
        if (this.isMuted || this.volume <= 0) return;

        let soundKey: string = sound;
        if (sound === "card_draw") {
            this.toggleDrawState = !this.toggleDrawState;
            soundKey = this.toggleDrawState ? "card_draw_1" : "card_draw_2";
        }

        const src = this.audioSources.get(soundKey);
        if (!src) return;

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

    public playChipStreamSFX(chipCount: number, durationMs: number) {
        if (this.isMuted || this.volume <= 0) return;

        try {
            const ctx = this.getAudioContext();
            if (!ctx) return;

            const numClacks = Math.max(2, Math.min(30, Math.round(chipCount)));
            const totalDurationSec = Math.max(0.2, Math.min(2, durationMs / 1000));
            const intervalSec = totalDurationSec / numClacks;

            const now = ctx.currentTime;
            const chipMult = this.soundVolumeMultipliers.get("chips") ?? 1.0;

            for (let i = 0; i < numClacks; i++) {
                const time = now + i * intervalSec;

                const baseFreq = 2600 + (Math.random() * 400 - 200);
                const duration = 0.025;

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
        } catch (e) { }
    }

    public initGlobalListeners() {
        if (typeof window === "undefined") return;
        if ((window as unknown as { _soundListenersInit?: boolean })._soundListenersInit) return;
        (window as unknown as { _soundListenersInit?: boolean })._soundListenersInit = true;

        document.addEventListener("click", (e) => {
            const target = e.target as HTMLElement | null;
            const btn = target?.closest("button, .btnWrapper, [role='button'], input[type='submit'], input[type='button']");
            if (btn && !btn.hasAttribute("data-no-sound") && !btn.closest("[data-no-sound='true']") && !btn.hasAttribute("data-no-click-sound") && !btn.closest("[data-no-click-sound='true']")) {
                this.playSound("button_click");
            }
        }, true);

        document.addEventListener("mouseover", (e) => {
            const target = e.target as HTMLElement | null;
            const btn = target?.closest("button, .btnWrapper, [role='button'], input[type='submit'], input[type='button']");
            if (btn && !btn.hasAttribute("data-no-hover-sound") && !btn.closest("[data-no-hover-sound='true']")) {
                if (!(btn as unknown as { _hasHoveredSFX?: boolean })._hasHoveredSFX) {
                    (btn as unknown as { _hasHoveredSFX?: boolean })._hasHoveredSFX = true;
                    this.playSound("button_hover", 0.5);
                }
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
        if (typeof window !== "undefined" && window.localStorage) {
            localStorage.setItem("globalMuted", this.isMuted.toString());
        }
        this.updateAllAudioVolumes();
        return this.isMuted;
    }

    public setVolume(vol: number) {
        this.volume = Math.min(1, Math.max(0, vol));
        if (typeof window !== "undefined" && window.localStorage) {
            localStorage.setItem("globalVolume", this.volume.toString());
        }
        if (this.volume > 0 && this.isMuted) {
            this.isMuted = false;
            if (typeof window !== "undefined" && window.localStorage) {
                localStorage.setItem("globalMuted", "false");
            }
        }
        this.updateAllAudioVolumes();
    }

    public getVolume(): number {
        return this.volume;
    }

    public getMuted(): boolean {
        return this.isMuted;
    }

    public setMuted(muted: boolean) {
        this.isMuted = muted;
        if (typeof window !== "undefined" && window.localStorage) {
            localStorage.setItem("globalMuted", muted.toString());
        }
        this.updateAllAudioVolumes();
    }

    private updateAllAudioVolumes() {
        for (const [soundKey, pool] of this.soundPool.entries()) {
            const soundMultiplier = this.soundVolumeMultipliers.get(soundKey) ?? 0.5;
            const vol = (this.isMuted || this.volume <= 0) ? 0 : Math.min(1, Math.max(0, this.volume * soundMultiplier));
            for (const audio of pool) {
                audio.volume = vol;
            }
        }
    }

    public playCountdownTick(step: number) {
        if (this.isMuted || this.volume <= 0) return;

        const src = this.audioSources.get("countdown");
        if (!src) return;

        try {
            let pool = this.soundPool.get("countdown");
            if (!pool) {
                pool = [];
                this.soundPool.set("countdown", pool);
            }

            let audio = pool.find(a => a.paused || a.ended);
            if (!audio) {
                audio = new Audio(src);
                audio.preload = "auto";
                if (pool.length < 5) {
                    pool.push(audio);
                }
            }

            const multiplier = this.soundVolumeMultipliers.get("countdown") ?? 0.5;
            audio.volume = Math.min(1, Math.max(0, this.volume * multiplier));
            audio.currentTime = 0;
            audio.play().catch(() => {});
        } catch {}
    }
}

export const soundManager = new SoundManager();
