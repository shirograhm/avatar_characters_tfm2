use mod_api_stable::*;

use super::*;
use crate::util::{enemies_near, has_buff, percent_of, permille_of, stat_of};

pub const SEISMIC_SENSE: &str = "toph_seismic_sense";
pub const ROCK_PILLAR: &str = "toph_rock_pillar";
pub const FIRST_METALBENDER: &str = "toph_first_metalbender";
pub const BLIND_BANDIT: &str = "toph_blind_bandit";
pub const BLIND_BANDIT_WAVE: &str = "toph_blind_bandit_wave";

fn is_champion_id(sim: &StableSim<'_>, entity: usize) -> bool {
    sim.get_entity(entity)
        .is_some_and(|entity| entity.is_champion())
}

/// Alive enough to be owed the rest of a hit.
///
/// Zero health counts as dead on its own: the mark's payoff is usually what
/// kills, and `is_alive` can still read true while the death animation runs -
/// which is exactly the window a burst would play over.
fn still_standing(sim: &StableSim<'_>, entity: usize) -> bool {
    sim.get_entity(entity).is_some_and(|entity| entity.is_alive() && entity.hp().0 > 0)
}

pub fn mark_bonus(caster_stat: &StatV1) -> usize {
    MARK_BONUS_MAGIC + percent_of(caster_stat.magic_power, MARK_BONUS_AP_RATIO)
}

/// The mark. Champions only - a mark is a promise of a second, larger hit, and
/// a minion rarely lives long enough to collect on one.
fn apply_mark(sim: &mut StableSim<'_>, target: usize) {
    if !is_champion_id(sim, target) || !still_standing(sim, target) {
        return;
    }
    // Re-marking refreshes the window rather than stacking a second one, so
    // the count on a unit only ever reads one and `tick` has nothing to
    // reconcile but corpses.
    sim.entity_remove_buff(target, MARK_BUFF);
    sim.add_buff(target, &BuffV1::timed(MARK_BUFF, MARK_DURATION));
}

/// Every point of damage Toph deals goes out through here, because Seismic
/// Sense says any of it cashes a mark in - not just the basic attack that set
/// one.
///
/// The mark is read *before* the damage lands and applied only afterwards, so
/// an attack can never pop the mark it is about to leave. That ordering is the
/// whole mechanic: the first hit on a fresh target marks, and everything after
/// it collects and re-marks.
fn hit(
    sim: &mut StableSim<'_>,
    caster: usize,
    target: usize,
    ad: usize,
    ap: usize,
    attack_type: AttackTypeV1,
) {
    let marked = has_buff(sim, target, MARK_BUFF);
    let bonus = mark_bonus(&stat_of(sim, caster));

    sim.deal_damage(caster, target, ad, ap, attack_type);

    if !marked {
        return;
    }
    sim.entity_remove_buff(target, MARK_BUFF);

    // The hit above is often what kills them, and a burst started on a corpse
    // plays over its death animation instead of their death.
    if !still_standing(sim, target) {
        return;
    }

    // Skill damage rather than base attack: this is the mark going off, not a
    // second swing, and typing it as an attack would put it back through every
    // on-hit the engine runs.
    sim.deal_damage(caster, target, 0, bonus, AttackTypeV1::Skill);
    sim.add_buff(
        target,
        &BuffV1::timed(MARK_POP_VFX_BUFF, MARK_POP_VFX_TICKS),
    );
}

pub struct SeismicSense;

impl StableEffectType for SeismicSense {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        input: InputTargetV1,
    ) {
        if InputTargetKindV1::from_code(input.kind) != Some(InputTargetKindV1::Target) {
            return;
        }
        let target = input.target_id;
        let stat = stat_of(sim, caster_id);

        hit(
            sim,
            caster_id,
            target,
            percent_of(stat.attack, ATTACK_AD_RATIO),
            0,
            AttackTypeV1::BaseAttack,
        );
        apply_mark(sim, target);
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        // The full bonus, not a share of it. Against the champion this number
        // is used to value, every attack after the first cashes a mark in, so
        // the sustained rate really is one proc per swing.
        (
            percent_of(caster_stat.attack, ATTACK_AD_RATIO),
            mark_bonus(caster_stat),
        )
    }
}

pub fn pillar_damage(caster_stat: &StatV1) -> usize {
    PILLAR_DAMAGE + percent_of(caster_stat.magic_power, PILLAR_AP_RATIO)
}

pub struct RockPillar;

impl StableEffectType for RockPillar {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        input: InputTargetV1,
    ) {
        if InputTargetKindV1::from_code(input.kind) != Some(InputTargetKindV1::Target) {
            return;
        }
        let target = input.target_id;
        let damage = pillar_damage(&stat_of(sim, caster_id));

        // Read before the hit: a tower is still owed the damage, it just has
        // nothing to pop into the air, and it may not survive to be asked.
        let liftable = sim
            .get_entity(target)
            .is_some_and(|entity| !entity.is_tower());

        hit(sim, caster_id, target, 0, damage, AttackTypeV1::Skill);

        if liftable && still_standing(sim, target) {
            sim.apply_cc(
                target,
                &CcV1::of_kind(CcKindV1::Airborne, PILLAR_AIRBORNE_TICKS),
            );
        }
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (0, pillar_damage(caster_stat))
    }

    fn expected_cc_time(&self) -> Option<usize> {
        Some(PILLAR_AIRBORNE_TICKS as usize)
    }
}

/// The suit itself. Flat armor and resistance, both scaling off her own
/// health, which is read at the moment she bends it - so a Toph who buys health
/// mid match wears a thicker suit from the next cast on.
pub fn metal_armor_buff(caster_stat: &StatV1) -> BuffV1 {
    let mut buff = BuffV1::timed(METAL_BUFF, METAL_DURATION);
    buff.defence = (METAL_ARMOR + permille_of(caster_stat.hp, METAL_ARMOR_HP_PERMILLE)) as i32;
    buff.magic_resistance =
        (METAL_RESIST + permille_of(caster_stat.hp, METAL_RESIST_HP_PERMILLE)) as i32;
    buff
}

pub fn metal_burst_damage(caster_stat: &StatV1) -> usize {
    METAL_BURST_DAMAGE + percent_of(caster_stat.magic_power, METAL_BURST_AP_RATIO)
}

/// The armor coming apart, from either of the two things that can set it off:
/// the recast, or the window running out under `tick`.
///
/// The markers come off first so that neither path can fire it twice, and the
/// burst is dealt through `hit` like everything else of hers - it is damage,
/// so it cashes marks in.
pub fn detonate(sim: &mut StableSim<'_>, caster: usize) {
    let damage = metal_burst_damage(&stat_of(sim, caster));

    sim.entity_remove_buff(caster, METAL_ARMED_BUFF);
    sim.entity_remove_buff(caster, METAL_BUFF);
    sim.add_buff(
        caster,
        &BuffV1::timed(METAL_BURST_VFX_BUFF, METAL_BURST_VFX_TICKS),
    );

    for enemy in enemies_near(sim, caster, caster, METAL_BURST_RADIUS) {
        if still_standing(sim, enemy) {
            hit(sim, caster, enemy, 0, damage, AttackTypeV1::Skill);
        }
    }
}

/// Both halves of the ability, chosen by what she is already wearing.
///
/// The engine has no recast of its own, so skill2 carries two charges and this
/// decides what each spends. See `METAL_ARMED_BUFF` for why the third branch
/// exists and does nothing.
pub struct FirstMetalbender;

impl StableEffectType for FirstMetalbender {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        // Already blew on its own timer this cycle. The charge still in hand
        // buys nothing - spending it here is what puts the ability back on
        // cooldown, and a second suit inside one cooldown is twice what the
        // ability pays for.
        if has_buff(sim, caster_id, METAL_SPENT_BUFF) {
            sim.entity_remove_buff(caster_id, METAL_SPENT_BUFF);
            return;
        }

        if has_buff(sim, caster_id, METAL_ARMED_BUFF) {
            detonate(sim, caster_id);
            return;
        }

        let stat = stat_of(sim, caster_id);
        sim.add_buff(caster_id, &metal_armor_buff(&stat));
        sim.add_buff(caster_id, &BuffV1::named(METAL_ARMED_BUFF));
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (0, metal_burst_damage(caster_stat))
    }

    fn expected_buff(&self, caster_stat: &StatV1) -> Option<BuffV1> {
        Some(metal_armor_buff(caster_stat))
    }

    fn on_caster(&self) -> bool {
        true
    }

    fn can_move(&self) -> bool {
        true
    }
}

/// How many shockwaves the slam is worth at this level. See
/// `BANDIT_WAVES_MIN` for why the lowest tier is unreachable in a normal
/// match and kept anyway.
pub fn wave_count(level: usize) -> usize {
    match level {
        level if level >= BANDIT_MAX_LEVEL => BANDIT_WAVES_MAX,
        level if level >= BANDIT_MID_LEVEL => BANDIT_WAVES_MID,
        _ => BANDIT_WAVES_MIN,
    }
}

pub fn bandit_damage(caster_stat: &StatV1) -> usize {
    BANDIT_DAMAGE
        + percent_of(caster_stat.magic_power, BANDIT_AP_RATIO)
        + percent_of(caster_stat.hp, BANDIT_HP_RATIO)
}

/// The slow the last wave leaves behind. A stat buff rather than a crowd
/// control: the engine's `CcKindV1` has no slow among its kinds.
pub fn slow_buff() -> BuffV1 {
    let mut buff = BuffV1::timed(BANDIT_SLOW_BUFF, BANDIT_SLOW_DURATION);
    buff.move_speed_mult = BANDIT_SLOW_PERCENT;
    buff
}

/// One shockwave, centred on wherever she is standing when it goes off rather
/// than where she cast from - the waves roll out from her, and the ult does not
/// root her in place.
fn shockwave(sim: &mut StableSim<'_>, caster: usize, last: bool) {
    if !still_standing(sim, caster) {
        return;
    }
    let damage = bandit_damage(&stat_of(sim, caster));

    // A wave lands on the tick the one before it finishes, so for that tick
    // both would be on her and the ring would draw twice. Refresh rather than
    // stack, the same way the mark does.
    sim.entity_remove_buff(caster, BANDIT_WAVE_VFX_BUFF);
    sim.add_buff(
        caster,
        &BuffV1::timed(BANDIT_WAVE_VFX_BUFF, BANDIT_WAVE_VFX_TICKS),
    );

    for enemy in enemies_near(sim, caster, caster, BANDIT_RADIUS) {
        if !still_standing(sim, enemy) {
            continue;
        }
        hit(sim, caster, enemy, 0, damage, AttackTypeV1::Skill);

        if !last {
            continue;
        }
        // The slow goes on anything the wave caught; the mark is champions
        // only, which `apply_mark` already knows. Both are read after the hit,
        // so neither lands on something the wave just killed.
        if still_standing(sim, enemy) {
            sim.add_buff(enemy, &slow_buff());
            apply_mark(sim, enemy);
        }
    }
}

pub struct BlindBandit;

impl StableEffectType for BlindBandit {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        let waves = sim
            .get_entity(caster_id)
            .map_or(BANDIT_WAVES_MIN, |caster| wave_count(caster.level()));

        shockwave(sim, caster_id, waves == 1);

        for index in 1..waves {
            // `x` carries which wave this is and `y` how many there are, so
            // the follow-up knows whether it is the last one. Wan's burn
            // passes its scale the same way.
            let queued = InputTargetV1 {
                x: index as u64,
                y: waves as u64,
                ..InputTargetV1::NONE
            };
            sim.queue_effect(
                BLIND_BANDIT_WAVE,
                AttackTypeV1::Skill,
                caster_id,
                &queued,
                index * BANDIT_WAVE_INTERVAL,
            );
        }
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        // The four-wave tier: the ult unlocks at level 5, so that is the
        // smallest slam she ever actually casts.
        (0, bandit_damage(caster_stat) * BANDIT_WAVES_MID)
    }

    fn expected_buff(&self, _caster_stat: &StatV1) -> Option<BuffV1> {
        Some(slow_buff())
    }

    fn on_caster(&self) -> bool {
        true
    }
}

/// The follow-up shockwaves, replayed from the effect queue.
pub struct BlindBanditWave;

impl StableEffectType for BlindBanditWave {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        input: InputTargetV1,
    ) {
        shockwave(sim, caster_id, input.x + 1 >= input.y);
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (0, bandit_damage(caster_stat))
    }

    fn on_caster(&self) -> bool {
        true
    }
}

/// Every enemy champion inside `range` already carrying a mark, closest first.
/// This is the whole set of openings the AI steers by: a marked champion is
/// worth the better part of a hundred extra magic damage to reach.
pub fn marked_enemies(sim: &StableSim<'_>, caster: usize, range: u64) -> Vec<(u64, usize)> {
    let Some(team) = sim.get_entity(caster).map(|me| me.team()) else {
        return Vec::new();
    };
    let range_sq = range.saturating_mul(range);

    (0..sim.champion_count())
        .map(|index| sim.champion_id_at(index))
        .filter(|&id| {
            sim.get_entity(id)
                .is_some_and(|other| other.is_alive() && other.team() != team)
        })
        .filter(|&id| has_buff(sim, id, MARK_BUFF))
        .map(|id| (sim.distance_sq(caster, id), id))
        .filter(|&(distance_sq, _)| distance_sq <= range_sq)
        .collect()
}

/// How many enemy champions a slam cast right now would catch. Minions are not
/// counted on purpose: the ult is worth sixty seconds against champions and
/// wasted on a wave, so a wave should read the same as an empty field.
pub fn slam_champions(sim: &StableSim<'_>, caster: usize) -> usize {
    enemies_near(sim, caster, caster, BANDIT_RADIUS)
        .into_iter()
        .filter(|&id| is_champion_id(sim, id))
        .count()
}
