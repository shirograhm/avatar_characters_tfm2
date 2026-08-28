//! Re-seats the mod's view buffs so their animations survive a seek backwards.
//!
//! Every animation this mod draws is a `view_buffs` entry in a
//! `.data_champion`: the view puts an animation on a unit because a named buff
//! appeared on it. No base champion uses that table - all ten of the game's own
//! are empty - so seeking back through a match is a path only mods take, and
//! the view does not come out of it drawing again. The buffs are still there in
//! the restored state, so they never *appear* a second time, nothing re-creates
//! the animation, and the unit goes bare for the rest of the match.
//!
//! Taking a buff off and putting it straight back inside one tick does not
//! rouse it - that was tried and the animations stayed gone - so the view is
//! not watching for the buff to be added; it is comparing what a unit holds
//! against what it held before, and a buff present on both sides of the
//! comparison reads as no change at all. Which means the only thing that will
//! register is a buff that is genuinely *missing* for a whole tick and back the
//! next.
//!
//! ## What this can blink, and why it is only one thing so far
//!
//! A gap is free when something else already knows how to rebuild the buff from
//! sim state, because then the sim is its own memory and the restore needs no
//! recollection of what was taken off. `ty_lee::tick::recount_chi_block`
//! rebuilds the stack marks from the Chi Block count every tick, so dropping
//! one is a one-tick hole it closes on its own.
//!
//! Nothing else here has that. The four buffs that actually stay dark after a
//! seek - `wan_element_*`, `wan_spirit_step`, `wan_harmonic_convergence` and
//! `ty_lee_lightfooted` - all carry gameplay as well as a picture, and a gap in
//! them is a gap in the ability: `window_open` reads `wan_spirit_step` to decide
//! whether Spirit Step is still storing, and `ty_lee_lightfooted` carries the
//! `base_attack_damaged_reduce` that *is* her dodge. Restoring one means putting
//! its stats and remaining ticks back, which this cannot remember across a tick
//! without holding state the sim would not rewind - and mod state that survives
//! a rewind desyncs a replay from the match it is replaying, which is a far
//! worse bug than a missing aura. Splitting each of them into a gameplay buff
//! and a statless mirror that a reconciler rebuilds is what makes them blinkable
//! too, and that is the next step if this one lands.
//!
//! The short timed ones - the splashes, the burn, the heal flash, the Chi Block
//! break burst - need nothing. They are reapplied constantly, and a fresh buff
//! on a unit that had none is an appearance under any reading, so they already
//! come back on their own after a seek.
//!
//! ## Cadence
//!
//! `REFRESH_INTERVAL` is how long a unit can draw nothing after a seek, and how
//! often a blink costs a frame of animation. Units are spread across it by id so
//! a blink is never a team-wide flicker, and the schedule is a function of sim
//! tick and entity id alone, so it replays identically.

use mod_api_stable::*;

use crate::ty_lee;

/// Ticks between blinks for any one entity, and so the worst case for how long
/// a unit draws nothing after a seek.
pub const REFRESH_INTERVAL: usize = 120;

/// View buffs that something else rebuilds from sim state, and which are
/// therefore safe to drop for a tick. Adding a name here is only sound if a
/// reconciler puts it back - see the module docs.
fn blinkable_names() -> [&'static str; 3] {
    ty_lee::CHI_BLOCK_VFX_BUFFS
}

/// Each entity comes up once per `REFRESH_INTERVAL`, on a tick of its own.
/// Keyed off the entity id rather than a counter so it stays a function of sim
/// state and nothing else.
fn due(tick: usize, entity: usize) -> bool {
    tick % REFRESH_INTERVAL == entity % REFRESH_INTERVAL
}

pub fn on_match_tick(sim: &mut StableSim<'_>) {
    let names = blinkable_names();
    let tick = sim.tick();

    let blink: Vec<(usize, &'static str)> = (0..sim.entity_count())
        .filter_map(|index| sim.entity_at(index))
        .filter(|entity| due(tick, entity.id()))
        .flat_map(|entity| {
            let held: Vec<&'static str> = names
                .into_iter()
                .filter(|name| {
                    (0..entity.buff_count())
                        .filter_map(|buff| entity.buff_at(buff))
                        .any(|buff| buff.name() == *name)
                })
                .collect();
            let id = entity.id();
            held.into_iter().map(move |name| (id, name))
        })
        .collect();

    // Left off, not put back: this runs after each champion's upkeep, so the
    // hole stands until the next tick's reconciler closes it, which is the
    // whole point - a buff that is missing at a tick boundary is the only thing
    // the view reads as a change.
    for (entity, name) in blink {
        sim.entity_remove_buff(entity, name);
    }
}
