use mod_api_stable::*;

use super::*;
use crate::util::{buff_stacks, has_buff, percent_of, stat_of};

pub const CHI_BLOCK_HIT: &str = "ty_lee_chi_block_hit";
pub const CHI_BLOCK_STUN: &str = "ty_lee_chi_block_stun";
pub const THREE_POINT_STRIKE: &str = "ty_lee_three_point_strike";
pub const THREE_POINT_STRIKE_HIT: &str = "ty_lee_three_point_strike_hit";
pub const LIGHTFOOTED: &str = "ty_lee_lightfooted_cast";
pub const BALANCING_ACT: &str = "ty_lee_balancing_act";
pub const BALANCING_ACT_LAND: &str = "ty_lee_balancing_act_land";

fn is_champion_id(sim: &StableSim<'_>, entity: usize) -> bool {
    sim.get_entity(entity)
        .is_some_and(|entity| entity.is_champion())
}

/// One stack of Chi Block; the fourth consumes the set. Only champions have chi
/// to block - minions and towers take the hit itself and nothing more, so the
/// stun needs no tower guard of its own down here.
fn apply_chi_block(sim: &mut StableSim<'_>, caster: usize, target: usize) {
    if !is_champion_id(sim, target) {
        return;
    }

    if buff_stacks(sim, target, CHI_BLOCK_BUFF) + 1 < CHI_BLOCK_MAX_STACKS {
        sim.add_buff(target, &BuffV1::timed(CHI_BLOCK_BUFF, CHI_BLOCK_DURATION));
        return;
    }

    sim.entity_remove_buff(target, CHI_BLOCK_BUFF);
    sim.deal_damage(
        caster,
        target,
        0,
        CHI_BLOCK_BONUS_MAGIC,
        AttackTypeV1::Skill,
    );
    // The damage above is often what kills them, and a burst started on a
    // corpse plays over its death animation instead of their death. A corpse
    // is owed no hold either, so both halves go with it.
    if !sim
        .get_entity(target)
        .is_some_and(|target| target.is_alive() && target.hp().0 > 0)
    {
        return;
    }

    sim.add_buff(
        target,
        &BuffV1::timed(CHI_BLOCK_BREAK_VFX_BUFF, CHI_BLOCK_BREAK_VFX_TICKS),
    );
    // The hold lands as the burst finishes rather than on the hit that set it
    // off, so the flourish is what visibly locks them up instead of playing
    // over a stun already running. The delay is that burst's own length, so
    // the two hand off on the same tick - see `CHI_BLOCK_BREAK_VFX_TICKS`.
    sim.queue_effect(
        CHI_BLOCK_STUN,
        AttackTypeV1::Skill,
        caster,
        &InputTargetV1::target(target),
        CHI_BLOCK_BREAK_VFX_TICKS,
    );
}

/// The hold itself, replayed from the effect queue once the break burst has
/// played out. Split off rather than applied inline because `apply_cc` takes
/// hold the tick it is called, and the burst wants that half-second first.
pub struct ChiBlockStun;

impl StableEffectType for ChiBlockStun {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _caster_id: usize,
        input: InputTargetV1,
    ) {
        // Half a second is long enough to die in, and the burst is not what
        // was holding them - anything could have finished them meanwhile.
        let target = input.target_id;
        if sim
            .get_entity(target)
            .is_some_and(|target| target.is_alive())
        {
            sim.apply_cc(target, &CcV1::stun(CHI_BLOCK_STUN_TICKS));
        }
    }

    fn expected_cc_time(&self) -> Option<usize> {
        Some(CHI_BLOCK_STUN_TICKS as usize)
    }
}

/// Average bonus magic damage per attack, for the AI's damage estimate.
fn chi_block_expected_magic() -> usize {
    CHI_BLOCK_BONUS_MAGIC / CHI_BLOCK_MAX_STACKS
}

pub struct ChiBlocking;

impl StableEffectType for ChiBlocking {
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

        // Balancing Act's mark is cashed in here: the crit rides a buff that
        // goes on and comes off around this one `deal_damage`, so the engine
        // crits the hit with its own multiplier and nothing else of hers ever
        // sees the crit chance.
        let marked = has_buff(sim, caster_id, BALANCE_MARK_BUFF);
        if marked {
            sim.add_buff(caster_id, &mark_crit_buff());
        }

        sim.deal_damage(
            caster_id,
            target,
            percent_of(stat.attack, ATTACK_AD_RATIO),
            0,
            AttackTypeV1::BaseAttack,
        );

        if marked {
            sim.entity_remove_buff(caster_id, BALANCE_MARK_CRIT_BUFF);
            sim.entity_remove_buff(caster_id, BALANCE_MARK_BUFF);
        }
        apply_chi_block(sim, caster_id, target);
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (
            percent_of(caster_stat.attack, ATTACK_AD_RATIO),
            chi_block_expected_magic(),
        )
    }

    fn expected_cc_time(&self) -> Option<usize> {
        Some(CHI_BLOCK_STUN_TICKS as usize / CHI_BLOCK_MAX_STACKS)
    }
}

fn strike_damage(caster_stat: &StatV1) -> usize {
    STRIKE_DAMAGE + percent_of(caster_stat.attack, STRIKE_AD_RATIO)
}

/// One of the three strikes. Skill damage does not crit through the engine's
/// own pipeline, so the roll happens here off the deterministic seed.
///
/// `index` is which of the three this is, and it is what makes them roll
/// independently. The follow-ups arrive as queued effects, and there is no
/// guarantee the engine hands each one a different seed - if it derives the
/// seed from the cast, all three would share it and crit together or not at
/// all, which is not "a chance on each hit". Offsetting by the index is exactly
/// how splitmix64 is meant to be walked, so the three rolls decorrelate.
fn strike(sim: &mut StableSim<'_>, rng_seed: u64, index: u64, caster: usize, target: usize) {
    let stat = stat_of(sim, caster);
    let mut damage = strike_damage(&stat);
    if crate::util::rng_percent(rng_seed.wrapping_add(index)) < stat.crit_chance {
        damage += percent_of(damage, STRIKE_CRIT_BONUS);
    }

    sim.deal_damage(caster, target, damage, 0, AttackTypeV1::Skill);
    apply_chi_block(sim, caster, target);
}

pub struct ThreePointStrike;

impl StableEffectType for ThreePointStrike {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        rng_seed: u64,
        caster_id: usize,
        input: InputTargetV1,
    ) {
        if InputTargetKindV1::from_code(input.kind) != Some(InputTargetKindV1::Target) {
            return;
        }

        strike(sim, rng_seed, 0, caster_id, input.target_id);

        for index in 1..STRIKE_COUNT {
            // `x` carries which strike this is, so the follow-up can offset its
            // own roll off it. Wan's burn passes its scale the same way.
            let queued = InputTargetV1 {
                x: index as u64,
                ..InputTargetV1::target(input.target_id)
            };
            sim.queue_effect(
                THREE_POINT_STRIKE_HIT,
                AttackTypeV1::Skill,
                caster_id,
                &queued,
                index * STRIKE_INTERVAL,
            );
        }
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        let per_strike = strike_damage(caster_stat)
            + percent_of(
                percent_of(strike_damage(caster_stat), STRIKE_CRIT_BONUS),
                caster_stat.crit_chance,
            );
        (
            per_strike * STRIKE_COUNT,
            chi_block_expected_magic() * STRIKE_COUNT,
        )
    }

    fn expected_cc_time(&self) -> Option<usize> {
        Some(CHI_BLOCK_STUN_TICKS as usize * STRIKE_COUNT / CHI_BLOCK_MAX_STACKS)
    }
}

/// The follow-up strikes, replayed from the effect queue.
pub struct ThreePointStrikeHit;

impl StableEffectType for ThreePointStrikeHit {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        rng_seed: u64,
        caster_id: usize,
        input: InputTargetV1,
    ) {
        strike(sim, rng_seed, input.x, caster_id, input.target_id);
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (strike_damage(caster_stat), 0)
    }
}

/// The open window. The reduction is the engine's own basic-attack mitigation,
/// so nothing has to be applied per hit while this is up - and it is read off
/// her crit chance once, at cast, not tracked as it moves.
/// The dodge. `base_attack_damaged_reduce` is the engine's own basic-attack
/// mitigation - a mod cannot cancel an incoming hit itself - and it runs on the
/// buff's own timer, the same three seconds the shield gets. Nothing watches
/// the shield: with the dodge up, basic attacks cannot spend it anyway, so the
/// two only come apart if a skill breaks it early.
pub fn lightfooted_buff() -> BuffV1 {
    let mut buff = BuffV1::timed(LIGHTFOOTED_BUFF, LIGHTFOOTED_DURATION);
    buff.base_attack_damaged_reduce = LIGHTFOOTED_DODGE;
    buff
}

pub fn lightfooted_shield(caster_stat: &StatV1) -> usize {
    LIGHTFOOTED_SHIELD
        + percent_of(caster_stat.attack, LIGHTFOOTED_SHIELD_AD_RATIO)
        + percent_of(caster_stat.hp, LIGHTFOOTED_SHIELD_HP_RATIO)
}

pub struct Lightfooted;

impl StableEffectType for Lightfooted {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        let stat = stat_of(sim, caster_id);

        // Recasting refreshes the window rather than stacking a second one.
        // The shield is left to stack: clearing would also throw away shields
        // she got from somewhere else.
        sim.entity_remove_buff(caster_id, LIGHTFOOTED_BUFF);
        sim.add_buff(caster_id, &lightfooted_buff());
        sim.entity_add_shield(
            caster_id,
            lightfooted_shield(&stat),
            LIGHTFOOTED_DURATION,
        );
    }

    fn expected_buff(&self, _caster_stat: &StatV1) -> Option<BuffV1> {
        Some(lightfooted_buff())
    }

    fn expected_shield(&self, caster_stat: &StatV1) -> usize {
        lightfooted_shield(caster_stat)
    }

    fn expected_move_distance(&self) -> Option<(usize, u64)> {
        Some((1, LIGHTFOOTED_RANGE))
    }

    fn on_caster(&self) -> bool {
        true
    }

    fn can_move(&self) -> bool {
        true
    }
}

/// Every enemy champion inside `range` already carrying Chi Block, as
/// `(stacks, distance squared, id)`. Only champions can be stacked, and an
/// unstacked one is no more interesting to her than any other target, so this
/// is the whole set of openings the AI steers by.
pub fn stacked_enemies(
    sim: &StableSim<'_>,
    caster: usize,
    range: u64,
) -> Vec<(usize, u64, usize)> {
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
        .map(|id| (buff_stacks(sim, id, CHI_BLOCK_BUFF), sim.distance_sq(caster, id), id))
        .filter(|&(stacks, distance_sq, _)| stacks > 0 && distance_sq <= range_sq)
        .collect()
}

/// Lightfooted goes to whatever is closest, either team, towers excluded - a
/// tower is the one unit a dash should never throw her onto.
pub fn nearest_unit(sim: &StableSim<'_>, caster: usize, range: u64) -> Option<usize> {
    let range_sq = range.saturating_mul(range);

    (0..sim.entity_count())
        .filter_map(|index| sim.entity_at(index))
        .filter(|entity| entity.is_alive() && !entity.is_tower())
        .map(|entity| entity.id())
        .filter(|&id| id != caster && sim.distance_sq(caster, id) <= range_sq)
        // Ties resolve by entity order, which is deterministic.
        .min_by_key(|&id| sim.distance_sq(caster, id))
}

/// The closest enemy champion, stacked or not.
///
/// `opening` only ever names champions already carrying Chi Block, and
/// `nearest_unit` counts minions. The combo needs neither: it engages champions
/// and nothing else, whether or not a stack has landed on them yet, and leaves
/// waves to the base AI.
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

fn balance_damage(caster_stat: &StatV1) -> usize {
    BALANCE_DAMAGE
        + percent_of(caster_stat.attack, BALANCE_AD_RATIO)
        + percent_of(caster_stat.crit_chance, BALANCE_CRIT_RATIO)
}

/// Ty Lee's ult aims itself: the player AI rewrites the cast onto whoever this
/// picks, so the engine dashes her at the champion the ability advertises.
pub fn highest_hp_enemy_champion(
    sim: &StableSim<'_>,
    caster: usize,
    range: u64,
) -> Option<usize> {
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
        .max_by_key(|&id| sim.get_entity(id).map_or(0, |entity| entity.hp().0))
}

/// Names the target the dash owes, so the arrival watch knows who to hit.
pub fn inbound_buff(target: usize) -> BuffV1 {
    BuffV1::timed(
        &format!("{BALANCE_INBOUND_PREFIX}|{target}"),
        BALANCE_INBOUND_TIMEOUT,
    )
}

pub fn parse_inbound(name: &str) -> Option<usize> {
    name.strip_prefix(BALANCE_INBOUND_PREFIX)?
        .strip_prefix('|')?
        .parse()
        .ok()
}

/// What the dash owes on arrival, to everything around where she lands. The
/// area is centred on her because that is where the dash puts her - on her
/// target - and the target is added in regardless, since the ability promises
/// it is hit and a radius should not be what decides that.
pub fn land_balancing_act(sim: &mut StableSim<'_>, caster: usize, target: usize) {
    let damage = balance_damage(&stat_of(sim, caster));

    let mut caught = crate::util::enemies_near(sim, caster, caster, BALANCE_AOE_RADIUS);
    if !caught.contains(&target) {
        caught.push(target);
    }

    for hit in caught {
        let Some(entity) = sim.get_entity(hit) else {
            continue;
        };
        if !entity.is_alive() {
            continue;
        }
        // Only champions are silenced - the rest just take the hit.
        let champion = entity.is_champion();

        sim.deal_damage(caster, hit, damage, 0, AttackTypeV1::Skill);
        if champion {
            sim.apply_cc(
                hit,
                &CcV1::of_kind(CcKindV1::BlockSkill, BALANCE_SILENCE_TICKS),
            );
            sim.apply_cc(
                hit,
                &CcV1::of_kind(CcKindV1::BlockMoveSkill, BALANCE_SILENCE_TICKS),
            );
        }
    }
}

/// The mark itself: a plain eight-second marker, no stats on it at all.
pub fn mark_buff() -> BuffV1 {
    BuffV1::timed(BALANCE_MARK_BUFF, BALANCE_MARK_DURATION)
}

/// What the mark is spent on, worn only for the instant the attack lands.
fn mark_crit_buff() -> BuffV1 {
    let mut buff = BuffV1::named(BALANCE_MARK_CRIT_BUFF);
    buff.crit_chance = BALANCE_MARK_CRIT as i32;
    buff
}

/// The arrival half of Balancing Act, fired by the `MoveTo`'s `end_effects` on
/// the tick the dash actually lands - so the damage and the silence hit when
/// she gets there, however far she had to travel. The target is read back off
/// the marker the cast left rather than re-picked here: by now she has moved,
/// and the healthiest champion near where she landed need not be the one she
/// dashed at.
pub struct BalancingActLand;

impl StableEffectType for BalancingActLand {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        let Some((marker, target)) = sim.get_entity(caster_id).and_then(|caster| {
            (0..caster.buff_count())
                .filter_map(|index| caster.buff_at(index))
                .find_map(|buff| {
                    let name = buff.name();
                    parse_inbound(name).map(|target| (name.to_string(), target))
                })
        }) else {
            return;
        };

        // Off first, so a dash that somehow ends twice cannot hit twice.
        sim.entity_remove_buff(caster_id, &marker);
        if sim.get_entity(target).is_some_and(|target| target.is_alive()) {
            land_balancing_act(sim, caster_id, target);
        }
    }

    fn on_caster(&self) -> bool {
        true
    }
}

pub struct BalancingAct;

impl StableEffectType for BalancingAct {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        input: InputTargetV1,
    ) {
        // The mark lands either way, so an ult that loses its target on the way
        // in still buys her a guaranteed crit and a double Chi Block.
        sim.add_buff(caster_id, &mark_buff());

        // `casting_target` is EnemyChampion, so the engine has already screened
        // the input down to a living enemy champion. Noting it here is what
        // lets the landing half hit whoever she actually dashed at, rather than
        // re-picking from wherever she ends up. The damage and the silence are
        // owed on arrival - see `BALANCE_INBOUND_PREFIX`.
        let target = match InputTargetKindV1::from_code(input.kind) {
            Some(InputTargetKindV1::Target) => Some(input.target_id),
            _ => highest_hp_enemy_champion(sim, caster_id, BALANCE_RANGE),
        };
        if let Some(target) = target {
            sim.add_buff(caster_id, &inbound_buff(target));
        }
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (balance_damage(caster_stat), 0)
    }

    fn expected_buff(&self, _caster_stat: &StatV1) -> Option<BuffV1> {
        Some(mark_buff())
    }

    fn expected_cc_time(&self) -> Option<usize> {
        Some(BALANCE_SILENCE_TICKS as usize)
    }

    fn expected_move_distance(&self) -> Option<(usize, u64)> {
        Some((1, BALANCE_RANGE))
    }

    fn on_caster(&self) -> bool {
        true
    }

    fn can_move(&self) -> bool {
        true
    }
}
