//! Ty Lee — a melee chi-blocker built on the same primitives as Wan: named
//! buffs as counters, native effects referenced from `.data_champion`, and a
//! slice of the per-tick match hook for the parts the data layer cannot do.
//!
//! ## What the in-game database editor can and cannot reach
//!
//! Her numbers are compiled in, here and in `effects.rs`, and the editor
//! cannot reach them. Three rules, established by reading the editor back
//! against deliberate edits and checked against the 0.5.6 engine, are why:
//!
//! 1. A native effect is handed only its `effect_ref`. The engine's own
//!    `DataEffectDef::Native` carries 41 numeric params beside it - `damage`,
//!    `attack_ratio`, `shield`, `mark_bonus_attack_ratio` and so on - but they
//!    are constructor arguments for the engine's *built-in* effects. A mod
//!    registers one already-built instance per `effect_ref` at load, before any
//!    champion data is parsed, and `apply` has no parameter to carry them.
//! 2. The editor addresses a number by field name, one row per name per
//!    action. The action's own names - `duration`, `cooltime`, `start_timing`,
//!    `range` - win outright; any other name goes to the first node in the
//!    effect tree that owns it. A `Native` owns all 41 of its names at their
//!    `0` defaults, so it masks those names on every node behind it - though
//!    not on one ahead of it, which is why Wan's Spirit Step dash speed shows.
//! 3. Only names with a label render at all: `database_edit.number_field` for
//!    a node's own fields, `database_edit.number_buff_field` for the stats on
//!    a buff.
//!
//! Moving her numbers into data nodes ahead of the natives was tried, and it
//! broke abilities in play: a buff or shield node in a cast aimed by
//! direction, or not aimed at all, is handed no target and silently does
//! nothing. So the kit stays native, Chi Block's stun included - see
//! `CHI_BLOCK_HOLD`.

pub mod effects;
pub mod player_ai;
pub mod tick;

// ------------------------------------------------ Defaults
pub const CHAMPION_KEY: &str = "ty_lee";

// ------------------------------------------------ Action lengths
/// How long each action occupies her, mirroring that action's `duration` in
/// `.data_champion`. The combo sequencer in `player_ai` has no way to ask the
/// engine whether a cast is still running, so it holds its own next step off
/// for this long after issuing one - keep these in step with the data file or
/// she talks over her own casts.
pub const ATTACK_ACTION_TICKS: usize = 24;
pub const STRIKE_ACTION_TICKS: usize = 45;
pub const LIGHTFOOTED_ACTION_TICKS: usize = 12;
pub const BALANCE_ACTION_TICKS: usize = 44;

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
pub const CHI_BLOCK_BONUS_MAGIC: usize = 60;
/// The bonus is magic damage, but it scales off her AD.
pub const CHI_BLOCK_BONUS_AD_RATIO: usize = 15;
/// 1.5 seconds.
pub const CHI_BLOCK_STUN_TICKS: usize = 90;
/// What a broken set holds the target with, every kind for the full
/// `CHI_BLOCK_STUN_TICKS`.
///
/// A stun from code is not a full stun. `apply_cc` with `Stun` alone holds the
/// target in place like a root and the engine draws no stars over it; only a
/// data `Stun` node gets the engine's own stun, and a data node cannot be
/// switched on by anything code decides (a `SwitchByBuff` does not see buffs
/// code puts on). So the hold is built from the kinds `apply_cc` does land -
/// the ult's silence is the same `BlockSkill` and `BlockMoveSkill` - and
/// between them nothing is left that the target can do.
pub const CHI_BLOCK_HOLD: [mod_api_stable::CcKindV1; 4] = [
    mod_api_stable::CcKindV1::Stun,
    mod_api_stable::CcKindV1::BlockAttack,
    mod_api_stable::CcKindV1::BlockSkill,
    mod_api_stable::CcKindV1::BlockMoveSkill,
];
/// The base game's stun stars, redrawn as a view buff of hers by
/// `tools/gen_ty_lee_stun.py`, since the engine draws none over the hold.
/// Timed to the hold, so the sheet's spin and the hold end together; change
/// `CHI_BLOCK_STUN_TICKS` and regenerate it.
pub const CHI_BLOCK_STUN_VFX_BUFF: &str = "ty_lee_chi_block_stun";
/// Put on a target when its set breaks. While it is up, Chi Block cannot stack
/// on that target again - the passive's per-target cooldown.
pub const CHI_BLOCK_COOLDOWN_BUFF: &str = "ty_lee_chi_block_cooldown";
pub const CHI_BLOCK_COOLDOWN_TICKS: usize = 6 * 60;
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

// ------------------------------------------------ Three Point Strike
pub const STRIKE_COUNT: usize = 3;
/// The strikes after the first are queued from code this far apart.
pub const STRIKE_INTERVAL: usize = 15;
pub const STRIKE_DAMAGE: usize = 60;
pub const STRIKE_AD_RATIO: usize = 60;
/// Crit chance is a flat damage term here, as it is on the ult: each strike
/// hits for this share of her crit chance on top of the rest, with nothing
/// rolled.
pub const STRIKE_CRIT_RATIO: usize = 60;

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
pub const LIGHTFOOTED_SHIELD_AD_RATIO: usize = 50;
pub const LIGHTFOOTED_SHIELD_HP_RATIO: usize = 10;
/// Full mitigation: the ability is written as dodging basic attacks outright.
pub const LIGHTFOOTED_DODGE: usize = 100;

// ------------------------------------------------ Balancing Act
pub const BALANCE_RANGE: u64 = 120_000;
pub const BALANCE_DAMAGE: usize = 220;
pub const BALANCE_AD_RATIO: usize = 110;
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
/// dash plus that burst rather than the dash alone. A `CasterViewEffect` beside
/// it plays the ring going out from where she lands, which is drawn to
/// `BALANCE_AOE_RADIUS` by `tools/gen_ty_lee_ult_impact.py` - change the two
/// together.
pub const BALANCE_SILENCE_TICKS: u64 = 120;
/// Marker carried while the dash is still carrying her in, naming the target it
/// owes: `ty_lee_balancing_act_inbound|<entity id>`. The hit lands from the
/// `MoveToTarget`'s own `end_effects`, which fire on arrival however far she
/// travelled - this is how that half knows who she dashed at, rather than
/// re-picking from wherever she ended up.
pub const BALANCE_INBOUND_PREFIX: &str = "ty_lee_balancing_act_inbound";
/// Dropped unfired if she never arrives - a stopped dash owes nothing. Well
/// clear of the longest trip (`BALANCE_RANGE` / speed 6000 = 20 ticks), since
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
