//! Helpers shared by every champion in the mod. Anything that only one
//! champion cares about belongs in that champion's own module instead.

use mod_api_stable::*;

pub const MAP_SIZE: u64 = 960_000;
pub const TICKS_PER_SECOND: f64 = 60.0;

/// The level each action slot unlocks at. An engine rule rather than a
/// champion one - nothing in `.data_champion` sets it, and the champion panel
/// labels every kit's three skills Lv.1 / Lv.3 / Lv.5.
///
/// This matters because the remaining-cooldown counters do not encode it: a
/// skill she has not learned yet reads back as zero ticks remaining, which is
/// indistinguishable from one that is off cooldown. Any hook that names an
/// action of its own has to check the level as well - see `ty_lee::player_ai`.
pub const SKILL_LEVEL: usize = 1;
pub const SKILL2_LEVEL: usize = 3;
pub const ULT_LEVEL: usize = 5;

pub fn percent_of(value: usize, percent: usize) -> usize {
    (value * percent) / 100
}

/// The same thing in tenths of a percent, for the ratios that are not whole
/// ones - 1.5% of health is `permille_of(hp, 15)`. Everything here is integer
/// maths, so a fractional percentage has nowhere else to go.
pub fn permille_of(value: usize, permille: usize) -> usize {
    (value * permille) / 1000
}

pub fn stat_of(sim: &StableSim<'_>, entity: usize) -> StatV1 {
    sim.get_entity(entity)
        .map_or(StatV1::default(), |entity| entity.stat())
}

/// Buffs stack as separate entries under the same name, so counting them is
/// how the mod stores a stack count on an entity.
pub fn buff_stacks(sim: &StableSim<'_>, entity: usize, name: &str) -> usize {
    let Some(entity) = sim.get_entity(entity) else {
        return 0;
    };
    (0..entity.buff_count())
        .filter_map(|index| entity.buff_at(index))
        .filter(|buff| buff.name() == name)
        .count()
}

pub fn has_buff(sim: &StableSim<'_>, entity: usize, name: &str) -> bool {
    buff_stacks(sim, entity, name) > 0
}

/// True when the entity is the champion registered under `key`. Entity names
/// come through as the display name, so "Ty Lee" matches `ty_lee`.
pub fn is_champion(entity: &StableEntity<'_, '_>, key: &str) -> bool {
    entity.is_champion()
        && entity
            .name()
            .is_some_and(|name| name.trim().to_ascii_lowercase().replace(' ', "_") == key)
}

pub fn full_step_toward(
    from: (u64, u64),
    toward: (u64, u64),
    distance: u64,
    margin: u64,
) -> Option<((u64, u64), (i64, i64))> {
    let dx = toward.0 as i64 - from.0 as i64;
    let dy = toward.1 as i64 - from.1 as i64;

    let length = isqrt((dx * dx + dy * dy) as u64) as i64;
    if length == 0 {
        return None;
    }

    let step_x = dx * distance as i64 / length;
    let step_y = dy * distance as i64 / length;

    let margin = margin.min(MAP_SIZE / 2);
    let axis = |origin: u64, step: i64| {
        let inside = |step: i64| {
            let end = origin as i64 + step;
            (end >= margin as i64 && end <= (MAP_SIZE - margin) as i64)
                .then_some((end as u64, step))
        };
        inside(step).or_else(|| inside(-step)).unwrap_or_else(|| {
            let end = (origin as i64 + step).clamp(margin as i64, (MAP_SIZE - margin) as i64);
            (end as u64, end - origin as i64)
        })
    };

    let (x, step_x) = axis(from.0, step_x);
    let (y, step_y) = axis(from.1, step_y);
    Some(((x, y), (step_x, step_y)))
}

pub fn isqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }

    let mut guess = value;
    let mut next = (guess + value / guess) / 2;
    while next < guess {
        guess = next;
        next = (guess + value / guess) / 2;
    }
    guess
}

pub fn enemies_near(
    sim: &StableSim<'_>,
    caster_id: usize,
    center_id: usize,
    radius: u64,
) -> Vec<usize> {
    let Some(caster) = sim.get_entity(caster_id) else {
        return Vec::new();
    };
    let team = caster.team();
    let radius_sq = radius.saturating_mul(radius);

    (0..sim.entity_count())
        .filter_map(|index| sim.entity_at(index))
        .filter(|entity| entity.is_alive() && entity.team() != team)
        .map(|entity| entity.id())
        .filter(|&id| id != center_id && sim.distance_sq(center_id, id) <= radius_sq)
        .collect()
}

/// The closest living enemy champion inside `range`, or `None` when there is
/// none. Towers and minions are never candidates - every caller so far wants
/// the champion fight rather than the wave.
///
/// Lives here rather than in one champion's module because both Ty Lee and
/// Toph aim off it: she reaches for the nearest champion when nothing better
/// presents itself, and he - well, she - opens on one.
pub fn nearest_enemy_champion(sim: &StableSim<'_>, caster: usize, range: u64) -> Option<usize> {
    let team = sim.get_entity(caster)?.team();
    let range_sq = range.saturating_mul(range);

    (0..sim.champion_count())
        .map(|index| sim.champion_id_at(index))
        .filter(|&id| id != caster)
        .filter(|&id| {
            sim.get_entity(id).is_some_and(|entity| {
                entity.is_alive() && entity.is_champion() && entity.team() != team
            })
        })
        .filter(|&id| sim.distance_sq(caster, id) <= range_sq)
        // Ties resolve by champion order, which is deterministic.
        .min_by_key(|&id| sim.distance_sq(caster, id))
}

/// Deterministic 0..99 roll derived from the seed the engine hands an effect.
/// splitmix64 — the effect API guarantees the seed is reproducible, so every
/// client replays the same rolls.
pub fn rng_percent(seed: u64) -> usize {
    let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((z ^ (z >> 31)) % 100) as usize
}
