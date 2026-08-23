use mod_api_stable::*;

use super::constants::*;
use super::element::{self, Element, CYCLE};
use crate::util::{has_buff, percent_of, stat_of};

pub const AVATAR_CYCLE: &str = "avatar_wan_avatar_cycle";
pub const SPIRIT_STEP: &str = "avatar_wan_spirit_step";
pub const HARMONIC_CONVERGENCE: &str = "avatar_wan_harmonic_convergence";
pub const FIRE_BURN_TICK: &str = "avatar_wan_fire_burn_tick";

pub struct SoulOfRaava(pub Option<Element>);

pub const ATTACK_HITS: [(&str, Option<Element>); 5] = [
    ("avatar_wan_attack_hit_air", Some(Element::Air)),
    ("avatar_wan_attack_hit_water", Some(Element::Water)),
    ("avatar_wan_attack_hit_earth", Some(Element::Earth)),
    ("avatar_wan_attack_hit_fire", Some(Element::Fire)),
    ("avatar_wan_attack_hit_convergence", None),
];

impl StableEffectType for SoulOfRaava {
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
        let converged = has_buff(sim, caster_id, CONVERGENCE_BUFF);
        let launched = self.0.unwrap_or(STARTING_ELEMENT);

        if element::current(sim, caster_id).is_none() {
            element::attune(sim, caster_id, launched);
        }

        sim.deal_damage(
            caster_id,
            target,
            percent_of(stat.attack, ATTACK_PHYSICAL_SHARE),
            percent_of(stat.attack, ATTACK_MAGIC_SHARE),
            AttackTypeV1::BaseAttack,
        );

        if converged {
            for candidate in CYCLE {
                element::proc(
                    sim,
                    caster_id,
                    target,
                    &stat,
                    candidate,
                    CONVERGENCE_BASE_SCALE,
                );
            }
        } else {
            element::proc(
                sim,
                caster_id,
                target,
                &stat,
                launched,
                CONVERGENCE_BASE_SCALE,
            );
        }
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        let (physical, magic) = match self.0 {
            Some(element) => element::proc_damage(caster_stat, element, CONVERGENCE_BASE_SCALE),
            // Converged: every attack procs all four at once.
            None => element::off_element_attack_damage(caster_stat),
        };
        (
            percent_of(caster_stat.attack, ATTACK_PHYSICAL_SHARE) + physical,
            percent_of(caster_stat.attack, ATTACK_MAGIC_SHARE) + magic,
        )
    }

    fn expected_heal(&self, caster_stat: &StatV1) -> usize {
        element::proc_heal(
            caster_stat,
            self.0.unwrap_or(Element::Water),
            CONVERGENCE_BASE_SCALE,
        )
    }
}

pub struct FireBurnTick;

impl StableEffectType for FireBurnTick {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        input: InputTargetV1,
    ) {
        let scale = if input.x == 0 {
            CONVERGENCE_BASE_SCALE
        } else {
            input.x as usize
        };
        let stat = stat_of(sim, caster_id);
        let damage = element::burn_tick_damage(&stat, scale);
        sim.deal_damage(caster_id, input.target_id, 0, damage, AttackTypeV1::Dot);
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        (
            0,
            element::burn_tick_damage(caster_stat, CONVERGENCE_BASE_SCALE),
        )
    }
}

pub struct TheAvatarCycle;

impl StableEffectType for TheAvatarCycle {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        let next = element::current(sim, caster_id)
            .unwrap_or(STARTING_ELEMENT)
            .next();
        element::attune(sim, caster_id, next);
    }

    fn on_caster(&self) -> bool {
        true
    }

    fn can_move(&self) -> bool {
        true
    }
}

pub struct SpiritStep;

impl SpiritStep {
    fn buff(caster_stat: &StatV1) -> BuffV1 {
        let mut buff = BuffV1::timed(STEP_BUFF, STEP_BUFF_DURATION);
        buff.attack_speed_mult = STEP_ATTACK_SPEED_PERCENT
            + percent_of(caster_stat.attack, STEP_ATTACK_SPEED_AD_RATIO) as i32;
        buff
    }
}

impl StableEffectType for SpiritStep {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        let hp = sim.get_entity(caster_id).map_or(0, |entity| entity.hp().0);
        let stat = stat_of(sim, caster_id);

        sim.add_buff(caster_id, &Self::buff(&stat));
        sim.add_buff(caster_id, &super::tick::ledger_buff(0, hp));
    }

    fn expected_buff(&self, caster_stat: &StatV1) -> Option<BuffV1> {
        Some(Self::buff(caster_stat))
    }

    fn expected_move_distance(&self) -> Option<(usize, u64)> {
        Some((STEP_DASH_TICKS, STEP_DASH_DISTANCE))
    }

    fn expected_heal(&self, caster_stat: &StatV1) -> usize {
        STEP_HEAL_FLAT
            + percent_of(
                percent_of(caster_stat.hp, STEP_STORE_PERCENT),
                STEP_HEAL_PERCENT,
            ) / 4
    }

    fn on_caster(&self) -> bool {
        true
    }

    fn can_move(&self) -> bool {
        true
    }
}

pub struct HarmonicConvergence;

pub fn convergence_buff(ticks: usize) -> BuffV1 {
    BuffV1::timed(CONVERGENCE_BUFF, ticks)
}

impl HarmonicConvergence {
    fn buff() -> BuffV1 {
        convergence_buff(CONVERGENCE_DURATION)
    }

    fn shield_amount(caster_stat: &StatV1) -> usize {
        CONVERGENCE_SHIELD
            + percent_of(caster_stat.magic_power, CONVERGENCE_SHIELD_AP_RATIO)
            + percent_of(caster_stat.hp, CONVERGENCE_SHIELD_HP_RATIO)
    }
}

impl StableEffectType for HarmonicConvergence {
    fn apply(
        &self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        caster_id: usize,
        _input: InputTargetV1,
    ) {
        let stat = stat_of(sim, caster_id);

        sim.add_buff(caster_id, &Self::buff());
        sim.entity_add_shield(
            caster_id,
            Self::shield_amount(&stat),
            CONVERGENCE_SHIELD_DURATION,
        );
    }

    fn expected_buff(&self, _caster_stat: &StatV1) -> Option<BuffV1> {
        Some(Self::buff())
    }

    fn expected_damage(&self, caster_stat: &StatV1) -> (usize, usize) {
        element::convergence_damage(caster_stat)
    }

    fn expected_shield(&self, caster_stat: &StatV1) -> usize {
        Self::shield_amount(caster_stat)
    }

    fn expected_heal(&self, caster_stat: &StatV1) -> usize {
        element::convergence_heal(caster_stat)
    }

    fn on_caster(&self) -> bool {
        true
    }

    fn can_move(&self) -> bool {
        true
    }
}
