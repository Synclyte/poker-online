import React from 'react';
import styles from '../../pages/Game/game.module.css';
import { CardView, SpecialCardInfo, ActiveModifierInfo, PlayerState, PlayerLobbyInfo } from '../../pages/Game/game.types';
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
        "d": { symbol: "diamond", color: "#e53935" },
    };

    const displayRank = rawRank.toUpperCase();
    const suitInfo = suitMap[rawSuit] || { symbol: "", color: "#000000" };

    return {
        isHidden: false,
        rank: displayRank,
        suitSymbol: suitInfo.symbol,
        color: suitInfo.color
    };
}

export function formatCardName(card?: CardView | null): string {
    if (!card || !("value" in card) || !card.value || card.value === "Unmarked") {
        return "Blank Card";
    }
    if (card.value.startsWith("*")) {
        return "Joker";
    }

    const parts = card.value.split(":");
    const rawRank = (parts[0] || "").toLowerCase();
    const rawSuit = (parts[1] || "").toLowerCase();

    const rankNames: Record<string, string> = {
        "2": "2",
        "3": "3",
        "4": "4",
        "5": "5",
        "6": "6",
        "7": "7",
        "8": "8",
        "9": "9",
        "10": "10",
        "j": "Jack",
        "q": "Queen",
        "k": "King",
        "a": "Ace",
    };
    const suitNames: Record<string, string> = {
        "s": "Spades",
        "c": "Clubs",
        "h": "Hearts",
        "d": "Diamonds",
    };

    const rankName = rankNames[rawRank];
    if (!rankName) {
        return "Blank Card";
    }
    const suitName = suitNames[rawSuit];
    return suitName ? `${rankName} of ${suitName}` : rankName;
}

export const specialCardInfo: Record<string, SpecialCardInfo> = {
    replace_card_self: {
        label: "Replace Hole Card",
        description: "Replaces a selected hole card with a random new card, drawn from the deck",
        target: "selfCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_replace_card_self.png",
    },
    draw_card_self: {
        label: "Draw Hole Card",
        description: "Draws an additional hole card from the deck. This card is visible to opponents",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_card_self.png",
    },
    draw_card_community: {
        label: "Draw Community Card",
        description: "Draws an additional card from the deck, adding it to community cards",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_card_community.png",
    },
    replace_card_community: {
        label: "Replace Community",
        description: "Replace a selected community card with a new card from the deck",
        target: "communityCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_replace_card_community.png",
    },
    remove_card_community: {
        label: "Remove Community",
        description: "Removes a selected community card",
        target: "communityCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_remove_card_community.png",
    },
    reveal_card: {
        label: "Reveal Card",
        description: "Reveals a chosen hole card of a selected opponent to you until the end of the current hand",
        target: "otherCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_reveal_opponent_card.png",
    },
    withdraw_bet: {
        label: "Withdraw Bet",
        description: "Fold recovering your bet from the pot",
        target: "none",
        imgSrc: "/src/assets/special/special_withdraw_bet.png",
    },
    ante_up: {
        label: "Ante Up",
        description: "Doubles the blind and minimum raise costs permanently. Grants a Chip Boost special card on use",
        target: "none",
        imgSrc: "/src/assets/special/special_ante_up.png",
    },
    pot_mult: {
        label: "Pot Multiplier",
        description: "Multiplies player winnings from the pot this hand by 1.5x",
        target: "none",
        imgSrc: "/src/assets/special/special_pot_mult.png",
    },
    raise_block: {
        label: "Raise Block",
        description: "Prevents raises by all players until your next turn",
        target: "none",
        imgSrc: "/src/assets/special/special_raise_block.png",
    },
    special_block: {
        label: "Special Block",
        description: "Prevent all special cards from being played until your next turn",
        target: "none",
        imgSrc: "/src/assets/special/special_special_block.png",
    },
    chip_boost: {
        label: "Chip Boost",
        description: "Gain chips proportional to the blind cost",
        target: "none",
        imgSrc: "/src/assets/special/special_chip_boost.png",
    },
    chip_gamble: {
        label: "Chip Gamble",
        description: "When used, has a chance to give a significant number of chips. If it fails, lose half of your chips and fold",
        target: "none",
        imgSrc: "/src/assets/special/special_chip_gamble.png",
    },
    blackjack: {
        label: "Blackjack",
        description: "Changes scoring system to Blackjack for the current hand. Provides a Draw Hole Card special card to all players on use",
        target: "none",
        imgSrc: "/src/assets/special/special_blackjack.png",
    },
    draw_heart: {
        label: "Draw Heart",
        description: "Forces the next drawn card to have the Heart suit",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_heart.png",
    },
    draw_spade: {
        label: "Draw Spade",
        description: "Forces the next drawn card to have the Spade suit",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_spade.png",
    },
    draw_diamond: {
        label: "Draw Diamond",
        description: "Forces the next drawn card to have the Diamond suit",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_diamond.png",
    },
    draw_club: {
        label: "Draw Club",
        description: "Forces the next drawn card to have the Club suit",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_club.png",
    },
    draw_face: {
        label: "Draw Face",
        description: "Forces the next drawn card to be a face card",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_face.png",
    },
    draw_high: {
        label: "Draw High",
        description: "Forces the next drawn card to be a 10 or above",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_high.png",
    },
    draw_low: {
        label: "Draw Low",
        description: "Forces the next drawn card to be a 5 or below",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_low.png",
    },
    invalidate_flush: {
        label: "No Flushes",
        description: "Invalidates Flushes for this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_invalidate_flush.png",
    },
    invalidate_straight: {
        label: "No Straights",
        description: "Invalidates Straights for this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_invalidate_straight.png",
    },
    invalidate_three_of_a_kind: {
        label: "No Three of a Kind",
        description: "Invalidates Three of a Kinds for this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_invalidate_three_of_a_kind.png",
    },
    invalidate_two_pair: {
        label: "No Two Pair",
        description: "Invalidates Two Pairs for this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_invalidate_two_pair.png",
    },
    invalidate_full_house: {
        label: "No Full House",
        description: "Invalidates Full Houses for this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_invalidate_full_house.png",
    },
    special: {
        label: "Special",
        description: "Small chance to obtain when using a special card. Gives multiple special cards on use, exceeding the typical special card limit",
        target: "none",
        imgSrc: "/src/assets/special/special_special.png",
    },
    draw_higher_than_last: {
        label: "Draw Higher",
        description: "Forces the next drawn card to have a rank equal to or higher than the previous draw",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_higher_than_last.png",
    },
    draw_lower_than_last: {
        label: "Draw Lower",
        description: "Forces the next drawn card to have a rank equal to or lower than the previous draw",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_lower_than_last.png",
    },
    draw_same_suit_as_last: {
        label: "Draw Same Suit",
        description: "Forces the next drawn card to be the same suit as the previous draw",
        target: "none",
        imgSrc: "/src/assets/special/special_draw_same_suit_as_last.png",
    },
    discard: {
        label: "Discard",
        description: "Small chance to obtain when discarding a special card. Prevents any more community cards from being drawn for the rest of this hand",
        target: "none",
        imgSrc: "/src/assets/special/special_discard.png",
    },
    hand_swap: {
        label: "Hand Swap",
        description: "Swaps your hand with that of a chosen opponent. Fails if their hand is currently better than yours",
        target: "other",
        imgSrc: "/src/assets/special/special_hand_swap.png",
    },
    joker: {
        label: "Joker",
        description: "Converts a selected community card into a Joker",
        target: "communityCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_joker.png",
    },
    special_spread: {
        label: "Special Spread",
        description: "Deals 1 special card to all players",
        target: "none",
        imgSrc: "/src/assets/special/special_special_spread.png",
    },
    rank_boost: {
        label: "Rank Boost",
        description: "Increases the rank of a selected hole or community card by 1",
        target: "selfOrCommunityCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_rank_boost.png",
    },
    suit_match: {
        label: "Suit Match",
        description: "Changes a selected hole card's suit to match a random other card in your hand",
        target: "selfCard",
        requiresTargetIndex: true,
        imgSrc: "/src/assets/special/special_suit_match.png",
    },
    redraw_hand: {
        label: "Redraw Hand",
        description: "Redraws all of the cards in your hand",
        target: "none",
        imgSrc: "/src/assets/special/special_redraw_hand.png",
    },
    loss_insurance: {
        label: "Loss Insurance",
        description: "Refunds 40% of chips spent by you this hand on round loss",
        target: "none",
        imgSrc: "/src/assets/special/special_loss_insurance.png",
    },
    bet_rebate: {
        label: "Bet Rebate",
        description: "Refunds 20% of all chips spent by you in the next 3 hands after the effect expires",
        target: "none",
        imgSrc: "/src/assets/special/special_bet_rebate.png",
    },
    chip_stipend: {
        label: "Chip Stipend",
        description: "Gain 1x blind size at the end of each hand for 4 hands, scaling with ante",
        target: "none",
        imgSrc: "/src/assets/special/special_chip_stipend.png",
    },
    draw: {
        label: "Draw",
        description: "Small chance to obtain when drawing a hole card. When used, deals 3 hole cards to you, 1 hole card to each opponent, and 1 community card. All drawn cards are revealed to everyone",
        target: "none",
        imgSrc: "/src/assets/special/special_draw.png",
    },
    extra_slot: {
        label: "Extra Slot",
        description: "Permanently increases your special card capacity by 1 for the rest of the game",
        target: "none",
        imgSrc: "/src/assets/special/special_extra_slot.png",
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

    const baseInfo = specialCardInfo[cardStr] ?? {
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

    if (cardStr === "chip_boost") {
        const boostChips = Math.round(bSize * aMult * 2.5);
        finalInfo = {
            ...finalInfo,
            description: `Gain +${boostChips} chips on use, scaling with blind cost`,
        };
    } else if (cardStr === "chip_gamble") {
        const gambleChips = Math.round(bSize * aMult * 7.5);
        const chance = gambleSuccessChance ?? 0.9;
        const successChance = Math.round(chance * 100);
        finalInfo = {
            ...finalInfo,
            description: `Results in one of two outcomes. ${successChance}% chance to gain +${gambleChips} chips, scaling with blind cost. ${100 - successChance}% chance to force you to fold and lose half of your chips`,
        };
    } else if (cardStr === "chip_stipend") {
        const stipendChips = Math.round(bSize * aMult);
        finalInfo = {
            ...finalInfo,
            description: `Gain +${stipendChips} chips at the end of each hand for 4 hands, scaling with blind cost`,
        };
    }

    return finalInfo;
}

export function buildSpecialCardFrontHTML(cardValue?: string | null): string {
    const info = cardValue ? getSpecialCardInfo(cardValue) : null;
    const imgSrc = info?.imgSrc ?? "";
    const imgHtml = imgSrc
        ? `<img src="${imgSrc}" alt="${info?.label ?? ''}" class="${styles.specialImage}" draggable="false" />`
        : `<span>${info?.label ?? "Special"}</span>`;

    const smallMark = getAssetUrl("/src/assets/special/specialsmall.png");
    return `
        <img style="top: 1px; left: 1px" class="${styles.specialMark}" src="${smallMark}" draggable="false" />
        <img style="top: 1px; right: 1px" class="${styles.specialMark}" src="${smallMark}" draggable="false" />
        ${imgHtml}
        <img style="bottom: 1px; left: 1px" class="${styles.specialMark}" src="${smallMark}" draggable="false" />
        <img style="bottom: 1px; right: 1px" class="${styles.specialMark}" src="${smallMark}" draggable="false" />
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
    royal_flush: "Royal Flush",
    straight_flush: "Straight Flush",
    four_of_a_kind: "Four of a Kind",
    full_house: "Full House",
    flush: "Flush",
    straight: "Straight",
    three_of_a_kind: "Three of a Kind",
    two_pair: "Two Pair",
    pair: "Pair",
    high_card: "High Card",
    none: "Nothing",
};

export function getSanitisedHandName(internalName: string): string {
    return handNameMap[internalName] ?? internalName;
}

export interface ModifierParseContext {
    myId?: number | null;
    players?: PlayerState[];
    lobbyPlayers?: PlayerLobbyInfo[];
}

export function parseModifier(
    mod: any,
    index: number,
    blindSize?: number,
    anteMult?: number,
    potMult?: number,
    context?: ModifierParseContext
): ActiveModifierInfo {
    let name = "Active Modifier";
    let description = "An active game modifier.";
    let duration = "Active";
    let typeKey = "unknown";

    let imgSrc = getAssetUrl("/src/assets/special/special.png");

    if (typeof mod.effect === "string") {
        switch (mod.effect) {
            case "raises_blocked":
                name = "Raise Block";
                description = "Raises are prevented";
                typeKey = "raises_blocked";
                imgSrc = getSpecialCardInfo("raise_block").imgSrc ?? imgSrc;
                break;
            case "specials_blocked":
                name = "Special Block";
                description = "Special cards cannot be played";
                typeKey = "specials_blocked";
                imgSrc = getSpecialCardInfo("special_block").imgSrc ?? imgSrc;
                break;
            case "blackjack_scoring":
                name = "Blackjack Scoring";
                description = "Closest hand sum to 21 wins";
                typeKey = "blackjack_scoring";
                imgSrc = getSpecialCardInfo("blackjack").imgSrc ?? imgSrc;
                break;
        }
    } else if (mod.effect && typeof mod.effect === "object") {
        if ("pot_multiplier" in mod.effect) {
            const rawVal = mod.effect.pot_multiplier?.multiplier ?? potMult ?? 1.5;
            const val = Math.round(rawVal * 100) / 100;
            name = `Pot Multiplier (${val}x)`;
            description = `Multiplies all pot gains by ${val}x.`;
            typeKey = "pot_multiplier";
            imgSrc = getSpecialCardInfo("pot_mult").imgSrc ?? imgSrc;
        } else if ("ante_multiplier" in mod.effect) {
            const rawVal = mod.effect.ante_multiplier?.multiplier ?? 2.0;
            const val = Math.round(rawVal * 100) / 100;
            name = `Ante Multiplier (${val}x)`;
            description = `Multiplies minimum bets and blinds by ${val}x.`;
            typeKey = "ante_multiplier";
            imgSrc = getSpecialCardInfo("ante_up").imgSrc ?? imgSrc;
        } else if ("reveal_card" in mod.effect) {
            const data = mod.effect.reveal_card;
            const targetId: number | undefined = data?.target_id;
            const cardIndex: number | undefined = data?.card_index;
            const viewerId: number | undefined = data?.viewer_id;
            const getPlayerName = (id?: number) => {
                if (id === undefined) return "Player";
                return context?.lobbyPlayers?.find(p => p.id === id)?.name || `Player ${id}`;
            };
            const revealedCard =
                targetId !== undefined && cardIndex !== undefined
                    ? context?.players?.find(p => p.id === targetId)?.holeCards[cardIndex]
                    : undefined;
            const cardName = formatCardName(revealedCard);

            name = "Reveal Card";
            if (context?.myId != null && viewerId === context.myId) {
                description = `Revealed ${getPlayerName(targetId)}'s ${cardName}`;
            } else if (context?.myId != null && targetId === context.myId) {
                description = `Your ${cardName} was revealed by ${getPlayerName(viewerId)}`;
            } else if (targetId !== undefined && viewerId !== undefined) {
                description = `${getPlayerName(targetId)}'s ${cardName} was revealed by ${getPlayerName(viewerId)}`;
            } else {
                description = "Opponent card revealed";
            }
            typeKey = `reveal_card-${targetId ?? "x"}-${cardIndex ?? "x"}-${viewerId ?? "x"}`;
            imgSrc = getSpecialCardInfo("reveal_card").imgSrc ?? imgSrc;
        } else if ("loss_insurance" in mod.effect) {
            const rate = mod.effect.loss_insurance?.rebate_rate ?? 0.4;
            const pct = Math.round(rate * 100);
            name = `Loss Insurance (${pct}%)`;
            description = `Refunds ${pct}% of lost chips on fold or showdown loss`;
            typeKey = "loss_insurance";
            imgSrc = getSpecialCardInfo("loss_insurance").imgSrc ?? imgSrc;
        } else if ("bet_rebate" in mod.effect) {
            const data = mod.effect.bet_rebate;
            const rate = data?.rebate_rate ?? 0.2;
            const accumulated = data?.accumulated_bet ?? 0;
            const pct = Math.round(rate * 100);
            name = `Bet Rebate (${pct}%)`;
            description = `Refunds ${pct}% of chips spent while this modifier is active on expiry (${accumulated} chips total)`;
            typeKey = "bet_rebate";
            imgSrc = getSpecialCardInfo("bet_rebate").imgSrc ?? imgSrc;
        } else if ("chip_stipend" in mod.effect) {
            const chips = Math.round((blindSize ?? 20) * (anteMult ?? 1));
            name = `Chip Stipend (+${chips} chips)`;
            description = `Grants ${chips} chips at the end of each hand`;
            typeKey = "chip_stipend";
            imgSrc = getSpecialCardInfo("chip_stipend").imgSrc ?? imgSrc;
        } else if ("extra_slot" in mod.effect) {
            const extraSlots = mod.effect.extra_slot?.extra_slots ?? 1;
            name = `Extra Slot (+${extraSlots})`;
            description = `Increases your special card capacity by ${extraSlots}`;
            typeKey = "extra_slot";
            imgSrc = getSpecialCardInfo("extra_slot").imgSrc ?? imgSrc;
        } else if ("invalidate_hand" in mod.effect) {
            const rawHand = mod.effect.invalidate_hand ?? "none";
            const rawHandStr = typeof rawHand === "string" ? rawHand : String(rawHand ?? "none");
            const handName = getSanitisedHandName(rawHandStr);
            const pluralHandName = `${handName}${handName === "Flush" ? "es" : "s"}`;
            name = `${pluralHandName} Disabled`;
            description = `${pluralHandName} are disabled and cannot score`;
            typeKey = "invalidate_hand";
            imgSrc = getSpecialCardInfo(`invalidate_${rawHandStr}`).imgSrc ?? imgSrc;
        } else if ("force_draw" in mod.effect) {
            const rule = mod.effect.force_draw?.rule;
            const ruleStr = typeof rule === "string" ? rule : String(rule ?? "");

            imgSrc = getSpecialCardInfo(`draw_${ruleStr}`).imgSrc ?? imgSrc;

            switch (ruleStr) {
                case "heart":
                    name = "Draw Heart";
                    description = "Forces the next card draw to be a Heart";
                    typeKey = "draw_heart";
                    break;
                case "spade":
                    name = "Draw Spade";
                    description = "Forces the next card draw to be a Spade";
                    typeKey = "draw_spade";
                    break;
                case "diamond":
                    name = "Draw Diamond";
                    description = "Forces the next card draw to be a Diamond";
                    typeKey = "draw_diamond";
                    break;
                case "club":
                    name = "Draw Club";
                    description = "Forces the next card draw to be a Club";
                    typeKey = "draw_club";
                    break;
                case "face":
                    name = "Draw Face Card";
                    description = "Forces the next card draw to be a face card";
                    typeKey = "draw_face";
                    break;
                case "high":
                    name = "Draw High Card";
                    description = "Forces the next card draw to have a rank of 10 or higher";
                    typeKey = "draw_high";
                    break;
                case "low":
                    name = "Draw Low Card";
                    description = "Forces the next card draw to have a rank of 5 or lower";
                    typeKey = "draw_low";
                    break;
                case "higher_than_last":
                    name = "Draw Higher";
                    description = "Forces the next card draw to have a rank equal to or higher than the previous draw";
                    typeKey = "draw_higher_than_last";
                    break;
                case "lower_than_last":
                    name = "Draw Lower";
                    description = "Forces the next card draw to have a rank equal to or lower than the previous draw";
                    typeKey = "draw_lower_than_last";
                    break;
                case "same_suit_as_last":
                    name = "Draw Same Suit";
                    description = "Forces the next card draw to have the same suit as the previous draw";
                    typeKey = "draw_same_suit_as_last";
                    break;
                case "nothing":
                    name = "Draw Blank";
                    description = "Forces the next card draw to be a blank card";
                    typeKey = "draw_nothing";
                    imgSrc = getSpecialCardInfo("discard").imgSrc ?? imgSrc;
                    break;
                default:
                    name = `Draw ${ruleStr}`;
                    description = `Draws follow the ${ruleStr} rule`;
                    typeKey = `draw_${ruleStr}`;
                    break;
            }
        }
    }

    if (mod.expiry === "on_game_end") {
        duration = "Duration: Game";
    } else if (mod.expiry && typeof mod.expiry === "object") {
        if ("on_card_draw" in mod.expiry) {
            const count = mod.expiry.on_card_draw?.count ?? 1;
            duration = `Duration: ${count} Draw${count > 1 ? "s" : ""}`;
        } else if ("on_player_turn" in mod.expiry) {
            const count = mod.expiry.on_player_turn?.count ?? 1;
            duration = `Duration: ${count} Turn Rotation${count > 1 ? "s" : ""}`;
        } else if ("on_round_end" in mod.expiry) {
            const count = mod.expiry.on_round_end?.count ?? 1;
            duration = `Duration: ${count} Round${count > 1 ? "s" : ""}`;
        } else if ("on_hand_end" in mod.expiry) {
            const count = mod.expiry.on_hand_end?.count ?? 1;
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
    const specialInfo = cardValue && cardType === "special" ? getSpecialCardInfo(cardValue) : null;
    const specialSmallMark = cardType === "special" ? getAssetUrl("/src/assets/special/specialsmall.png") : "";
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
                    <>
                        <img style={{ top: 1, left: 1 }} className={styles.specialMark} src={specialSmallMark} alt="" draggable={false} />
                        <img style={{ top: 1, right: 1 }} className={styles.specialMark} src={specialSmallMark} alt="" draggable={false} />
                        {specialInfo?.imgSrc ? (
                            <img src={specialInfo.imgSrc} alt={specialInfo.label} className={styles.specialImage} draggable={false} />
                        ) : (
                            <span>{specialInfo?.label ?? "Special"}</span>
                        )}
                        <img style={{ bottom: 1, left: 1 }} className={styles.specialMark} src={specialSmallMark} alt="" draggable={false} />
                        <img style={{ bottom: 1, right: 1 }} className={styles.specialMark} src={specialSmallMark} alt="" draggable={false} />
                    </>
                )}

                {cardType === "playing" && playingInfo && !isJoker && (
                    <>
                        <span>{playingInfo?.rank}</span>
                        {playingInfo.suitSymbol && (
                            <img
                                src={getAssetUrl(`/src/assets/suits/${playingInfo.suitSymbol}.png`)}
                                alt=""
                                draggable={false}
                            />
                        )}
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
