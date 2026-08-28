//! Client-side VFX for the champions this mod defines in Rust.
//!
//! A champion registered through `add_champion` has no `view_buffs` block to
//! hang its animations on - `StableChampion` carries an id, stats, tags,
//! icons and four actions, and nothing else - so the drawing the data layer
//! used to do for free is done here instead.
//!
//! The split into two halves is forced by the API rather than chosen. Buffs
//! only exist inside the simulation, `draw_sprite` only works inside a render
//! hook, and the two never share a call stack. So the match hook photographs
//! every buff that has a binding once a tick, and the extension draws that
//! photograph on the frames that follow. A snapshot is at most one tick stale,
//! which at 60Hz is not a thing anyone can see.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use mod_api_stable::*;

/// World units per drawing unit on the "Game" map. Entity positions come back
/// in the same thousandths the ranges in `toph::mod` are written in.
const WORLD_UNITS: f32 = 1000.0;

/// One cell of an animation, already normalised into its sheet.
#[derive(Clone, Copy)]
pub struct Frame {
    pub ticks: usize,
    pub uv: (f32, f32, f32, f32),
}

/// An animation tied to a buff name: while an entity carries the buff, the
/// binding is drawn on it. This is the same contract the `view_buffs` block
/// had, which is why the buff names did not have to change.
pub struct Binding {
    pub buff: &'static str,
    pub texture: &'static str,
    pub z: i32,
    pub frames: Vec<Frame>,
}

impl Binding {
    /// The cell showing at `elapsed` ticks, looping. Every one of this mod's
    /// one-shot animations is worn as a buff timed to its own length, so the
    /// loop only ever comes round for the ones that are meant to.
    fn frame_at(&self, elapsed: usize) -> Option<Frame> {
        let total: usize = self.frames.iter().map(|frame| frame.ticks.max(1)).sum();
        if total == 0 {
            return None;
        }
        let mut cursor = elapsed % total;
        for frame in &self.frames {
            let ticks = frame.ticks.max(1);
            if cursor < ticks {
                return Some(*frame);
            }
            cursor -= ticks;
        }
        self.frames.last().copied()
    }
}

/// A binding for a sheet laid out as one left-to-right strip of equal cells,
/// which is the only layout `tools/gen_toph_effects.py` writes.
///
/// `uv` is the normalised source rect inside the texture, written (x, y, w, h)
/// - the reading `StableSpriteParams` documents, where (0, 0, 1, 1) is the
/// whole image. If the engine turns out to want two corners instead, the third
/// component becomes `(index + 1) / cells` and this is the only place it
/// changes.
pub fn strip(
    buff: &'static str,
    texture: &'static str,
    z: i32,
    cells: usize,
    ticks: usize,
) -> Binding {
    let width = 1.0 / cells as f32;
    Binding {
        buff,
        texture,
        z,
        frames: (0..cells)
            .map(|index| Frame {
                ticks,
                uv: (index as f32 * width, 0.0, width, 1.0),
            })
            .collect(),
    }
}

fn bindings() -> &'static [Binding] {
    static BINDINGS: OnceLock<Vec<Binding>> = OnceLock::new();
    BINDINGS.get_or_init(crate::toph::vfx::bindings)
}

fn binding_for(buff: &str) -> Option<&'static Binding> {
    bindings().iter().find(|binding| binding.buff == buff)
}

/// One sprite the extension should draw. Resolved on the simulation side so
/// the render hook stays a loop over ready-made draw calls - it has no way to
/// ask what tick it is, and no business locking the frame tables.
#[derive(Clone, Copy)]
struct Sprite {
    texture: &'static str,
    z: i32,
    uv: (f32, f32, f32, f32),
    x: f32,
    y: f32,
}

/// When each (entity, binding) pair first appeared, which is what turns a buff
/// that just says "on" into a frame number.
fn starts() -> &'static Mutex<HashMap<(usize, &'static str), usize>> {
    static STARTS: OnceLock<Mutex<HashMap<(usize, &'static str), usize>>> = OnceLock::new();
    STARTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn sprites() -> &'static Mutex<Vec<Sprite>> {
    static SPRITES: OnceLock<Mutex<Vec<Sprite>>> = OnceLock::new();
    SPRITES.get_or_init(|| Mutex::new(Vec::new()))
}

/// Retakes the photograph. Called once a tick from the match hook.
pub fn refresh(sim: &StableSim<'_>) {
    let now = sim.tick();
    let mut starts = starts().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut live = HashSet::new();
    let mut frame = Vec::new();

    for index in 0..sim.entity_count() {
        let Some(entity) = sim.entity_at(index) else {
            continue;
        };
        // A corpse shows nothing. Buffs outlive the unit wearing them, so
        // without this the last hit of a fight draws over its own kill - and
        // zero health counts as dead on its own here, the same way it does in
        // `toph::tick`, because `is_alive` can still read true while the death
        // animation runs.
        if !entity.is_alive() || entity.hp().0 == 0 {
            continue;
        }
        let id = entity.id();
        let (x, y) = entity.pos();

        for buff_index in 0..entity.buff_count() {
            let Some(buff) = entity.buff_at(buff_index) else {
                continue;
            };
            let Some(binding) = binding_for(buff.name()) else {
                continue;
            };
            // Stacks share one animation: the mod counts stacks by stacking
            // buffs of the same name, and four copies of a sprite in the same
            // place is just a brighter sprite.
            if !live.insert((id, binding.buff)) {
                continue;
            }
            let started = *starts.entry((id, binding.buff)).or_insert(now);
            let Some(cell) = binding.frame_at(now.saturating_sub(started)) else {
                continue;
            };
            frame.push(Sprite {
                texture: binding.texture,
                z: binding.z,
                uv: cell.uv,
                x: x as f32 / WORLD_UNITS,
                y: y as f32 / WORLD_UNITS,
            });
        }
    }

    // Anything that fell off this tick starts from frame zero if it comes
    // back, which is what re-applying a mark should look like.
    starts.retain(|key, _| live.contains(key));
    drop(starts);

    *sprites().lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = frame;
}

/// Drops everything at the start of a match. Entity ids are only unique within
/// one, so a leftover start tick would put a new match's first mark partway
/// through its animation.
pub fn clear() {
    starts().lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clear();
    sprites().lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clear();
}

pub struct Extension;

impl StableExtension for Extension {
    fn post_render(&self, client: &mut StableClient<'_>) {
        if !client.is_in_game() || !client.can_draw() {
            return;
        }
        let frame = sprites()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();

        for sprite in frame {
            let params = StableSpriteParams {
                x: sprite.x,
                y: sprite.y,
                z: sprite.z,
                // The sheets are drawn around whatever they sit on, so the
                // cell's middle is the point that goes on the entity.
                pivot_x: 0.5,
                pivot_y: 0.5,
                uv: sprite.uv,
                sample_nearest: true,
                ..StableSpriteParams::default()
            };
            let _ = client.draw_sprite("Game", sprite.texture, &params);
        }
    }
}
