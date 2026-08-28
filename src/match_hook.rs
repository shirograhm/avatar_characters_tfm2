use mod_api_stable::*;

/// The mod gets one match hook, so it fans out to each champion's own per-tick
/// upkeep rather than holding any champion logic itself.
pub struct ModTick;

impl StableMatchHook for ModTick {
    /// Entity ids are only unique inside one match, so anything keyed by them
    /// has to go before the next one starts.
    fn on_match_start(&self, _sim: &mut StableSim<'_>) {
        crate::vfx::clear();
    }

    fn on_match_tick(&self, sim: &mut StableSim<'_>, _rng_seed: u64) {
        crate::avatar_wan::tick::on_match_tick(sim);
        crate::ty_lee::tick::on_match_tick(sim);
        crate::toph::tick::on_match_tick(sim);

        // Last, so the frame it hands the renderer is the one the upkeep above
        // just settled - a mark cleared off a corpse this tick should not get
        // one more frame of being drawn.
        crate::vfx::refresh(sim);
    }
}
