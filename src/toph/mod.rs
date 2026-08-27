//! Toph Beifong — a melee earthbender who marks what she hits and cashes the
//! marks in with everything else. Built on the same primitives as Ty Lee:
//! named buffs as counters, native effects referenced from `.data_champion`,
//! and a slice of the per-tick match hook for the parts the data layer cannot
//! do.

pub mod effects;
pub mod player_ai;
pub mod tick;

// ------------------------------------------------ Defaults
pub const CHAMPION_KEY: &str = "toph";

// ------------------------------------------------ Action lengths
/// How long The First Metalbender occupies her, mirroring `skill2.duration` in
/// `.data_champion` - keep the two in step. The AI has no way to ask the engine
/// whether a cast is still running, so after issuing the recast itself it goes
/// quiet for this long rather than issuing into an action already playing.
///
/// It is the only one of her four that needs a length here, because it is the
/// only cast this mod ever starts on its own; everything else the AI does is a
/// re-aim of a cast the base AI had already decided to make.
pub const METAL_ACTION_TICKS: usize = 18;

// ------------------------------------------------ Seismic Sense (base attack)
pub const ATTACK_AD_RATIO: usize = 100;
/// Mirrors `attack.range` in `.data_champion`. The AI needs to know what she
/// can hit without walking first, and it cannot read the data file - keep the
/// two in step.
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

// ------------------------------------------------ Rock Column
/// Mirrors `skill.range` in `.data_champion`. She bends the pillar up under
/// someone rather than reaching them, so it outranges her fist by a good way.
pub const COLUMN_RANGE: u64 = 45_000;
pub const COLUMN_DAMAGE: usize = 50;
pub const COLUMN_AP_RATIO: usize = 75;
/// 1.25 seconds of airborne. Towers are excluded from the lift - there is
/// nothing to pop - but they still take the hit.
pub const COLUMN_AIRBORNE_TICKS: u64 = 75;

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
/// `cooltime_use_count: 2` in `.data_champion` - two casts before the cooldown
/// starts - and these two markers decide what each of them does.
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
