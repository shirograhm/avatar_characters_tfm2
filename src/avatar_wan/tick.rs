//! Wan's per-tick upkeep: the Spirit Step damage ledger, keeping every Wan
//! attuned to an element, and extending Harmonic Convergence on a takedown.

use mod_api_stable::*;

use super::constants::*;
use super::{element, is_wan};
use crate::util::percent_of;

pub fn ledger_buff(banked: usize, last_hp: usize) -> BuffV1 {
    BuffV1::named(&format!("{STEP_STORE_PREFIX}|{banked}|{last_hp}"))
}

fn parse_ledger(name: &str) -> Option<(usize, usize)> {
    let rest = name.strip_prefix(STEP_STORE_PREFIX)?.strip_prefix('|')?;
    let (banked, last_hp) = rest.split_once('|')?;
    Some((banked.parse().ok()?, last_hp.parse().ok()?))
}

fn find_ledger(entity: &StableEntity<'_, '_>) -> Option<(String, usize, usize)> {
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .find_map(|buff| {
            let name = buff.name();
            parse_ledger(name).map(|(banked, hp)| (name.to_string(), banked, hp))
        })
}

fn window_open(entity: &StableEntity<'_, '_>) -> bool {
    (0..entity.buff_count()).any(|i| entity.buff_at(i).is_some_and(|b| b.name() == STEP_BUFF))
}

struct Pending {
    entity: usize,
    old_name: String,
    banked: usize,
    hp: usize,
    open: bool,
}

fn attune_unattuned(sim: &mut StableSim<'_>) {
    let unattuned: Vec<usize> = (0..sim.entity_count())
        .filter_map(|index| sim.entity_at(index))
        .filter(|entity| {
            entity.is_alive() && is_wan(entity) && element::current_of(entity).is_none()
        })
        .map(|entity| entity.id())
        .collect();

    for entity in unattuned {
        element::attune(sim, entity, STARTING_ELEMENT);
    }
}

fn takedowns_last_tick(sim: &StableSim<'_>, team: usize, lane: u32) -> usize {
    let Some(previous) = sim.tick().checked_sub(1) else {
        return 0;
    };

    (0..sim.kill_log_count())
        .filter_map(|index| sim.kill_log_at(index))
        .filter(|log| log.tick == previous && log.killer_team == team)
        .filter(|log| {
            let assists = (log.assist_count as usize).min(log.assist_positions.len());
            log.killer_position == lane || log.assist_positions[..assists].contains(&lane)
        })
        .count()
}

fn convergence_remaining(entity: &StableEntity<'_, '_>) -> Option<usize> {
    (0..entity.buff_count())
        .filter_map(|i| entity.buff_at(i))
        .find(|buff| buff.name() == CONVERGENCE_BUFF)
        .map(|buff| buff.duration_tick)
}

fn extend_convergence_on_takedown(sim: &mut StableSim<'_>) {
    let mut extended: Vec<(usize, usize, usize)> = Vec::new();

    for index in 0..sim.player_count() {
        let Some(player) = sim.player_at(index) else {
            continue;
        };
        let Some(champion) = player.champion() else {
            continue;
        };
        if !champion.is_alive() || !is_wan(&champion) {
            continue;
        }
        let (Some(lane), Some(remaining)) = (player.lane(), convergence_remaining(&champion))
        else {
            continue;
        };

        let takedowns = takedowns_last_tick(sim, player.team(), lane.code());
        if takedowns > 0 {
            let bonus = takedowns * CONVERGENCE_TAKEDOWN_EXTENSION;
            extended.push((champion.id(), remaining + bonus, champion.shield()));
        }
    }

    for (entity, duration, shield) in extended {
        sim.entity_remove_buff(entity, CONVERGENCE_BUFF);
        sim.add_buff(entity, &super::effects::convergence_buff(duration));

        if shield > 0 {
            sim.entity_clear_shield(entity);
            sim.entity_add_shield(entity, shield, duration);
        }
    }
}

pub fn on_match_tick(sim: &mut StableSim<'_>) {
    attune_unattuned(sim);
    extend_convergence_on_takedown(sim);

    let pending: Vec<Pending> = (0..sim.entity_count())
        .filter_map(|index| sim.entity_at(index))
        .filter(|entity| entity.is_alive())
        .filter_map(|entity| {
            let (old_name, banked, last_hp) = find_ledger(&entity)?;
            let hp = entity.hp().0;
            let taken = last_hp.saturating_sub(hp);
            Some(Pending {
                entity: entity.id(),
                old_name,
                banked: banked + percent_of(taken, STEP_STORE_PERCENT),
                hp,
                open: window_open(&entity),
            })
        })
        .collect();

    for p in pending {
        sim.entity_remove_buff(p.entity, &p.old_name);

        if p.open {
            sim.add_buff(p.entity, &ledger_buff(p.banked, p.hp));
        } else {
            let heal = STEP_HEAL_FLAT + percent_of(p.banked, STEP_HEAL_PERCENT);
            if heal > 0 {
                sim.heal(p.entity, p.entity, heal);
                sim.add_buff(
                    p.entity,
                    &BuffV1::timed(STEP_HEAL_VFX_BUFF, STEP_HEAL_VFX_TICKS),
                );
            }
        }
    }
}
