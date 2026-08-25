//! Ty Lee — a melee chi-blocker built on the same primitives as Wan: named
//! buffs as counters, native effects referenced from `.data_champion`, and a
//! slice of the per-tick match hook for the parts the data layer cannot do.

pub mod effects;
pub mod player_ai;
pub mod tick;

// ------------------------------------------------ Defaults
pub const CHAMPION_KEY: &str = "ty_lee";

// ------------------------------------------------ Chi Blocking (base attack)
pub const ATTACK_AD_RATIO: usize = 100;
/// Mirrors `attack.range` in `.data_champion`. The AI needs to know what she
/// can hit without walking first, and it cannot read the data file - keep the
/// two in step.
pub const ATTACK_RANGE: u64 = 17_000;
pub const CHI_BLOCK_BUFF: &str = "ty_lee_chi_block";
/// Stacks fall off if she stops hitting the same target.
pub const CHI_BLOCK_DURATION: usize = 6 * 60;
pub const CHI_BLOCK_MAX_STACKS: usize = 4;
pub const CHI_BLOCK_BONUS_MAGIC: usize = 85;
pub const CHI_BLOCK_STUN_TICKS: u64 = 60;
/// One VFX buff per visible stack count, rendered by the `view_buffs` table in
/// `.data_champion`. There is no entry for a full set because the hit that
/// completes it consumes the stacks in the same breath, so the count on a unit
/// only ever reads 1, 2 or 3 - change `CHI_BLOCK_MAX_STACKS` and this stops
/// compiling until the sheet grows a frame to match.
pub const CHI_BLOCK_VFX_BUFFS: [&str; CHI_BLOCK_MAX_STACKS - 1] = [
    "ty_lee_chi_block_1",
    "ty_lee_chi_block_2",
    "ty_lee_chi_block_3",
];
/// The set breaking. One-shot, so unlike the stack marks it is timed rather
/// than reconciled. Its own length, not the stun's: the burst is a flourish on
/// the hit and wants to stay punchy, so a longer stun leaves the target held
/// after it has played rather than stretching it out. Keep it equal to the
/// sheet's 4 frames x 0.125s or the last frame hangs.
pub const CHI_BLOCK_BREAK_VFX_BUFF: &str = "ty_lee_chi_block_break";
pub const CHI_BLOCK_BREAK_VFX_TICKS: usize = 30;

// ------------------------------------------------ Three Point Strike
pub const STRIKE_COUNT: usize = 3;
pub const STRIKE_INTERVAL: usize = 15;
pub const STRIKE_DAMAGE: usize = 40;
pub const STRIKE_AD_RATIO: usize = 40;
pub const STRIKE_CRIT_BONUS: usize = 20;

// ------------------------------------------------ Lightfooted
/// She dashes to whatever is closest, either team. The player AI makes the
/// pick and a `Rush` in `.data_champion` carries her - the same pairing
/// Balancing Act uses, because a native effect cannot move its own caster.
pub const LIGHTFOOTED_RANGE: u64 = 40_000;
/// Carries `base_attack_damaged_reduce`, the engine's own basic-attack
/// mitigation, so the dodge needs no hook on the way in. Taking it back off
/// does need one - see `tick`.
pub const LIGHTFOOTED_BUFF: &str = "ty_lee_lightfooted";
pub const LIGHTFOOTED_DURATION: usize = 4 * 60;
pub const LIGHTFOOTED_SHIELD: usize = 100;
pub const LIGHTFOOTED_SHIELD_AD_RATIO: usize = 20;
pub const LIGHTFOOTED_SHIELD_HP_RATIO: usize = 10;
/// Full mitigation: the ability is written as dodging basic attacks outright.
pub const LIGHTFOOTED_DODGE: usize = 100;

// ------------------------------------------------ Balancing Act
pub const BALANCE_RANGE: u64 = 120_000;
pub const BALANCE_DAMAGE: usize = 220;
pub const BALANCE_AD_RATIO: usize = 80;
/// Crit chance is a flat damage term here, not a chance to do more: the ult
/// hits for her crit chance on top of the rest, whether or not it rolls.
pub const BALANCE_CRIT_RATIO: usize = 100;
/// Everything caught around where she lands takes the hit; champions among them
/// are silenced. She lands on her target, so the area is centred on her.
pub const BALANCE_AOE_RADIUS: u64 = 30_000;
/// The dash itself is not here: a native effect's `linear_move_speed` does not
/// move a caster, so what carries her is a `MoveToTarget` in the ult's effect
/// list in `.data_champion` - the same effect Gragas' Body Slam dashes on. It
/// goes to the target rather than a fixed distance, so she stops on them
/// instead of sailing past anything closer than `BALANCE_RANGE`, and a
/// `CasterAnimation` beside it plays the sheet's `ult_dash` tag over the trip.
/// A second one in the `end_effects` swaps that for `ult_impact` - the spin the
/// AoE lands on - which is why the action's `duration` has to cover the longest
/// dash plus that burst rather than the dash alone.
pub const BALANCE_SILENCE_TICKS: u64 = 90;
/// Marker carried while the dash is still carrying her in, naming the target it
/// owes: `ty_lee_balancing_act_inbound|<entity id>`. The hit lands from the
/// `MoveToTarget`'s own `end_effects`, which fire on arrival however far she
/// travelled - this is how that half knows who she dashed at, rather than
/// re-picking from wherever she ended up.
pub const BALANCE_INBOUND_PREFIX: &str = "ty_lee_balancing_act_inbound";
/// Dropped unfired if she never arrives - a stopped dash owes nothing. Well
/// clear of the longest trip (`BALANCE_RANGE` / speed 4000 = 30 ticks), since
/// expiring early would silently cost her the whole hit.
pub const BALANCE_INBOUND_TIMEOUT: usize = 90;
pub const BALANCE_MARK_BUFF: &str = "ty_lee_balancing_act";
/// The crit the mark is cashed in for. The engine's own crit multiplier is not
/// exposed anywhere - not in the settings, not in any ability text - so rather
/// than invent one, the marked attack borrows the engine's by wearing this much
/// crit chance while it lands.
///
/// It rides a buff of its own, put on and taken off inside the same call as the
/// attack, so it exists for that one `deal_damage` and nothing else of hers can
/// see it. The mark itself carries no stats: as a stat buff it would have sat
/// on her for eight seconds, and everything else that reads her crit chance -
/// Three Point Strike's roll, the ult's own crit-scaled damage - would have
/// been riding it too.
pub const BALANCE_MARK_CRIT: usize = 100;
pub const BALANCE_MARK_CRIT_BUFF: &str = "ty_lee_balancing_act_crit";
pub const BALANCE_MARK_DURATION: usize = 8 * 60;
/// Chi Block stacks the marked attack applies, in place of the usual one.
pub const BALANCE_MARK_CHI_BLOCKS: usize = 2;
