//! Ty Lee aims her own targets, and steers her kit toward the Chi Block stun.
//!
//! Two separate jobs share this hook.
//!
//! Her two dashes are aimed differently because they are built on different
//! engine effects. Balancing Act rides `MoveToTarget`, so it is a `Targeting`
//! cast and needs a champion: this picks the healthiest one in range, which is
//! what the ability promises. Lightfooted rides `MoveTo`, which follows the
//! cast's heading rather than a target, so it is aimed as a point - stepped
//! exactly as far as the unit it is going to, so she stops on them.
//!
//! The rest is the stun. Her whole kit feeds one payoff - four applications of
//! Chi Block on the same enemy - and the base AI picks on damage and threat,
//! which knows nothing about that count. So this reads the stacks already on
//! nearby enemies and reaches for whichever ability finishes the set: Three
//! Point Strike lands three more applications, so from one stack up it closes
//! the set on its own; at three, a plain basic attack is the fourth; and if the
//! stacked enemy is out of reach entirely, Lightfooted closes the gap first.
//!
//! It only ever changes *who* an action points at, never *which* action she
//! takes. There is no way to ask the engine whether she is already mid-cast, so
//! a hook that answered "cast Three Point Strike" would keep answering it on
//! every tick of that cast, re-issuing the input into an action already running.
//! Re-aiming what the base AI has already decided to do cannot do that.
//!
//! Every pick is allowed to decline. Returning `None` leaves the base AI's own
//! choice alone, which is what happens whenever no opening is in reach.
//!
//! It only rewrites Ty Lee's own combat actions. `Move` and `Return` fall
//! through untouched - hijacking those would fight the base AI's positioning
//! and pull her out of its retreats. A human player aiming by hand is not
//! overridden either; this runs on the AI's input.

use std::cmp::Reverse;

use mod_api_stable::*;

use super::effects::{highest_hp_enemy_champion, nearest_unit, stacked_enemies};
use super::*;

/// `None` when the input already says exactly this, so an unchanged pick is
/// left as the base AI's rather than replaced with an identical one.
fn aim(kind: InputKindV1, input: &InputV1, target: usize) -> Option<InputV1> {
    let unchanged = InputKindV1::from_code(input.kind) == Some(kind)
        && InputTargetKindV1::from_code(input.target.kind) == Some(InputTargetKindV1::Target)
        && input.target.target_id == target;

    (!unchanged).then(|| InputV1::action(kind, InputTargetV1::target(target)))
}

/// The best opening on the board: the enemy closest to a full set of Chi
/// Block, nearest first among equals. `within` bounds how far out to look.
fn opening(sim: &StableSim<'_>, me: usize, within: u64) -> Option<usize> {
    stacked_enemies(sim, me, within)
        .into_iter()
        // Most stacks first, then closest. Ties resolve by champion order,
        // which is deterministic.
        .min_by_key(|&(stacks, distance_sq, id)| (Reverse(stacks), distance_sq, id))
        .map(|(_, _, id)| id)
}

/// Where an action should point: at a unit, or along a heading toward a point.
enum Plan {
    At(usize),
    Toward(((u64, u64), (i64, i64))),
}

pub struct AimTyLee;

impl StablePlayerAi for AimTyLee {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(AimTyLee)
    }

    fn id(&self) -> String {
        "ty_lee_aim".to_string()
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
        let kind = InputKindV1::from_code(input.kind)?;
        if !matches!(
            kind,
            InputKindV1::Attack | InputKindV1::Skill | InputKindV1::Skill2 | InputKindV1::Ult
        ) {
            return None;
        }

        let player = ctx.player_id();
        let plan = {
            let sim = ctx.sim()?;
            let me = sim.get_player(player).and_then(|player| player.champion())?;
            let (my_pos, my_radius, me) = (me.pos(), me.radius() as u64, me.id());

            match kind {
                // The ult outranks the Chi Block plan: a dash onto the
                // healthiest enemy is worth more than one more stack.
                InputKindV1::Ult => {
                    Plan::At(highest_hp_enemy_champion(&sim, me, BALANCE_RANGE)?)
                }
                // Both only reach as far as she can hit, so an opening further
                // out is not one she can take with them.
                InputKindV1::Attack | InputKindV1::Skill => {
                    Plan::At(opening(&sim, me, ATTACK_RANGE)?)
                }
                // Lightfooted closes on an opening she cannot hit yet, and goes
                // to the nearest unit when there is none. Untargeted, so what
                // the engine needs is a point rather than a champion.
                _ => {
                    let unit = opening(&sim, me, LIGHTFOOTED_RANGE)
                        .or_else(|| nearest_unit(&sim, me, LIGHTFOOTED_RANGE))?;
                    let to = sim.get_entity(unit)?.pos();
                    // Step exactly as far as they are rather than the full
                    // range, so she stops on them instead of sailing past.
                    let gap = crate::util::isqrt(sim.distance_sq(me, unit));
                    Plan::Toward(crate::util::full_step_toward(
                        my_pos, to, gap, my_radius,
                    )?)
                }
            }
        };

        match plan {
            Plan::At(target) => {
                let aimed = aim(kind, &input, target)?;
                ctx.is_valid_input(&aimed).then_some(aimed)
            }
            // Point first, heading as the fallback - the same pair Wan's dash
            // offers the engine.
            Plan::Toward((dest, offset)) => {
                let mut aimed = InputV1::action(kind, InputTargetV1::pos(dest.0, dest.1));
                aimed.x = dest.0;
                aimed.y = dest.1;
                if ctx.is_valid_input(&aimed) {
                    return Some(aimed);
                }

                let headed = InputV1::action(kind, InputTargetV1::dir(offset.0, offset.1));
                ctx.is_valid_input(&headed).then_some(headed)
            }
        }
    }
}
