//! Toph's animations, one per buff she can be seen wearing.
//!
//! This is the `view_buffs` block of the old `.data_champion` rewritten as
//! code. The buff names and z-orders are the ones it carried, so nothing about
//! what draws where has changed - only who decides it.
//!
//! Frame lengths are the sheet's own, in ticks rather than the seconds
//! `#anim.fanim` stores them in: 0.15s is 9 ticks, 0.1s is 6, and the pillar's
//! 0.08s rounds to 5. Multiplied out they are the `*_VFX_TICKS` constants in
//! `super`, which is what keeps a one-shot animation and the buff carrying it
//! the same length.

use crate::vfx::{strip, Binding};

use super::*;

pub fn bindings() -> Vec<Binding> {
    vec![
        // Over her target, so it reads on top of them.
        strip(
            MARK_BUFF,
            "asset/avatar_characters_tfm2/effects/toph/seismic_mark#sheet",
            1,
            4,
            9,
        ),
        strip(
            MARK_POP_VFX_BUFF,
            "asset/avatar_characters_tfm2/effects/toph/seismic_mark_pop#sheet",
            1,
            4,
            6,
        ),
        // Under her: the suit is worn, so she stands in front of it.
        strip(
            METAL_BUFF,
            "asset/avatar_characters_tfm2/effects/toph/metal_armor#sheet",
            -1,
            4,
            9,
        ),
        strip(
            METAL_BURST_VFX_BUFF,
            "asset/avatar_characters_tfm2/effects/toph/metal_burst#sheet",
            1,
            5,
            6,
        ),
        strip(
            BANDIT_WAVE_VFX_BUFF,
            "asset/avatar_characters_tfm2/effects/toph/shockwave#sheet",
            -1,
            5,
            6,
        ),
        strip(
            BANDIT_SLOW_BUFF,
            "asset/avatar_characters_tfm2/effects/toph/slow#sheet",
            -1,
            4,
            9,
        ),
        // The pillar coming up under whoever the skill was aimed at. It was a
        // `view_effect` rather than a `view_buff` before the port - a detached
        // animation played at their feet - and is a short buff now, which is
        // the only channel a Rust champion has. The visible difference is that
        // it rides the target rather than staying put, and that a killing blow
        // cuts it short instead of letting it finish over the corpse.
        strip(
            PILLAR_VFX_BUFF,
            "asset/avatar_characters_tfm2/effects/toph/rock_pillar#sheet",
            -1,
            6,
            5,
        ),
    ]
}
