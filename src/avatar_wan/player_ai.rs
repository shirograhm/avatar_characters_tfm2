use mod_api_stable::*;

use super::constants::*;
use super::CHAMPION_KEY;
use crate::util::has_buff;

enum Plan {
    Keep,
    Attack(usize),
    Disengage,
    Dash((u64, u64), (i64, i64)),
}

pub struct AggressiveWan;

impl AggressiveWan {
    fn committed(sim: &StableSim<'_>, entity: usize) -> bool {
        has_buff(sim, entity, STEP_BUFF) || has_buff(sim, entity, CONVERGENCE_BUFF)
    }

    fn converging(sim: &StableSim<'_>, entity: usize) -> bool {
        has_buff(sim, entity, CONVERGENCE_BUFF)
    }

    fn effective_hp_percent(sim: &StableSim<'_>, entity: usize) -> Option<usize> {
        let entity = sim.get_entity(entity)?;
        let (hp, max_hp) = entity.hp();
        (max_hp > 0).then(|| (hp + entity.shield()) * 100 / max_hp)
    }

    fn aim_point(sim: &StableSim<'_>, from: (u64, u64), input: &InputV1) -> Option<(u64, u64)> {
        match InputTargetKindV1::from_code(input.target.kind) {
            Some(InputTargetKindV1::Target) => sim
                .get_entity(input.target.target_id)
                .map(|target| target.pos()),
            Some(InputTargetKindV1::Pos) => Some((input.target.x, input.target.y)),
            Some(InputTargetKindV1::Dir) => Some((
                from.0.saturating_add_signed(input.target.dir_x),
                from.1.saturating_add_signed(input.target.dir_y),
            )),
            _ => ((input.x, input.y) != (0, 0)).then_some((input.x, input.y)),
        }
    }

    fn any_enemy_near(sim: &StableSim<'_>, entity: usize, range: u64) -> Option<usize> {
        crate::util::enemies_near(sim, entity, entity, range)
            .into_iter()
            // Ties resolve by entity order, which is deterministic.
            .min_by_key(|&id| sim.distance_sq(entity, id))
    }

    fn enemy_champions_near(sim: &StableSim<'_>, entity: usize, range: u64) -> Vec<(u64, usize)> {
        let Some(team) = sim.get_entity(entity).map(|me| me.team()) else {
            return Vec::new();
        };
        let range_sq = range.saturating_mul(range);

        (0..sim.champion_count())
            .map(|index| sim.champion_id_at(index))
            .filter(|&id| {
                sim.get_entity(id)
                    .is_some_and(|other| other.is_alive() && other.team() != team)
            })
            .map(|id| (sim.distance_sq(entity, id), id))
            .filter(|&(distance_sq, _)| distance_sq <= range_sq)
            .collect()
    }

    fn enemy_champion_near(sim: &StableSim<'_>, entity: usize, range: u64) -> Option<usize> {
        Self::enemy_champions_near(sim, entity, range)
            .into_iter()
            // Ties resolve by champion order, which is deterministic.
            .min_by_key(|&(distance_sq, _)| distance_sq)
            .map(|(_, id)| id)
    }

    fn weakest_champion_near(sim: &StableSim<'_>, entity: usize, range: u64) -> Option<usize> {
        Self::enemy_champions_near(sim, entity, range)
            .into_iter()
            .filter_map(|(distance_sq, id)| {
                Self::effective_hp_percent(sim, id).map(|hp| (hp, distance_sq, id))
            })
            .filter(|&(hp, _, _)| hp < AGGRO_ULT_FOCUS_HP)
            .min_by_key(|&(hp, distance_sq, _)| (hp, distance_sq))
            .map(|(_, _, id)| id)
    }
}

impl StablePlayerAi for AggressiveWan {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(AggressiveWan)
    }

    fn id(&self) -> String {
        "avatar_wan_aggression".to_string()
    }

    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name == CHAMPION_KEY
    }

    fn think(
        &mut self,
        ctx: &mut StableAiContext<'_>,
        base_input: Option<InputV1>,
    ) -> Option<InputV1> {
        let base_kind = base_input.and_then(|input| InputKindV1::from_code(input.kind));

        if !matches!(
            base_kind,
            Some(InputKindV1::Move)
                | Some(InputKindV1::Return)
                | Some(InputKindV1::Attack)
                | Some(InputKindV1::Skill)
                | Some(InputKindV1::Skill2)
                | None
        ) {
            return None;
        }

        let player = ctx.player_id();
        let hp_percent = ctx.hp_ratio_percent()?;

        let plan = {
            let sim = ctx.sim()?;
            let me = sim
                .get_player(player)
                .and_then(|player| player.champion())?;
            let (my_pos, my_radius, me) = (me.pos(), me.radius() as u64, me.id());

            let committed = Self::committed(&sim, me);
            let converging = Self::converging(&sim, me);
            let in_combat = Self::enemy_champion_near(&sim, me, AGGRO_COMBAT_RANGE).is_some();

            if base_kind == Some(InputKindV1::Skill2) {
                if !in_combat && hp_percent < AGGRO_LOW_HP {
                    Plan::Disengage
                } else {
                    match base_input
                        .and_then(|input| Self::aim_point(&sim, my_pos, &input))
                        .and_then(|aim| {
                            crate::util::full_step_toward(
                                my_pos,
                                aim,
                                STEP_DASH_DISTANCE,
                                my_radius,
                            )
                        }) {
                        Some((dest, offset)) => Plan::Dash(dest, offset),
                        None => Plan::Keep,
                    }
                }
            } else if base_kind == Some(InputKindV1::Skill) && !converging {
                Plan::Keep
            } else if converging && hp_percent < AGGRO_ULT_HP_FLOOR {
                Plan::Disengage
            } else if converging {
                match Self::weakest_champion_near(&sim, me, AGGRO_ULT_ENGAGE_RANGE) {
                    Some(target) => Plan::Attack(target),
                    None if base_kind == Some(InputKindV1::Attack) => Plan::Keep,
                    None => match Self::enemy_champion_near(&sim, me, AGGRO_ULT_ENGAGE_RANGE)
                        .or_else(|| Self::any_enemy_near(&sim, me, ATTACK_RANGE))
                    {
                        Some(target) => Plan::Attack(target),
                        None => Plan::Keep,
                    },
                }
            } else if !committed && base_kind != Some(InputKindV1::Move) {
                Plan::Keep
            } else {
                let floor = if committed {
                    AGGRO_COMMITTED_HP_FLOOR
                } else {
                    AGGRO_HP_FLOOR
                };

                let reach = if committed {
                    AGGRO_ENGAGE_RANGE
                } else {
                    ATTACK_RANGE
                };

                match Self::enemy_champion_near(&sim, me, reach) {
                    Some(target) if hp_percent >= floor => Plan::Attack(target),
                    _ => Plan::Keep,
                }
            }
        };

        match plan {
            Plan::Keep => None,
            Plan::Attack(target) => {
                let input = InputV1::action(InputKindV1::Attack, InputTargetV1::target(target));
                ctx.is_valid_input(&input).then_some(input)
            }
            Plan::Dash(dest, offset) => {
                let mut aimed =
                    InputV1::action(InputKindV1::Skill2, InputTargetV1::pos(dest.0, dest.1));
                aimed.x = dest.0;
                aimed.y = dest.1;
                if ctx.is_valid_input(&aimed) {
                    return Some(aimed);
                }

                let headed =
                    InputV1::action(InputKindV1::Skill2, InputTargetV1::dir(offset.0, offset.1));
                ctx.is_valid_input(&headed).then_some(headed)
            }
            Plan::Disengage => {
                if ctx.is_safe_to_recall() {
                    ctx.recall_input()
                } else {
                    ctx.run_away_without_skill_input()
                }
            }
        }
    }
}
