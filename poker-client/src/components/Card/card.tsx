import React from 'react';
import styles from '../../pages/Game/game.module.css';
import { CardView, SpecialCardInfo, ActiveModifierInfo } from '../../pages/Game/game.types';
import { getAssetUrl } from '../../utils/assets';

export function parseCardString(cardStr: string) {
    if (!cardStr) {
        return { isHidden: true, rank: "", suitSymbol: "", color: "" };
    }

    const cleanStr = cardStr.startsWith("*") ? cardStr.substring(1) : cardStr;
    const parts = cleanStr.split(":");
    const rawRank = (parts[0] || "").toLowerCase();
    const rawSuit = (parts[1] || "").toLowerCase();

    const suitMap: { [key: string]: { symbol: string; color: string } } = {
        "s": { symbol: "spade", color: "#000000" },
        "c": { symbol: "club", color: "#000000" },
        "h": { symbol: "heart", color: "#e53935" },
        "d": { symbol: "diamond", color: "#e53935" }
    };

    const displayRank = rawRank.toUpperCase();
    const suitInfo = suitMap[rawSuit] || { symbol: "?", color: "#000000" };

    return {
        isHidden: false,
        rank: displayRank,
        suitSymbol: suitInfo.symbol,
        color: suitInfo.color
    };
}

export const specialCardInfo: Record<string, SpecialCardInfo> = {
    ReplaceCardSelf: {
        label: "Replace Hole Card",
        description: "Replaces a selected hole card with a random new card, drawn from the deck",
        target: "selfCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_replace_card_self.png",
    },
    DrawCardSelf: {
        label: "Draw Hole Card",
        description: "Draws an additional hole card from the deck. This card is visible to opponents",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_card_self.png",
    },
    DrawCardCommunity: {
        label: "Draw Community Card",
        description: "Draws an additional card from the deck, adding it to community cards",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_card_community.png",
    },
    ReplaceCardCommunity: {
        label: "Replace Community",
        description: "Replace a selected community card with a new card from the deck",
        target: "communityCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_replace_card_community.png",
    },
    RemoveCardCommunity: {
        label: "Remove Community",
        description: "Removes a selected community card",
        target: "communityCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_remove_card_community.png",
    },
    RevealOpponentCard: {
        label: "Reveal Card",
        description: "Reveals a chosen hole card of a selected opponent to you until the end of the current hand",
        target: "otherCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_reveal_opponent_card.png",
    },
    WithdrawBet: {
        label: "Withdraw Bet",
        description: "Fold recovering your bet from the pot",
        target: "none",
        imgSrc: "/src/assets/special/special_withdraw_bet.png",
    },
    AnteUp: {
        label: "Ante Up",
        description: "Doubles the blind and minimum raise costs permanently. Grants a Chip Boost special card on use",
        target: "none",
        imgSrc: "/src/assets/special/special_ante_up.png",
    },
    PotMult: {
        label: "Pot Multiplier",
        description: "Multiplies player winnings from the pot this hand by 1.4x",
        target: "none",
        imgSrc: "/src/assets/special/special_pot_mult.png",
    },
    RaiseBlock: {
        label: "Raise Block",
        description: "Prevents raises by all players until your next turn",
        target: "none",
        imgSrc: "/src/assets/special/special_raise_block.png",
    },
    SpecialBlock: {
        label: "Special Block",
        description: "Prevent all special cards from being played until your next turn",
        target: "none",
        imgSrc: "/src/assets/special/special_special_block.png",
    },
    ChipBoost: {
        label: "Chip Boost",
        description: "Gain chips proportional to the blind cost",
        target: "none",
        imgSrc: "/src/assets/special/special_chip_boost.png",
    },
    ChipGamble: {
        label: "Chip Gamble",
        description: "When used, has a chance to give a significant number of chips. If it fails, lose half of your chips and fold",
        target: "none",
        imgSrc: "/src/assets/special/special_chip_gamble.png",
    },
    Blackjack: {
        label: "Blackjack",
        description: "Changes scoring system to Blackjack for the current hand. Provides a Draw Hole Card special card to all players on use",
        target: "none",
        imgSrc: "/src/assets/special/special_blackjack.png",
    },
    DrawHeart: {
        label: "Draw Heart",
        description: "Forces the next drawn community card to have the Heart suit",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_heart.png",
    },
    DrawSpade: {
        label: "Draw Spade",
        description: "Forces the next drawn community card to have the Spade suit",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_spade.png",
    },
    DrawDiamond: {
        label: "Draw Diamond",
        description: "Forces the next drawn community card to have the Diamond suit",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_diamond.png",
    },
    DrawClub: {
        label: "Draw Club",
        description: "Forces the next drawn community card to have the Club suit",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_club.png",
    },
    DrawFace: {
        label: "Draw Face",
        description: "Forces the next drawn community card to be a face card",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_face.png",
    },
    DrawHigh: {
        label: "Draw High",
        description: "Forces the next community card to be a 10 or above",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_high.png",
    },
    DrawLow: {
        label: "Draw Low",
        description: "Forces the next community card to be a 5 or below",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_low.png",
    },
    InvalidateFlush: {
        label: "No Flushes",
        description: "Invalidates Flushes for this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_invalidate_flush.png",
    },
    InvalidateStraight: {
        label: "No Straights",
        description: "Invalidates Straights for this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_invalidate_straight.png",
    },
    InvalidateThreeOfAKind: {
        label: "No Three of a Kind",
        description: "Invalidates Three of a Kinds for this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_invalidate_three_of_a_kind.png",
    },
    InvalidateTwoPair: {
        label: "No Two Pair",
        description: "Invalidates Two Pairs for this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_invalidate_two_pair.png",
    },
    InvalidateFullHouse: {
        label: "No Full House",
        description: "Invalidates Full Houses for this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_invalidate_full_house.png",
    },
    Special: {
        label: "Special",
        description: "Secret - small chance to obtain when using a special card. Gives multiple special cards on use, exceeding the typical special card limit",
        target: "none",
        imgSrc: "/src/assets/special/special_special.png",
    },
    DrawHigherThanLast: {
        label: "Draw Higher",
        description: "Forces the next drawn community card to have a rank equal to or higher than the previous draw",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_higher_than_last.png",
    },
    DrawLowerThanLast: {
        label: "Draw Lower",
        description: "Forces the next drawn community card to have a rank equal to or lower than the previous draw",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_lower_than_last.png",
    },
    DrawSameSuitAsLast: {
        label: "Draw Same Suit",
        description: "Forces the next drawn community card to be the same suit as the previous draw",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_same_suit_as_last.png",
    },
    Discard: {
        label: "Discard",
        description: "Secret - small chance to obtain when discarding a special card. Prevents any more community cards from being drawn for the rest of this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_discard.png",
    },
    HandSwap: {
        label: "Hand Swap",
        description: "Swaps your hand with that of a chosen opponent. Fails if their hand is currently better than yours",
        target: "other",
        imgSrc: "/src/assets/special/special_hand_swap.png",
    },
    Joker: {
        label: "Joker",
        description: "Converts a selected community card into a Joker",
        target: "communityCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_joker.png",
    },
};

export function getSpecialCardInfo(
    cardStr: string,
    blindSize?: number,
    anteMult?: number,
    gambleSuccessChance?: number
): SpecialCardInfo {
    if (!cardStr) {
        return { label: "Special", description: "No description", target: "none" };
    }
    const info = specialCardInfo[cardStr] ||
        specialCardInfo[cardStr.charAt(0).toUpperCase() + cardStr.slice(1)] ||
        Object.entries(specialCardInfo).find(([k]) => k.toLowerCase() === cardStr.toLowerCase())?.[1];

    const baseInfo = info ?? {
        label: cardStr,
        description: "No description",
        target: "none",
    };

    const bSize = blindSize ?? 20;
    const aMult = anteMult ?? 1;

    let finalInfo = {
        ...baseInfo,
        imgSrc: baseInfo.imgSrc ? getAssetUrl(baseInfo.imgSrc) : baseInfo.imgSrc,
    };

    if (cardStr.toLowerCase().includes("chipboost")) {
        const boostChips = Math.round(bSize * aMult * 2.5);
        finalInfo = {
            ...finalInfo,
            description: `Gain +${boostChips} chips on use, scaling with blind cost`,
        };
    } else if (cardStr.toLowerCase().includes("chipgamble")) {
        const gambleChips = Math.round(bSize * aMult * 7.5);
        const chance = gambleSuccessChance ?? 0.9;
        const successChance = Math.round(chance * 100);
        finalInfo = {
            ...finalInfo,
            description: `Results in one of two outcomes. ${successChance}% chance to gain +${gambleChips} chips, scaling with blind cost. ${100 - successChance}% chance to force you to fold and lose half of your chips`,
        };
    }

    return finalInfo;
}

export function buildSpecialCardFrontHTML(cardValue?: string | null): string {
    const info = cardValue ? getSpecialCardInfo(cardValue) : null;
    const imgSrc = info?.imgSrc ? getAssetUrl(info.imgSrc) : "";
    const imgHtml = imgSrc
        ? `<img src="${imgSrc}" alt="${info?.label ?? ''}" class="${styles.specialImage}" draggable="false" />`
        : `<span>${info?.label ?? "Special"}</span>`;

    const smallMark = getAssetUrl("/src/assets/special/specialsmall.png");
    return `
        <img style="top: 0; left: 0" class="${styles.specialMark}" src="${smallMark}" draggable="false" />
        <img style="top: 0; right: 0" class="${styles.specialMark}" src="${smallMark}" draggable="false" />
        ${imgHtml}
        <img style="bottom: 0; left: 0" class="${styles.specialMark}" src="${smallMark}" draggable="false" />
        <img style="bottom: 0; right: 0" class="${styles.specialMark}" src="${smallMark}" draggable="false" />
    `.trim();
}

export function buildSpecialCardInnerHTML(cardValue: string, isSelf: boolean): string {
    const specialImg = getAssetUrl("/src/assets/special/special.png");
    return `
        <div class="${styles.cardInner} ${isSelf ? styles.cardFlipped : ""}">
            <div class="${styles.cardBackFace} ${styles.specialBackFace}">
                <img src="${specialImg}" alt="" draggable="false" />
            </div>
            <div class="${styles.cardFront}">
                ${buildSpecialCardFrontHTML(cardValue)}
            </div>
        </div>
    `.trim();
}

const handNameMap: Record<string, string> = {
    royalflush: "Royal Flush",
    straightflush: "Straight Flush",
    fourofakind: "Four of a Kind",
    fullhouse: "Full House",
    flush: "Flush",
    straight: "Straight",
    threeofakind: "Three of a Kind",
    twopair: "Two Pair",
    pair: "Pair",
    highcard: "High Card",
    none: "Nothing",
};

export function getSanitisedHandName(internalName: string): string {
    return handNameMap[internalName.toLowerCase()] ?? internalName;
}

export function parseModifier(mod: any, index: number): ActiveModifierInfo {
    let name = "Active Modifier";
    let description = "An active game modifier.";
    let duration = "Active";
    let typeKey = "unknown";

    let imgSrc = "/src/assets/special/special.png";

    if (typeof mod.effect === "string") {
        const eff = mod.effect.toLowerCase();
        if (eff.includes("raisesblocked")) {
            name = "Raise Block";
            description = "Raises are prevented";
            typeKey = "raises_blocked";
            imgSrc = getSpecialCardInfo("RaiseBlock").imgSrc ?? imgSrc;
        } else if (eff.includes("specialsblocked")) {
            name = "Special Block";
            description = "Special cards cannot be played";
            typeKey = "specials_blocked";
            imgSrc = getSpecialCardInfo("SpecialBlock").imgSrc ?? imgSrc;
        } else if (eff.includes("blackjackscoring")) {
            name = "Blackjack Scoring";
            description = "Closest hand sum to 21 wins";
            typeKey = "blackjack_scoring";
            imgSrc = getSpecialCardInfo("Blackjack").imgSrc ?? imgSrc;
        } else if (eff.includes("revealcard") || eff.includes("revealopponentcard")) {
            name = "Reveal Card";
            description = "A card has been revealed";
            typeKey = "reveal_card";
            imgSrc = getSpecialCardInfo("RevealOpponentCard").imgSrc ?? imgSrc;
        }
    } else if (mod.effect && typeof mod.effect === "object") {
        if ("potMultiplier" in mod.effect || "PotMultiplier" in mod.effect) {
            const val = mod.effect.potMultiplier?.multiplier ?? mod.effect.PotMultiplier?.multiplier ?? 1.4;
            name = `Pot Multiplier (${val}x)`;
            description = `Multiplies all pot gains by ${val}x.`;
            typeKey = "pot_multiplier";
            imgSrc = getSpecialCardInfo("PotMult").imgSrc ?? imgSrc;
        } else if ("anteMultiplier" in mod.effect || "AnteMultiplier" in mod.effect) {
            const val = mod.effect.anteMultiplier?.multiplier ?? mod.effect.AnteMultiplier?.multiplier ?? 2.0;
            name = `Ante Multiplier (${val}x)`;
            description = `Multiplies minimum bets and blinds by ${val}x.`;
            typeKey = "ante_multiplier";
            imgSrc = getSpecialCardInfo("AnteUp").imgSrc ?? imgSrc;
        } else if ("revealCard" in mod.effect || "RevealCard" in mod.effect || "reveal_card" in mod.effect) {
            name = "Reveal Card";
            description = "Opponent card revealed";
            typeKey = "reveal_card";
            imgSrc = getSpecialCardInfo("RevealOpponentCard").imgSrc ?? imgSrc;
        } else if ("invalidateHand" in mod.effect || "InvalidateHand" in mod.effect) {
            const rawHand = mod.effect.invalidateHand ?? mod.effect.InvalidateHand ?? "Hand";
            const rawHandStr = typeof rawHand === "string" ? rawHand : String(rawHand ?? "Hand");
            const handName = getSanitisedHandName(rawHandStr);
            const pluralHandName = `${handName}${handName === "Flush" ? "es" : "s"}`;
            name = `${pluralHandName} Disabled`;
            description = `${pluralHandName} are disabled and cannot score`;
            typeKey = "invalidate_hand";

            const cardKey = `Invalidate${rawHandStr.charAt(0).toUpperCase() + rawHandStr.slice(1)}`;
            imgSrc = getSpecialCardInfo(cardKey).imgSrc ?? imgSrc;
        } else if ("forceCommunityDraw" in mod.effect || "ForceCommunityDraw" in mod.effect) {
            const rule = mod.effect.forceCommunityDraw ?? mod.effect.ForceCommunityDraw;
            const ruleStr = typeof rule === "string" ? rule : String(rule ?? "");

            const cardKey = `Draw${ruleStr}`;
            imgSrc = getSpecialCardInfo(cardKey).imgSrc ?? imgSrc;

            switch (ruleStr) {
                case "Heart":
                    name = "Draw Heart";
                    description = "Forces community card draws to have the Heart suit";
                    typeKey = "draw_heart";
                    break;
                case "Spade":
                    name = "Draw Spade";
                    description = "Forces community card draws to have the Spade suit";
                    typeKey = "draw_spade";
                    break;
                case "Diamond":
                    name = "Draw Diamond";
                    description = "Forces community card draws to have the Diamond suit";
                    typeKey = "draw_diamond";
                    break;
                case "Club":
                    name = "Draw Club";
                    description = "Forces community card draws to have the Club suit";
                    typeKey = "draw_club";
                    break;
                case "Face":
                    name = "Draw Face Card";
                    description = "Forces community card draws to be face cards (Jack, Queen, King)";
                    typeKey = "draw_face";
                    break;
                case "High":
                    name = "Draw High Card";
                    description = "Forces community card draws to be 10 or higher (10, Jack, Queen, King, Ace)";
                    typeKey = "draw_high";
                    break;
                case "Low":
                    name = "Draw Low Card";
                    description = "Forces community card draws to be 5 or lower (5, 4, 3, 2, Ace)";
                    typeKey = "draw_low";
                    break;
                case "HigherThanLast":
                    name = "Draw Higher";
                    description = "Forces community card draws to have a rank equal to or higher than the previous draw";
                    typeKey = "draw_higher_than_last";
                    break;
                case "LowerThanLast":
                    name = "Draw Lower";
                    description = "Forces community card draws to have a rank equal to or lower than the previous draw";
                    typeKey = "draw_lower_than_last";
                    break;
                case "SameSuitAsLast":
                    name = "Draw Same Suit";
                    description = "Forces community card draws to have the same suit as the previous draw";
                    typeKey = "draw_same_suit_as_last";
                    break;
                default:
                    name = `Draw ${ruleStr}`;
                    description = `Community card draws follow the ${ruleStr} rule`;
                    typeKey = `draw_${ruleStr.toLowerCase()}`;
                    break;
            }
        }
    }

    if (typeof mod.expiry === "string") {
        if (mod.expiry.toLowerCase().includes("gameend")) {
            duration = "Duration: Game";
        }
    } else if (mod.expiry && typeof mod.expiry === "object") {
        if ("onCommunityDraw" in mod.expiry || "OnCommunityDraw" in mod.expiry) {
            const count = mod.expiry.onCommunityDraw?.count ?? mod.expiry.OnCommunityDraw?.count ?? 1;
            duration = `Duration: ${count} Community Draw${count > 1 ? "s" : ""}`;

        } else if ("onPlayerTurn" in mod.expiry || "OnPlayerTurn" in mod.expiry) {
            const count = mod.expiry.onPlayerTurn?.count ?? mod.expiry.OnPlayerTurn?.count ?? 1;
            duration = `Duration: ${count} Turn Rotation${count > 1 ? "s" : ""}`;

        } else if ("onRoundEnd" in mod.expiry || "OnRoundEnd" in mod.expiry) {
            const count = mod.expiry.onRoundEnd?.count ?? mod.expiry.OnRoundEnd?.count ?? 1;
            duration = `Duration: ${count} Round${count > 1 ? "s" : ""}`;

        } else if ("onHandEnd" in mod.expiry || "OnHandEnd" in mod.expiry) {
            const count = mod.expiry.onHandEnd?.count ?? mod.expiry.OnHandEnd?.count ?? 1;
            duration = `Duration: ${count} Hand${count > 1 ? "s" : ""}`;
        }
    }

    return {
        id: `modifier-${typeKey}-${index}`,
        name,
        description,
        duration,
        imgSrc,
    };
}

export interface CardProps {
    card: CardView | null;
    cardType: "playing" | "special";
    selected?: boolean;
    onClick?: () => void;
    innerRef?: React.Ref<HTMLDivElement>;
    slotRef?: React.Ref<HTMLDivElement>;
    slotAttribute?: string;
}

export const Card: React.FC<CardProps> = ({
    card,
    cardType,
    selected = false,
    onClick,
    innerRef,
    slotRef,
    slotAttribute,
}) => {
    const hasCard = Boolean(card);
    const className = cardType === "special"
        ? [styles.specialCardSlot, hasCard ? styles.hasCard : ''].filter(Boolean).join(' ')
        : styles.cardSlot;
    const isFaceUp = card?.visibility === "visible";

    return (
        <div
            ref={slotRef}
            data-slot-attr={slotAttribute}
            className={[className, selected ? styles.selectedSpecialCard : ""].filter(Boolean).join(" ")}
            onClick={onClick}
            role={onClick ? "button" : undefined}
            tabIndex={onClick ? 0 : undefined}
            onKeyDown={event => {
                if (onClick && (event.key === "Enter" || event.key === " ")) {
                    event.preventDefault();
                    onClick();
                }
            }}
        >
            <div
                ref={innerRef}
                className={[styles.cardInner, isFaceUp ? styles.cardFlipped : ""].filter(Boolean).join(" ")}
            >
                {card && (
                    <CardFaces
                        card={card}
                        cardType={cardType}
                    />
                )}
            </div>
        </div>
    );
};

interface CardFacesProps {
    card: CardView;
    cardType: "playing" | "special";
}

export const CardFaces: React.FC<CardFacesProps> = ({ card, cardType }) => {
    const cardValue = "value" in card ? card.value : null;
    const playingInfo = cardValue && cardType === "playing" ? parseCardString(cardValue) : null;
    const isJoker = cardType === "playing" && cardValue && cardValue.startsWith("*");

    return (
        <>
            <div
                className={[
                    styles.cardBackFace,
                    cardType === "special" ? styles.specialBackFace : styles.playingBackFace]
                    .filter(Boolean).join(" ")}
            >
                {cardType === "special" && (
                    <img src={getAssetUrl("/src/assets/special/special.png")} alt="" draggable={false} />
                )}
            </div>

            <div
                className={[styles.cardFront, cardType === "playing" && isJoker ? styles.joker : ""].filter(Boolean).join(" ")}
                style={cardType === "playing" ? { color: playingInfo?.color } : undefined}
            >
                {cardType === "special" && (
                    <div
                        style={{ display: "contents" }}
                        dangerouslySetInnerHTML={{
                            __html: buildSpecialCardFrontHTML(cardValue),
                        }}
                    />
                )}

                {cardType === "playing" && playingInfo && !isJoker && (
                    <>
                        <span>{playingInfo?.rank}</span>
                        <img
                            src={getAssetUrl(`/src/assets/suits/${playingInfo?.suitSymbol}.png`)}
                            alt=""
                            draggable={false}
                        />
                    </>
                )}

                {cardType === "playing" && isJoker && (
                    <>
                        <span style={{ transform: 'rotate(90deg)', left: '-15px', top: '16px', position: 'absolute' }}>Joker</span>
                        <span style={{ transform: 'rotate(270deg)', right: '-15px', bottom: '16px', position: 'absolute' }}>Joker</span>
                        <img src={getAssetUrl('/src/assets/suits/heartsmall.png')} />
                        <img src={getAssetUrl('/src/assets/suits/spadesmall.png')} />
                        <img src={getAssetUrl('/src/assets/suits/diamondsmall.png')} />
                        <img src={getAssetUrl('/src/assets/suits/clubsmall.png')} />
                    </>
                )}
            </div>
        </>
    );
};

export const PlayingCard: React.FC<Omit<CardProps, "cardType">> = props => (<Card {...props} cardType="playing" />);
export const SpecialCard: React.FC<Omit<CardProps, "cardType">> = props => (<Card {...props} cardType="special" />);
