//! Toph Beifong — a melee earthbender who marks what she hits and cashes the
//! marks in with everything else. Built on the same primitives as Ty Lee -
//! named buffs as counters, and a slice of the per-tick match hook for the
//! parts an effect cannot see - but unlike Wan and Ty Lee she carries no
//! `.data_champion` at all: `champion.rs` is her definition, and `vfx.rs`
//! draws what `view_buffs` used to.
//!
//! Every number the JSON held now lives here, which is the point of the port:
//! the action shapes and the effect constants they feed are finally in one
//! file, and the comments that used to say "keep the two in step" have nothing
//! left to keep step with.

use mod_api_stable::StatV1;

pub mod champion;
pub mod effects;
/// Built but not registered while the progression freeze is being chased -
/// see the note in `lib.rs`. Kept compiling so putting it back is one line.
#[allow(dead_code)]
pub mod player_ai;
pub mod tick;
pub mod vfx;

// ------------------------------------------------ Defaults
pub const CHAMPION_KEY: &str = "toph";

/// Level 1, and what each level adds. She is built to be stood in front of:
/// middling attack, real health and armor, and the magic power that everything
/// but the basic attack scales off arriving faster than anything else.
pub const BASE_STAT: StatV1 = StatV1 {
    attack: 68,
    magic_power: 10,
    hp: 1010,
    defence: 32,
    magic_resistance: 22,
    move_speed: 950,
    hp_regen: 0,
    stack: 0,
    crit_chance: 0,
};

pub const GROWTH_STAT: StatV1 = StatV1 {
    attack: 6,
    magic_power: 11,
    hp: 112,
    defence: 9,
    magic_resistance: 4,
    move_speed: 3,
    hp_regen: 0,
    stack: 0,
    crit_chance: 0,
};

// ------------------------------------------------ Action shapes
/// How long each cast occupies her, when in that window the effect fires, and
/// what it costs. `start_timing` is an offset into the animation, so it is the
/// frame the swing connects on rather than a delay of its own.
///
/// `METAL_ACTION_TICKS` is the one the AI reads as well as the engine: it has
/// no way to ask whether a cast is still running, so after issuing the recast
/// itself it goes quiet for exactly this long rather than issuing into an
/// action already playing. It is the only cast this mod ever starts on its
/// own; everything else the AI does is a re-aim of one the base AI had already
/// decided to make.
pub const ATTACK_ACTION_TICKS: usize = 24;
pub const ATTACK_START_TIMING: usize = 8;
pub const ATTACK_COOLTIME: usize = 60;

pub const PILLAR_ACTION_TICKS: usize = 36;
pub const PILLAR_START_TIMING: usize = 12;
pub const PILLAR_COOLTIME: usize = 480;

pub const METAL_ACTION_TICKS: usize = 18;
pub const METAL_START_TIMING: usize = 1;
pub const METAL_COOLTIME: usize = 720;
/// Two casts before the cooldown starts - the engine has no recast of its own,
/// so this is what pays for one. See `METAL_ARMED_BUFF`.
pub const METAL_USE_COUNT: usize = 2;

pub const BANDIT_ACTION_TICKS: usize = 48;
pub const BANDIT_START_TIMING: usize = 4;
pub const BANDIT_COOLTIME: usize = 3600;

// ------------------------------------------------ Seismic Sense (base attack)
pub const ATTACK_AD_RATIO: usize = 100;
/// What she can hit without walking first. The AI steers by it too, which is
/// why it sits with the mark rather than with the action shape above.
pub const ATTACK_RANGE: u64 = 17_000;
/// The mark itself. Champions only: a mark is a promise of a second, bigger
/// hit, and a minion rarely lives to collect it.
pub const MARK_BUFF: &str = "toph_seismic_mark";
pub const MARK_DURATION: usize = 3 * 60;
pub const MARK_BONUS_MAGIC: usize = 80;
pub const MARK_BONUS_AP_RATIO: usize = 8;
/// The mark going off. One-shot, so unlike the mark itself it is timed rather
/// than reconciled. Keep it equal to the sheet's 4 frames x 0.1s or the last
/// frame hangs.
pub const MARK_POP_VFX_BUFF: &str = "toph_seismic_mark_pop";
pub const MARK_POP_VFX_TICKS: usize = 24;

// ------------------------------------------------ Rock Pillar
/// She bends the pillar up under someone rather than reaching them, so it
/// outranges her fist by a good way.
pub const PILLAR_RANGE: u64 = 45_000;
pub const PILLAR_DAMAGE: usize = 50;
pub const PILLAR_AP_RATIO: usize = 75;
/// 1.25 seconds of airborne. Towers are excluded from the lift - there is
/// nothing to pop - but they still take the hit.
pub const PILLAR_AIRBORNE_TICKS: u64 = 75;
/// The pillar itself. A `view_effect` in the data file, a buff worn by the
/// target now that `crate::vfx` is what draws her - keep it equal to the
/// sheet's 6 frames x 5 ticks or the last frame hangs.
pub const PILLAR_VFX_BUFF: &str = "toph_rock_pillar_vfx";
pub const PILLAR_VFX_TICKS: usize = 30;

// ------------------------------------------------ The First Metalbender
/// The armor she is wearing, and the window before it blows.
pub const METAL_BUFF: &str = "toph_metal_armor";
pub const METAL_DURATION: usize = 6 * 60;
pub const METAL_ARMOR: usize = 60;
pub const METAL_ARMOR_HP_PERMILLE: usize = 30;
pub const METAL_RESIST: usize = 30;
pub const METAL_RESIST_HP_PERMILLE: usize = 15;

/// The recast half of the ability, which the engine will not give her for
/// free: an action is castable again only once its cooldown is up, and the
/// armor's own window closes long before that. So skill2 carries
/// `METAL_USE_COUNT` charges - two casts before the cooldown starts - and
/// these two markers decide what each of them does.
///
/// `ARMED` says the armor is on and the next cast is the detonation. `SPENT`
/// says this cycle already detonated on its own timer, so the cast still in
/// hand buys nothing: without it the leftover charge would don a second suit
/// inside one cooldown, which is twice what a recast pays for.
///
/// `SPENT` is only ever set on the expiry path. A recast spends both charges
/// itself, so there is no charge left over to guard against, and a marker left
/// standing would eat the first cast of the next cycle instead.
pub const METAL_ARMED_BUFF: &str = "toph_metal_armed";
pub const METAL_SPENT_BUFF: &str = "toph_metal_spent";
/// What is left of the cooldown once the armor has run its window out, which
/// is exactly how long the leftover charge can still be spent for - if the
/// engine starts an action's cooldown on its *first* use.
///
/// It is timed rather than permanent on purpose, and the difference matters
/// because `cooltime_use_count` has not been read out of the engine, only off
/// its schema. A permanent marker would close the hole outright, but if the
/// engine turns out to ignore the field - or to start the cooldown on the last
/// charge instead of the first - a permanent one would sit on her forever and
/// eat the opening cast of every cycle after this one, which is the ability
/// broken rather than the ability loose. Timed, the two ways this can be wrong
/// are: she occasionally gets a second armor window inside one cooldown, or the
/// marker times out having done nothing at all. Both are survivable.
pub const METAL_SPENT_TICKS: usize = 6 * 60;

pub const METAL_BURST_DAMAGE: usize = 100;
pub const METAL_BURST_AP_RATIO: usize = 60;
pub const METAL_BURST_RADIUS: u64 = 35_000;
/// Keep equal to the sheet's 5 frames x 0.1s or the last frame hangs.
pub const METAL_BURST_VFX_BUFF: &str = "toph_metal_burst";
pub const METAL_BURST_VFX_TICKS: usize = 30;

// ------------------------------------------------ Blind Bandit
pub const BANDIT_DAMAGE: usize = 200;
pub const BANDIT_AP_RATIO: usize = 45;
pub const BANDIT_HP_RATIO: usize = 5;
/// Centred on her, so the whole ult is one self-cast and needs no target.
pub const BANDIT_RADIUS: u64 = 50_000;
/// Half a second between waves, which is also the shockwave sheet's own
/// length - each ring finishes as the next one starts.
pub const BANDIT_WAVE_INTERVAL: usize = 30;
pub const BANDIT_WAVE_VFX_BUFF: &str = "toph_shockwave";
pub const BANDIT_WAVE_VFX_TICKS: usize = 30;

/// Waves by level: 3 up to `BANDIT_MID_LEVEL`, then 4, then 5 from
/// `BANDIT_MAX_LEVEL`.
///
/// The three-wave tier is written to match the ability text but she cannot
/// reach it in a normal match: the engine unlocks ults at level 5
/// (`util::ULT_LEVEL`), so the first Blind Bandit she ever casts is already a
/// four-wave one. It is kept rather than folded away because anything that
/// moves that unlock - a rule change, another mod - should find the tier here
/// rather than a floor of four.
pub const BANDIT_WAVES_MIN: usize = 3;
pub const BANDIT_WAVES_MID: usize = 4;
pub const BANDIT_WAVES_MAX: usize = 5;
pub const BANDIT_MID_LEVEL: usize = 5;
pub const BANDIT_MAX_LEVEL: usize = 9;

/// The last wave only. A slow is a stat buff rather than a crowd control here -
/// the engine has no slow among its `CcKindV1` kinds, and `move_speed_mult`
/// is what every other percentage speed change in the mod rides on.
pub const BANDIT_SLOW_BUFF: &str = "toph_blind_bandit_slow";
pub const BANDIT_SLOW_PERCENT: i32 = -20;
pub const BANDIT_SLOW_DURATION: usize = 2 * 60;
