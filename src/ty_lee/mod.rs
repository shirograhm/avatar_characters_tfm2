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
pub const CHI_BLOCK_BUFF: &str = "ty_lee_chi_block";
/// Stacks fall off if she stops hitting the same target.
pub const CHI_BLOCK_DURATION: usize = 6 * 60;
pub const CHI_BLOCK_MAX_STACKS: usize = 4;
pub const CHI_BLOCK_BONUS_MAGIC: usize = 20;
pub const CHI_BLOCK_STUN_TICKS: u64 = 30;

// ------------------------------------------------ Three Point Strike
pub const STRIKE_COUNT: usize = 3;
pub const STRIKE_INTERVAL: usize = 8;
pub const STRIKE_DAMAGE: usize = 45;
pub const STRIKE_AD_RATIO: usize = 45;
pub const STRIKE_CRIT_BONUS: usize = 20;

// ------------------------------------------------ Circus Freak
/// The open window. Carries `base_attack_damaged_reduce` at her crit chance,
/// which is the engine's own basic-attack mitigation.
pub const EVASION_BUFF: &str = "ty_lee_circus_freak";
pub const EVASION_DURATION: usize = 2 * 60;
/// Untimed marker the match hook watches. Window buff gone but this still on
/// her means the window just closed and the shield is owed.
pub const EVASION_PENDING_BUFF: &str = "ty_lee_circus_freak_pending";
pub const EVASION_SHIELD: usize = 120;
pub const EVASION_SHIELD_AD_RATIO: usize = 20;
pub const EVASION_SHIELD_DURATION: usize = 2 * 60;

// ------------------------------------------------ Balancing Act
pub const BALANCE_RANGE: u64 = 80_000;
pub const BALANCE_DAMAGE: usize = 180;
pub const BALANCE_AD_RATIO: usize = 80;
/// Dash speed handed to the engine. The engine owns the movement - a mod
/// cannot move a champion mid-action itself and make it stick.
pub const BALANCE_DASH_SPEED: usize = 6_800;
pub const BALANCE_SILENCE_TICKS: u64 = 90;
pub const BALANCE_MARK_BUFF: &str = "ty_lee_balancing_act";
pub const BALANCE_MARK_DURATION: usize = 5 * 60;
/// Chi Block stacks the marked attack applies, in place of the usual one.
pub const BALANCE_MARK_CHI_BLOCKS: usize = 2;

pub fn is_ty_lee(entity: &mod_api_stable::StableEntity<'_, '_>) -> bool {
    crate::util::is_champion(entity, CHAMPION_KEY)
}
