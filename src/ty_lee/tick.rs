//! Ty Lee's per-tick upkeep: the Chi Block stack counter.
//!
//! Chi Block's stacks are timed, so they fall off without going back through
//! `apply_chi_block` - the count can drop with nothing to hook, so the VFX buff
//! that draws it is reconciled rather than set on apply.

use mod_api_stable::*;

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

/// What Chi Block should be drawing on this unit: the overlay for its stack
/// count, and whether the break burst is still allowed to be on it.
///
/// A corpse shows neither. Buffs outlive the unit here, so anything left on one
/// keeps drawing over its death animation.
///
/// Zero health counts as dead on its own: the hit that finishes a set is
/// usually the one that kills, and `is_alive` can still read true while the
/// death animation runs - which is exactly the window the burst was showing in.
fn chi_block_vfx(entity: &StableEntity<'_, '_>) -> (Option<&'static str>, bool) {
    if !entity.is_alive() || entity.hp().0 == 0 {
        return (None, false);
    }

    let stack = stacks(entity, CHI_BLOCK_BUFF)
        .checked_sub(1)
        .and_then(|index| CHI_BLOCK_VFX_BUFFS.get(index).copied());
    (stack, true)
}

fn recount_chi_block(sim: &mut StableSim<'_>) {
    let miscounted: Vec<(usize, Option<&'static str>, bool)> = (0..sim.entity_count())
        .filter_map(|index| sim.entity_at(index))
        .filter_map(|entity| {
            let (wanted, burst_allowed) = chi_block_vfx(&entity);
            let shown = CHI_BLOCK_VFX_BUFFS
                .iter()
                .copied()
                .find(|name| has_buff(&entity, name));
            let drop_burst = !burst_allowed && has_buff(&entity, CHI_BLOCK_BREAK_VFX_BUFF);

            (wanted != shown || drop_burst).then_some((entity.id(), wanted, drop_burst))
        })
        .collect();

    for (entity, wanted, drop_burst) in miscounted {
        for name in CHI_BLOCK_VFX_BUFFS {
            sim.entity_remove_buff(entity, name);
        }
        if drop_burst {
            sim.entity_remove_buff(entity, CHI_BLOCK_BREAK_VFX_BUFF);
        }
        if let Some(name) = wanted {
            sim.add_buff(entity, &BuffV1::named(name));
        }
    }
}

pub fn on_match_tick(sim: &mut StableSim<'_>) {
    recount_chi_block(sim);
}
