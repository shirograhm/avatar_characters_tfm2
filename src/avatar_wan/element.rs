use mod_api_stable::*;

use super::constants::*;
use crate::util::{buff_stacks, percent_of};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Element {
    Air,
    Water,
    Earth,
    Fire,
}

pub const CYCLE: [Element; 4] = [Element::Air, Element::Water, Element::Earth, Element::Fire];

impl Element {
    pub fn buff_name(self) -> &'static str {
        match self {
            Element::Air => "wan_element_air",
            Element::Water => "wan_element_water",
            Element::Earth => "wan_element_earth",
            Element::Fire => "wan_element_fire",
        }
    }

    fn from_buff_name(name: &str) -> Option<Self> {
        CYCLE
            .into_iter()
            .find(|element| element.buff_name() == name)
    }

    pub fn next(self) -> Self {
        let index = CYCLE
            .iter()
            .position(|&element| element == self)
            .unwrap_or(0);
        CYCLE[(index + 1) % CYCLE.len()]
    }
}

pub fn current(sim: &StableSim<'_>, entity: usize) -> Option<Element> {
    current_of(&sim.get_entity(entity)?)
}

pub fn current_of(entity: &StableEntity<'_, '_>) -> Option<Element> {
    (0..entity.buff_count())
        .filter_map(|index| entity.buff_at(index))
        .find_map(|buff| Element::from_buff_name(buff.name()))
}

pub fn attune(sim: &mut StableSim<'_>, entity: usize, element: Element) {
    for stale in CYCLE {
        sim.entity_remove_buff(entity, stale.buff_name());
    }
    sim.add_buff(entity, &BuffV1::named(element.buff_name()));
}

pub fn proc(
    sim: &mut StableSim<'_>,
    caster: usize,
    target: usize,
    caster_stat: &StatV1,
    element: Element,
    scale: usize,
) {
    match element {
        Element::Air => {
            if buff_stacks(sim, caster, AIR_STACK_BUFF) >= AIR_MAX_STACKS {
                return;
            }
            let mut swiftness = BuffV1::timed(AIR_STACK_BUFF, AIR_DURATION);
            swiftness.move_speed_mult = percent_of(AIR_MOVE_SPEED_PERCENT as usize, scale) as i32;
            sim.add_buff(caster, &swiftness);
        }

        Element::Water => {
            let Some((current, max)) = sim.get_entity(caster).map(|entity| entity.hp()) else {
                return;
            };
            sim.heal(
                caster,
                caster,
                water_heal(max.saturating_sub(current), scale),
            );

            sim.add_buff(
                target,
                &BuffV1::timed(WATER_SPLASH_VFX_BUFF, WATER_SPLASH_VFX_TICKS),
            );
        }

        Element::Earth => {
            let (physical, magic) = earth_splash_damage(caster_stat, scale);
            for splashed in crate::util::enemies_near(sim, caster, target, EARTH_RADIUS) {
                // Towers do not take the splash, the same way they do not burn.
                if sim
                    .get_entity(splashed)
                    .is_some_and(|entity| entity.is_tower())
                {
                    continue;
                }
                sim.deal_damage(caster, splashed, physical, magic, AttackTypeV1::BaseAttack);
            }

            sim.add_buff(
                target,
                &BuffV1::timed(EARTH_SPLASH_VFX_BUFF, EARTH_SPLASH_VFX_TICKS),
            );
        }

        Element::Fire => {
            // Towers do not burn
            if sim
                .get_entity(target)
                .is_some_and(|entity| entity.is_tower())
            {
                return;
            }

            let input = InputTargetV1 {
                x: scale as u64,
                ..InputTargetV1::target(target)
            };
            for tick in 1..=BURN_TICKS {
                sim.queue_effect(
                    super::effects::FIRE_BURN_TICK,
                    AttackTypeV1::Dot,
                    caster,
                    &input,
                    tick * BURN_TICK_INTERVAL,
                );
            }

            sim.add_buff(target, &BuffV1::timed(BURN_VFX_BUFF, BURN_VFX_TICKS));
        }
    }
}

pub fn earth_splash_damage(caster_stat: &StatV1, scale: usize) -> (usize, usize) {
    let total = percent_of(percent_of(caster_stat.attack, EARTH_ATTACK_SHARE), scale);
    (
        percent_of(total, ATTACK_PHYSICAL_SHARE),
        percent_of(total, ATTACK_MAGIC_SHARE),
    )
}

pub fn proc_damage(caster_stat: &StatV1, element: Element, scale: usize) -> (usize, usize) {
    match element {
        Element::Air | Element::Water => (0, 0),
        Element::Earth => earth_splash_damage(caster_stat, scale),
        Element::Fire => (0, burn_tick_damage(caster_stat, scale) * BURN_TICKS),
    }
}

pub fn proc_heal(caster_stat: &StatV1, element: Element, scale: usize) -> usize {
    match element {
        Element::Water => water_heal(
            percent_of(caster_stat.hp, EXPECTED_MISSING_HP_PERCENT),
            scale,
        ),
        _ => 0,
    }
}

pub fn convergence_attacks() -> usize {
    CONVERGENCE_DURATION / ATTACK_COOLTIME
}

pub fn convergence_heal(caster_stat: &StatV1) -> usize {
    proc_heal(caster_stat, Element::Water, CONVERGENCE_BASE_SCALE) * convergence_attacks()
}

pub fn convergence_damage(caster_stat: &StatV1) -> (usize, usize) {
    let (physical, magic) = off_element_attack_damage(caster_stat);
    let attacks = convergence_attacks();
    (physical * attacks, magic * attacks)
}

pub fn off_element_attack_damage(caster_stat: &StatV1) -> (usize, usize) {
    let (physical, earth_magic) = earth_splash_damage(caster_stat, CONVERGENCE_BASE_SCALE);
    let burn = burn_tick_damage(caster_stat, CONVERGENCE_BASE_SCALE) * BURN_TICKS;
    (physical, earth_magic + burn)
}

pub fn water_heal(missing_hp: usize, scale: usize) -> usize {
    percent_of(
        WATER_HEAL_FLAT + percent_of(missing_hp, WATER_MISSING_HP_PERCENT),
        scale,
    )
}

pub fn burn_tick_damage(caster_stat: &StatV1, scale: usize) -> usize {
    let total = BURN_DAMAGE + percent_of(caster_stat.magic_power, BURN_AP_RATIO);
    percent_of(total / BURN_TICKS, scale)
}
