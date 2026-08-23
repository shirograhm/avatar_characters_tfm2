//! Balancing Act aims itself.
//!
//! The ability is written as "dashes to the highest health target", but the
//! engine will only move a champion for an action it has a cast target for, so
//! the ult is a `Targeting` skill and the pick has to happen before the cast
//! rather than inside the effect. This hook rewrites the ult input onto the
//! champion the description promises.
//!
//! It only touches Ty Lee's own ult inputs - every other input, and every other
//! champion, falls through untouched. A human player aiming the ult by hand is
//! not overridden either; this runs on the AI's input.

use mod_api_stable::*;

use super::effects::highest_hp_enemy_champion;
use super::*;

pub struct AimBalancingAct;

impl StablePlayerAi for AimBalancingAct {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(AimBalancingAct)
    }

    fn id(&self) -> String {
        "ty_lee_balancing_act_aim".to_string()
    }

    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name == CHAMPION_KEY
    }

    fn think(
        &mut self,
        ctx: &mut StableAiContext<'_>,
        base_input: Option<InputV1>,
    ) -> Option<InputV1> {
        let input = base_input?;
        if InputKindV1::from_code(input.kind) != Some(InputKindV1::Ult) {
            return None;
        }

        let player = ctx.player_id();
        let sim = ctx.sim()?;
        let me = sim
            .get_player(player)
            .and_then(|player| player.champion())?
            .id();

        let target = highest_hp_enemy_champion(&sim, me, BALANCE_RANGE)?;
        // Already aimed where it should be - leave the AI's own input alone.
        if InputTargetKindV1::from_code(input.target.kind) == Some(InputTargetKindV1::Target)
            && input.target.target_id == target
        {
            return None;
        }

        Some(InputV1::action(
            InputKindV1::Ult,
            InputTargetV1::target(target),
        ))
    }
}
