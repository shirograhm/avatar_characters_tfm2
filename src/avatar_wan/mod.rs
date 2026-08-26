//! Avatar Wan — a ranged elementalist whose basic attack changes with the
//! element he is attuned to.

use mod_api_stable::*;

pub mod constants;
pub mod effects;
pub mod element;
pub mod player_ai;
pub mod tick;

pub const CHAMPION_KEY: &str = "avatar_wan";

pub fn is_wan(entity: &StableEntity<'_, '_>) -> bool {
    crate::util::is_champion(entity, CHAMPION_KEY)
}
