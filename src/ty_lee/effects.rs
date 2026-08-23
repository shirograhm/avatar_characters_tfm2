use mod_api_stable::*;

use super::*;
use crate::util::{buff_stacks, has_buff, percent_of, stat_of};

pub const CHI_BLOCK_HIT: &str = "ty_lee_chi_block_hit";
pub const THREE_POINT_STRIKE: &str = "ty_lee_three_point_strike";
pub const THREE_POINT_STRIKE_HIT: &str = "ty_lee_three_point_strike_hit";
pub const CIRCUS_FREAK: &str = "ty_lee_circus_freak_cast";
pub const BALANCING_ACT: &str = "ty_lee_balancing_act";

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
    sim.apply_cc(target, &CcV1::stun(CHI_BLOCK_STUN_TICKS));
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

        // Balancing Act's mark: the buff itself carries +100 crit, so the
        // engine crits this hit for us. It has to still be on her when
        // `deal_damage` rolls, so the buff is spent below and not here.
        let marked = has_buff(sim, caster_id, BALANCE_MARK_BUFF);

        sim.deal_damage(
            caster_id,
            target,
            percent_of(stat.attack, ATTACK_AD_RATIO),
            0,
            AttackTypeV1::BaseAttack,
        );

        if marked {
            sim.entity_remove_buff(caster_id, BALANCE_MARK_BUFF);
        }
        let blocks = if marked { BALANCE_MARK_CHI_BLOCKS } else { 1 };
        for _ in 0..blocks {
            apply_chi_block(sim, caster_id, target);
        }
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
fn strike(sim: &mut StableSim<'_>, rng_seed: u64, caster: usize, target: usize) {
    let stat = stat_of(sim, caster);
    let mut damage = strike_damage(&stat);
    if crate::util::rng_percent(rng_seed) < stat.crit_chance {
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

        strike(sim, rng_seed, caster_id, input.target_id);

        let queued = InputTargetV1::target(input.target_id);
        for index in 1..STRIKE_COUNT {
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
        strike(sim, rng_seed, caster_id, input.target_id);
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (strike_damage(caster_stat), 0)
    }
}

/// The open window. The reduction is the engine's own basic-attack mitigation,
/// so nothing has to be applied per hit while this is up - and it is read off
/// her crit chance once, at cast, not tracked as it moves.
pub fn evasion_buff(crit_chance: usize) -> BuffV1 {
    let mut buff = BuffV1::timed(EVASION_BUFF, EVASION_DURATION);
    buff.base_attack_damaged_reduce = crit_chance.min(100);
    buff
}

pub fn evasion_shield(caster_stat: &StatV1) -> usize {
    EVASION_SHIELD + percent_of(caster_stat.attack, EVASION_SHIELD_AD_RATIO)
}

pub struct CircusFreak;

impl StableEffectType for CircusFreak {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        let stat = stat_of(sim, caster_id);

        // Recasting inside an open window restarts it rather than stacking a
        // second one, so the shield still lands exactly once, at the new end.
        sim.entity_remove_buff(caster_id, EVASION_BUFF);
        sim.entity_remove_buff(caster_id, EVASION_PENDING_BUFF);
        sim.add_buff(caster_id, &evasion_buff(stat.crit_chance));
        sim.add_buff(caster_id, &BuffV1::named(EVASION_PENDING_BUFF));
    }

    fn expected_buff(&self, caster_stat: &StatV1) -> Option<BuffV1> {
        Some(evasion_buff(caster_stat.crit_chance))
    }

    fn expected_shield(&self, caster_stat: &StatV1) -> usize {
        evasion_shield(caster_stat)
    }

    fn on_caster(&self) -> bool {
        true
    }

    fn can_move(&self) -> bool {
        true
    }
}

fn balance_damage(caster_stat: &StatV1) -> usize {
    BALANCE_DAMAGE + percent_of(caster_stat.attack, BALANCE_AD_RATIO)
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

pub fn mark_buff() -> BuffV1 {
    let mut buff = BuffV1::timed(BALANCE_MARK_BUFF, BALANCE_MARK_DURATION);
    buff.crit_chance = 100;
    buff
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

        if InputTargetKindV1::from_code(input.kind) != Some(InputTargetKindV1::Target) {
            return;
        }
        let target = input.target_id;

        let stat = stat_of(sim, caster_id);
        sim.deal_damage(
            caster_id,
            target,
            balance_damage(&stat),
            0,
            AttackTypeV1::Skill,
        );

        // `casting_target` is EnemyChampion, so the engine has already screened
        // this down to a living enemy champion.
        sim.apply_cc(
            target,
            &CcV1::of_kind(CcKindV1::BlockSkill, BALANCE_SILENCE_TICKS),
        );
        sim.apply_cc(
            target,
            &CcV1::of_kind(CcKindV1::BlockMoveSkill, BALANCE_SILENCE_TICKS),
        );
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

    /// What actually carries her to the target. Nightmare's teleport ult takes
    /// the same knob as a `speed` field on its native effect.
    fn linear_move_speed(&self) -> Option<usize> {
        Some(BALANCE_DASH_SPEED)
    }

    fn on_caster(&self) -> bool {
        true
    }

    fn can_move(&self) -> bool {
        true
    }
}
