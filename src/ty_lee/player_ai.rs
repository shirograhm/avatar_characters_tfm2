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
//! On top of that sits her combo. Left alone the base AI spends her kit as
//! separate reactions; she wants to spend it as one trade - open with
//! Lightfooted so the shield and the dodge are already up, then land the Chi
//! Block stacks behind it. So once the base AI has decided to fight, this
//! drives the sequence itself: Lightfooted, ult, Three Point Strike, basic
//! attack.
//!
//! Both of her combos are that one list. Every step is checked against the
//! engine's remaining-cooldown counters when its turn comes and skipped if she
//! does not have it, so an ult on cooldown drops straight out and leaves
//! S2 -> S1 -> AA, while an ult that is up gives S2 -> Ult -> S1 -> AA. Nothing
//! branches.
//!
//! That cooldown check is load-bearing, and it is `player.cooldowns()` rather
//! than `is_valid_input` - the latter does not gate cooldowns and will accept
//! an ult she does not have. Naming an action she cannot cast locks the match
//! up, which is a hazard the base AI never faces, because it only ever picks
//! from what it already holds.
//!
//! The combo is for champions only. Its opener needs an enemy champion inside
//! dash range before it will fire, and every step aims at a champion; when
//! there is none, no combo is built and a wave is left to the base AI, which
//! is what should be clearing it.
//!
//! Only the *opener* is allowed to start a combo. If Lightfooted is down she
//! does not begin one halfway through - going in is the point, and the rest of
//! the kit without the shield is just her normal fighting, which is what the
//! re-aiming below already handles.
//!
//! There is no way to ask the engine whether she is already mid-cast, so a
//! hook that answered "cast Three Point Strike" would keep answering it on
//! every tick of that cast, re-issuing the input into an action already
//! running. That is what `busy_until` is for: after issuing a step she goes
//! quiet for that action's own `duration` and lets the cast play out.
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

use super::effects::{
    highest_hp_enemy_champion, nearest_enemy_champion, nearest_unit, stacked_enemies,
};
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

/// Where one action should point, read fresh off the board. `None` means there
/// is nothing it can be aimed at - which is also how a combo step declines its
/// turn and hands it to the next one.
fn plan_for(
    sim: &StableSim<'_>,
    me: usize,
    my_pos: (u64, u64),
    my_radius: u64,
    kind: InputKindV1,
    combo: bool,
) -> Option<Plan> {
    match kind {
        // The ult outranks the Chi Block plan: a dash onto the healthiest enemy
        // is worth more than one more stack. Champions only, so a combo run at
        // a minion wave drops this step instead of spending the ult on it.
        InputKindV1::Ult => Some(Plan::At(highest_hp_enemy_champion(sim, me, BALANCE_RANGE)?)),
        // Both only reach as far as she can hit, so an opening further out is
        // not one she can take with them.
        InputKindV1::Attack | InputKindV1::Skill => Some(Plan::At(
            opening(sim, me, ATTACK_RANGE)
                // A combo hit lands on a champion even before one carries a
                // stack. Outside a combo this stays as it was and an unstacked
                // target is left to the base AI's own pick.
                .or_else(|| combo.then(|| nearest_enemy_champion(sim, me, ATTACK_RANGE))?)?,
        )),
        // Lightfooted closes on an opening she cannot hit yet, and goes to the
        // nearest unit when there is none. Untargeted, so what the engine needs
        // is a point rather than a champion.
        _ => {
            // Champions only when this is the combo opening: going in on a
            // wave is not a trade. The re-aiming path keeps the old behaviour
            // and may still send a dash at whatever unit is nearest.
            let unit = opening(sim, me, LIGHTFOOTED_RANGE)
                .or_else(|| nearest_enemy_champion(sim, me, LIGHTFOOTED_RANGE))
                .or_else(|| (!combo).then(|| nearest_unit(sim, me, LIGHTFOOTED_RANGE))?)?;
            let to = sim.get_entity(unit)?.pos();
            // Step exactly as far as they are rather than the full range, so
            // she stops on them instead of sailing past.
            let gap = crate::util::isqrt(sim.distance_sq(me, unit));
            Some(Plan::Toward(crate::util::full_step_toward(
                my_pos, to, gap, my_radius,
            )?))
        }
    }
}

/// The input the engine will actually accept for a plan, or `None` when it will
/// accept none of them - a target that has moved out of range, or a heading it
/// will not take. This is a shape check only; whether she *has* the action is
/// `ready`'s job, and both have to pass before a step goes out.
fn issue(ctx: &mut StableAiContext<'_>, kind: InputKindV1, plan: &Plan) -> Option<InputV1> {
    match plan {
        Plan::At(target) => {
            let aimed = InputV1::action(kind, InputTargetV1::target(*target));
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

/// Whether a step is actually off cooldown, from the engine's own remaining-tick
/// counters.
///
/// This is the check, not `is_valid_input`. That one does not gate cooldowns:
/// it will happily accept an ult she does not have, and issuing a cast that
/// cannot start is what locked a match up. The base AI never hit it because it
/// only ever picks actions it already holds - the hazard is new the moment a
/// hook starts naming actions of its own.
fn ready(cooldowns: (usize, usize, usize, usize), kind: InputKindV1) -> bool {
    let (attack, skill, skill2, ult) = cooldowns;
    match kind {
        InputKindV1::Skill2 => skill2 == 0,
        InputKindV1::Ult => ult == 0,
        InputKindV1::Skill => skill == 0,
        _ => attack == 0,
    }
}

/// The combo, in order. Skipping is what makes this one list cover both of the
/// sequences she is meant to trade with - see the module note.
const COMBO: [InputKindV1; 4] = [
    InputKindV1::Skill2,
    InputKindV1::Ult,
    InputKindV1::Skill,
    InputKindV1::Attack,
];

/// How long the action behind each step keeps her busy.
fn action_ticks(kind: InputKindV1) -> usize {
    match kind {
        InputKindV1::Skill2 => LIGHTFOOTED_ACTION_TICKS,
        InputKindV1::Ult => BALANCE_ACTION_TICKS,
        InputKindV1::Skill => STRIKE_ACTION_TICKS,
        _ => ATTACK_ACTION_TICKS,
    }
}

pub struct AimTyLee {
    /// How far into `COMBO` she has got. `COMBO.len()` - the default - means no
    /// combo is running and she is fighting on the base AI's own picks.
    step: usize,
    /// Tick the action she last issued stops playing on. Nothing new goes out
    /// before it.
    busy_until: usize,
}

impl Default for AimTyLee {
    fn default() -> Self {
        // Past the end of `COMBO`, not at the start of it: a fresh instance is
        // not mid-combo, and `derive` would have had her opening on tick one.
        Self {
            step: COMBO.len(),
            busy_until: 0,
        }
    }
}

impl AimTyLee {
    fn idle(&self) -> bool {
        self.step >= COMBO.len()
    }

    /// Remembers what she just issued and goes quiet for its cast.
    fn issued(&mut self, step: usize, kind: InputKindV1, tick: usize) {
        self.step = step + 1;
        self.busy_until = tick + action_ticks(kind);
    }

    fn abandon(&mut self) {
        self.step = COMBO.len();
    }
}

impl StablePlayerAi for AimTyLee {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(AimTyLee::default())
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

        // Still in the last step's cast. Saying nothing leaves the engine to
        // play it out rather than issuing into an action already running.
        let tick = ctx.tick();
        if tick < self.busy_until {
            return None;
        }

        let player = ctx.player_id();

        // Everything the board decides, gathered while `sim` is alive so that
        // `ctx` is free for `is_valid_input` afterwards.
        let (steps, fallback) = {
            let sim = ctx.sim()?;
            let player = sim.get_player(player)?;
            // No cooldown counters, no combo. Naming an action she may not have
            // is the one thing this must never do, so without the readings it
            // falls back to re-aiming, which can only ever repoint a cast the
            // base AI already decided it could make.
            let cooldowns = player.cooldowns();
            let me = player.champion()?;

            // Nothing is named while she is dead or held. A cast she cannot
            // start is the failure this must avoid, and being under crowd
            // control is the other way to be unable to start one - cooldowns
            // only cover whether she *has* the action. Any CC at all counts:
            // sorting out which kinds block which actions would be guessing,
            // and the base AI already handles being locked down.
            if !me.is_alive() || me.cc_count() > 0 {
                return None;
            }

            let (my_pos, my_radius, me) = (me.pos(), me.radius() as u64, me.id());

            // Opening is gated on an enemy *champion* in dash range: this is
            // her skirmish pattern, and minions are the base AI's business.
            let skirmishing = nearest_enemy_champion(&sim, me, LIGHTFOOTED_RANGE).is_some();

            // Idle, only the opener may start one. Mid-combo, everything left.
            let remaining = match (self.idle(), skirmishing, cooldowns.is_some()) {
                (_, _, false) => 0..0,
                (true, true, _) => 0..1,
                (true, false, _) => 0..0,
                (false, _, _) => self.step..COMBO.len(),
            };

            let steps: Vec<(usize, InputKindV1, Plan)> = remaining
                .filter(|&index| cooldowns.is_some_and(|left| ready(left, COMBO[index])))
                .filter_map(|index| {
                    let step = COMBO[index];
                    Some((index, step, plan_for(&sim, me, my_pos, my_radius, step, true)?))
                })
                .collect();

            (steps, plan_for(&sim, me, my_pos, my_radius, kind, false))
        };

        for (index, step, plan) in &steps {
            if let Some(aimed) = issue(ctx, *step, plan) {
                self.issued(*index, *step, tick);
                return Some(aimed);
            }
        }

        // Either she never opened or the rest of the combo is gone. Fall back
        // to what this hook has always done: re-aim the base AI's own pick.
        self.abandon();

        match fallback? {
            Plan::At(target) => {
                let aimed = aim(kind, &input, target)?;
                ctx.is_valid_input(&aimed).then_some(aimed)
            }
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
