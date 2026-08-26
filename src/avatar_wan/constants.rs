use super::element::Element;
use crate::util::TICKS_PER_SECOND;

macro_rules! ticks {
    ($num:expr) => {
        $num * TICKS_PER_SECOND
    };
}

// ------------------------------------------------ Soul of Raava
pub const ATTACK_MAGIC_SHARE: usize = 50;
pub const ATTACK_PHYSICAL_SHARE: usize = 50;
pub const ATTACK_COOLTIME: usize = 50;

// ------------------------------------------------ Elements
pub const STARTING_ELEMENT: Element = Element::Fire;

pub const AIR_STACK_BUFF: &str = "wan_air_stack";
pub const AIR_MOVE_SPEED_PERCENT: i32 = 6;
pub const AIR_DURATION: usize = ticks!(4.0) as usize;
pub const AIR_MAX_STACKS: usize = 4;

pub const WATER_HEAL_FLAT: usize = 3;
pub const WATER_MISSING_HP_PERCENT: usize = 3;
pub const WATER_SPLASH_VFX_BUFF: &str = "wan_water_splash";
pub const WATER_SPLASH_VFX_TICKS: usize = 18;

pub const EARTH_ATTACK_SHARE: usize = 40;
pub const EARTH_RADIUS: u64 = 40_000;
pub const EARTH_SPLASH_VFX_BUFF: &str = "wan_earth_splash";
pub const EARTH_SPLASH_VFX_TICKS: usize = 36;

pub const BURN_DAMAGE: usize = 12;
pub const BURN_AP_RATIO: usize = 30;
// The burn lands as two damage types. Only this half is physical - the flat
// BURN_DAMAGE stays on the magic side with the AP scaling.
pub const BURN_AD_RATIO: usize = 6;
pub const BURN_TICKS: usize = 6;
pub const BURN_TICK_INTERVAL: usize = 30;
pub const BURN_VFX_BUFF: &str = "wan_fire_burn";
pub const BURN_VFX_TICKS: usize = BURN_TICKS * BURN_TICK_INTERVAL;

// ------------------------------------------------ Spirit Step
pub const STEP_BUFF: &str = "wan_spirit_step";
// 500 AD * 0.03% = 15% AS
pub const STEP_ATTACK_SPEED_PERCENT: i32 = 10;
pub const STEP_ATTACK_SPEED_AD_RATIO: usize = 3;
pub const STEP_BUFF_DURATION: usize = ticks!(3.0) as usize;
pub const STEP_STORE_PERCENT: usize = 80;
pub const STEP_HEAL_PERCENT: usize = 80;
pub const STEP_HEAL_FLAT: usize = 30;
pub const STEP_STORE_PREFIX: &str = "wan_step_store";
pub const STEP_DASH_DISTANCE: u64 = 60_000;
pub const STEP_HEAL_VFX_BUFF: &str = "wan_spirit_step_heal";
pub const STEP_HEAL_VFX_TICKS: usize = 22;
pub const STEP_DASH_TICKS: usize = 20;

// ------------------------------------------------ Harmonic Convergence
pub const CONVERGENCE_BUFF: &str = "wan_harmonic_convergence";
pub const CONVERGENCE_DURATION: usize = ticks!(6.0) as usize;
// Matches the buff's base duration; takedown extensions lengthen both.
pub const CONVERGENCE_SHIELD_DURATION: usize = ticks!(6.0) as usize;
pub const CONVERGENCE_SHIELD: usize = 300;
pub const CONVERGENCE_SHIELD_AP_RATIO: usize = 60;
pub const CONVERGENCE_SHIELD_HP_RATIO: usize = 6;
pub const CONVERGENCE_BASE_SCALE: usize = 100;
// Duration each takedown — a kill or an assist — adds to an active ult.
pub const CONVERGENCE_TAKEDOWN_EXTENSION: usize = ticks!(1.5) as usize;
// How hurt to assume he is when valuing Water's heal — used by the basic
// attack and by the ult, since neither can read current HP.
pub const EXPECTED_MISSING_HP_PERCENT: usize = 25;

// ------------------------------------------------ Aggression
pub const ATTACK_RANGE: u64 = 70_000;
pub const AGGRO_ENGAGE_RANGE: u64 = 82_000;
pub const AGGRO_COMBAT_RANGE: u64 = 90_000;
pub const AGGRO_LOW_HP: usize = 50;
pub const AGGRO_HP_FLOOR: usize = 50;
pub const AGGRO_COMMITTED_HP_FLOOR: usize = 25;
pub const AGGRO_ULT_HP_FLOOR: usize = 12;
pub const AGGRO_ULT_ENGAGE_RANGE: u64 = 95_000;
pub const AGGRO_ULT_FOCUS_HP: usize = 40;
