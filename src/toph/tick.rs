//! Toph's per-tick upkeep: the armor's own timer, and keeping her mark off
//! corpses.
//!
//! Both jobs exist for the same reason. Buffs here are timed, so they fall off
//! with nothing to hook - there is no "this buff expired" callback anywhere in
//! the API - and buffs outlive the unit wearing them, so anything left on one
//! keeps drawing over its death animation. What the data layer cannot notice
//! on its own gets noticed here instead.

use mod_api_stable::*;

use super::effects::detonate;
use super::*;

fn stacks(entity: &StableEntity<'_, '_>, name: &str) -> usize {
    (0..entity.buff_count())
        .filter_map(|index| entity.buff_at(index))
        .filter(|buff| buff.name() == name)
        .count()
}

fn has_buff(entity: &StableEntity<'_, '_>, name: &str) -> bool {
    stacks(entity, name) > 0
}

fn is_toph(entity: &StableEntity<'_, '_>) -> bool {
    crate::util::is_champion(entity, CHAMPION_KEY)
}

/// What an armed Toph is owed this tick.
enum Armor {
    /// The window ran out with the suit still on her. This is the "after 6
    /// seconds" half of the ability.
    Blow,
    /// She died wearing it. A corpse blows nothing up, and the markers have to
    /// go or she would come back from the respawn already armed - the next
    /// cast would detonate a suit she is not wearing.
    Forget,
}

fn armor_due(entity: &StableEntity<'_, '_>) -> Option<Armor> {
    if !has_buff(entity, METAL_ARMED_BUFF) {
        return None;
    }
    if !entity.is_alive() || entity.hp().0 == 0 {
        return Some(Armor::Forget);
    }
    // Still wearing it: nothing due until the window closes.
    (!has_buff(entity, METAL_BUFF)).then_some(Armor::Blow)
}

fn resolve_armor(sim: &mut StableSim<'_>) {
    let due: Vec<(usize, Armor)> = (0..sim.entity_count())
        .filter_map(|index| sim.entity_at(index))
        .filter(is_toph)
        .filter_map(|entity| Some((entity.id(), armor_due(&entity)?)))
        .collect();

    for (entity, armor) in due {
        match armor {
            Armor::Blow => {
                // Clears the markers itself, so this cannot fire twice.
                detonate(sim, entity);
                // The charge she is still holding buys nothing now - see
                // `METAL_SPENT_BUFF`. It is timed to the rest of the cooldown,
                // which is all the time that charge has left to be spent in.
                sim.add_buff(
                    entity,
                    &BuffV1::timed(METAL_SPENT_BUFF, METAL_SPENT_TICKS),
                );
            }
            Armor::Forget => {
                sim.entity_remove_buff(entity, METAL_ARMED_BUFF);
                sim.entity_remove_buff(entity, METAL_BUFF);
                sim.entity_remove_buff(entity, METAL_SPENT_BUFF);
            }
        }
    }
}

/// A corpse shows no mark. The hit that cashes one in is usually the hit that
/// kills, and zero health counts as dead on its own - `is_alive` can still read
/// true while the death animation plays, which is exactly the window the mark
/// and its burst were drawing in.
fn clear_marks_from_corpses(sim: &mut StableSim<'_>) {
    let marked: Vec<usize> = (0..sim.entity_count())
        .filter_map(|index| sim.entity_at(index))
        .filter(|entity| !entity.is_alive() || entity.hp().0 == 0)
        .filter(|entity| has_buff(entity, MARK_BUFF) || has_buff(entity, MARK_POP_VFX_BUFF))
        .map(|entity| entity.id())
        .collect();

    for entity in marked {
        sim.entity_remove_buff(entity, MARK_BUFF);
        sim.entity_remove_buff(entity, MARK_POP_VFX_BUFF);
    }
}

pub fn on_match_tick(sim: &mut StableSim<'_>) {
    resolve_armor(sim);
    clear_marks_from_corpses(sim);
}
