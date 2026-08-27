use mod_api_stable::*;

/// The mod gets one match hook, so it fans out to each champion's own per-tick
/// upkeep rather than holding any champion logic itself.
pub struct ModTick;

impl StableMatchHook for ModTick {
    fn on_match_tick(&self, sim: &mut StableSim<'_>, _rng_seed: u64) {
        crate::avatar_wan::tick::on_match_tick(sim);
        crate::ty_lee::tick::on_match_tick(sim);
        crate::toph::tick::on_match_tick(sim);
    }
}
