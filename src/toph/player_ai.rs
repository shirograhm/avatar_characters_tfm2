//! Toph steers by her own mark, and refuses to slam an empty field.
//!
//! This is deliberately a much smaller hook than Ty Lee's. She has no combo to
//! sequence - her kit is three independent buttons - so all it does is fix the
//! three things the base AI cannot know about:
//!
//! - **Cash the mark.** Seismic Sense is worth the better part of a hundred
//!   extra magic damage on a marked champion, and the base AI picks on damage
//!   and threat, which knows nothing about who is carrying one. So her fist and
//!   her pillar go to the closest marked champion in reach when there is one.
//!
//! - **Hold the ult.** Blind Bandit is a self-cast that hits whatever happens
//!   to be standing around her, so the base AI can spend sixty seconds of
//!   cooldown on empty ground. If the slam would catch no enemy champion at
//!   all, this puts her fist in instead - and only when the engine will take
//!   that, otherwise the base AI's own pick stands.
//!
//! - **Recast the armor.** The First Metalbender's second half is a recast the
//!   base AI has no concept of; left alone it always goes off on its timer,
//!   wherever she happens to be. Once she is past the halfway point of the
//!   window - so the suit has already been worth wearing - a champion inside
//!   the burst is enough to set it off deliberately.
//!
//! Everything else falls through untouched. `Move` and `Return` especially:
//! hijacking those would fight the base AI's positioning and pull her out of
//! its retreats.
//!
//! Naming an action she does not hold is the one thing this must never do -
//! a cast that cannot start locks the match up, which is a hazard the base AI
//! never faces because it only ever picks from what it already has. So every
//! pick this makes of its own is gated on `player.cooldowns()` *and* on her
//! level: an unlearned slot reports zero ticks remaining, exactly like one that
//! is off cooldown. `is_valid_input` is not that gate - it does not look at
//! cooldowns at all - it is only the shape check that runs afterwards.

use mod_api_stable::*;

use super::effects::{marked_enemies, slam_champions};
use super::*;
use crate::util::nearest_enemy_champion;

/// `None` when the input already says exactly this, so an unchanged pick is
/// left as the base AI's rather than replaced with an identical one.
fn aim(kind: InputKindV1, input: &InputV1, target: usize) -> Option<InputV1> {
    let unchanged = InputKindV1::from_code(input.kind) == Some(kind)
        && InputTargetKindV1::from_code(input.target.kind) == Some(InputTargetKindV1::Target)
        && input.target.target_id == target;

    (!unchanged).then(|| InputV1::action(kind, InputTargetV1::target(target)))
}

/// The closest marked enemy champion within `range`.
fn best_mark(sim: &StableSim<'_>, me: usize, range: u64) -> Option<usize> {
    marked_enemies(sim, me, range)
        // Closest first; ties resolve by champion order, which is
        // deterministic.
        .into_iter()
        .min()
        .map(|(_, id)| id)
}

/// Whether a step is one she actually holds: unlocked at her level, and off
/// cooldown by the engine's own remaining-tick counters. Both halves are
/// needed - see the module note.
fn ready(cooldowns: (usize, usize, usize, usize), level: usize, kind: InputKindV1) -> bool {
    let (attack, skill, skill2, ult) = cooldowns;
    match kind {
        InputKindV1::Skill2 => skill2 == 0 && level >= crate::util::SKILL2_LEVEL,
        InputKindV1::Ult => ult == 0 && level >= crate::util::ULT_LEVEL,
        InputKindV1::Skill => skill == 0 && level >= crate::util::SKILL_LEVEL,
        _ => attack == 0,
    }
}

/// How far into the armor's window she is, or `None` when she is not wearing
/// it. Read off the buff's own remaining ticks rather than tracked here, so a
/// suit refreshed or stripped by anything else stays honest.
fn armor_remaining(entity: &StableEntity<'_, '_>) -> Option<usize> {
    (0..entity.buff_count())
        .filter_map(|index| entity.buff_at(index))
        .find(|buff| buff.name() == METAL_BUFF)
        .map(|buff| buff.duration_tick)
}

/// What this hook decided to do, before `ctx` is free to validate it.
enum Pick {
    /// Point `kind` at this unit.
    At(InputKindV1, usize),
    /// Leave the base AI's own pick alone.
    Keep,
}

#[derive(Default)]
pub struct AimToph {
    /// Tick the recast she last issued stops playing on. Nothing new goes out
    /// before it - there is no way to ask the engine whether a cast is still
    /// running, and re-issuing into one already running is what `busy_until`
    /// exists to prevent.
    busy_until: usize,
}

impl StablePlayerAi for AimToph {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(AimToph::default())
    }

    fn id(&self) -> String {
        "toph_aim".to_string()
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

        let tick = ctx.tick();
        let player = ctx.player_id();

        // Everything the board decides, gathered while `sim` is alive so that
        // `ctx` is free for `is_valid_input` afterwards. Both come back rather
        // than one: the recast is the only pick that can be refused on shape
        // alone, and a refusal should fall through to the re-aim rather than
        // cost her the tick.
        let (detonate, pick) = {
            let sim = ctx.sim()?;
            let player = sim.get_player(player)?;
            let cooldowns = player.cooldowns();
            let level = player.level();
            let me = player.champion()?;

            // Nothing is named while she is dead or held. Being under crowd
            // control is the other way to be unable to start a cast, and
            // cooldowns only cover whether she has the action at all.
            if !me.is_alive() || me.cc_count() > 0 {
                return None;
            }

            let held = |kind| cooldowns.is_some_and(|left| ready(left, level, kind));
            let remaining = armor_remaining(&me);
            let me = me.id();

            // The recast outranks re-aiming: it is the only pick here that can
            // go out on a tick the base AI was not already spending.
            let burst_worth_it = remaining.is_some_and(|left| left <= METAL_DURATION / 2)
                && nearest_enemy_champion(&sim, me, METAL_BURST_RADIUS).is_some();

            let detonate =
                tick >= self.busy_until && burst_worth_it && held(InputKindV1::Skill2);

            let pick = match kind {
                // Both go to whoever is already carrying a mark. Without one in
                // reach the base AI's pick stands - an unmarked target is no
                // more interesting to her than any other.
                InputKindV1::Attack => best_mark(&sim, me, ATTACK_RANGE)
                    .map_or(Pick::Keep, |target| Pick::At(kind, target)),
                InputKindV1::Skill => best_mark(&sim, me, COLUMN_RANGE)
                    .map_or(Pick::Keep, |target| Pick::At(kind, target)),
                // A slam that catches no champion is sixty seconds of cooldown
                // spent on empty ground. Her fist is the substitute, and only
                // if she has one to throw.
                InputKindV1::Ult if slam_champions(&sim, me) == 0 => {
                    match nearest_enemy_champion(&sim, me, ATTACK_RANGE) {
                        Some(target) if held(InputKindV1::Attack) => {
                            Pick::At(InputKindV1::Attack, target)
                        }
                        _ => Pick::Keep,
                    }
                }
                _ => Pick::Keep,
            };

            (detonate, pick)
        };

        if detonate {
            // Self-cast, so the engine wants no target at all.
            let aimed = InputV1::action(InputKindV1::Skill2, InputTargetV1::NONE);
            if ctx.is_valid_input(&aimed) {
                self.busy_until = tick + METAL_ACTION_TICKS;
                return Some(aimed);
            }
        }

        match pick {
            Pick::At(kind, target) => {
                let aimed = aim(kind, &input, target)?;
                ctx.is_valid_input(&aimed).then_some(aimed)
            }
            Pick::Keep => None,
        }
    }
}
