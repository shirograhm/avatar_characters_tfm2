//! Circus Freak's delayed payoff.
//!
//! The window itself needs no upkeep: `base_attack_damaged_reduce` on a timed
//! buff is the engine's own basic-attack mitigation, so it applies and expires
//! without help. What does need a hook is the shield at the end - a buff cannot
//! fire an effect when it runs out, so the cast leaves an untimed marker behind
//! and this watches for the tick where the window is gone but the marker is
//! not. Wan's Spirit Step banks its heal the same way.

use mod_api_stable::*;

use super::effects::evasion_shield;
use super::*;

fn has_buff(entity: &StableEntity<'_, '_>, name: &str) -> bool {
    (0..entity.buff_count())
        .filter_map(|index| entity.buff_at(index))
        .any(|buff| buff.name() == name)
}

/// `Some` on the tick the window closes, carrying the shield it owes. A Ty Lee
/// who died mid-window is filtered out before this and is owed nothing.
fn closed_window(entity: &StableEntity<'_, '_>) -> Option<(usize, usize)> {
    (has_buff(entity, EVASION_PENDING_BUFF) && !has_buff(entity, EVASION_BUFF))
        .then(|| (entity.id(), evasion_shield(&entity.stat())))
}

pub fn on_match_tick(sim: &mut StableSim<'_>) {
    let owed: Vec<(usize, usize)> = (0..sim.entity_count())
        .filter_map(|index| sim.entity_at(index))
        .filter(|entity| entity.is_alive() && is_ty_lee(entity))
        .filter_map(|entity| closed_window(&entity))
        .collect();

    for (entity, shield) in owed {
        sim.entity_remove_buff(entity, EVASION_PENDING_BUFF);
        if shield > 0 {
            sim.entity_add_shield(entity, shield, EVASION_SHIELD_DURATION);
        }
    }
}
