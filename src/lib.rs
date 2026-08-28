use mod_api_stable::*;

mod avatar_wan;
mod match_hook;
mod toph;
mod ty_lee;
mod util;

const MOD_ID: &str = "avatar_characters_tfm2";

fn init(host: &StableHost) -> StableMod {
    host.log(
        LogLevel::Info,
        "avatar_characters_tfm2: registering Avatar Wan, Ty Lee and Toph effects",
    );

    let mut reg = StableMod::new(MOD_ID);

    use avatar_wan::effects as wan;
    for (name, element) in wan::ATTACK_HITS {
        reg.add_native_effect(name, wan::SoulOfRaava(element));
    }
    reg.add_native_effect(wan::AVATAR_CYCLE, wan::TheAvatarCycle);
    reg.add_native_effect(wan::SPIRIT_STEP, wan::SpiritStep);
    reg.add_native_effect(wan::HARMONIC_CONVERGENCE, wan::HarmonicConvergence);
    reg.add_native_effect(wan::FIRE_BURN_TICK, wan::FireBurnTick);

    use ty_lee::effects as ty_lee;
    reg.add_native_effect(ty_lee::CHI_BLOCK_HIT, ty_lee::ChiBlocking);
    reg.add_native_effect(ty_lee::CHI_BLOCK_STUN, ty_lee::ChiBlockStun);
    reg.add_native_effect(ty_lee::THREE_POINT_STRIKE, ty_lee::ThreePointStrike);
    reg.add_native_effect(ty_lee::THREE_POINT_STRIKE_HIT, ty_lee::ThreePointStrikeHit);
    reg.add_native_effect(ty_lee::LIGHTFOOTED, ty_lee::Lightfooted);
    reg.add_native_effect(ty_lee::BALANCING_ACT, ty_lee::BalancingAct);
    reg.add_native_effect(ty_lee::BALANCING_ACT_LAND, ty_lee::BalancingActLand);

    use toph::effects as toph;
    reg.add_native_effect(toph::SEISMIC_SENSE, toph::SeismicSense);
    reg.add_native_effect(toph::ROCK_PILLAR, toph::RockPillar);
    reg.add_native_effect(toph::FIRST_METALBENDER, toph::FirstMetalbender);
    reg.add_native_effect(toph::BLIND_BANDIT, toph::BlindBandit);
    // Her ult schedules its follow-up shockwaves through `sim.queue_effect`,
    // which takes a registered id and nothing else - so the wave needs a name
    // of its own even though no action ever casts it.
    reg.add_native_effect(toph::BLIND_BANDIT_WAVE, toph::BlindBanditWave);

    reg.set_match_hook(match_hook::ModTick);

    // Keeps him in fights his kit is built to win. `matches` limits it to
    // Wan's own athletes, so no other champion's AI is touched.
    reg.add_player_input_ai(avatar_wan::player_ai::AggressiveWan);

    // Puts her own picks in front of the base AI's: the two dashes need a cast
    // target before the engine will move her, and her basic attack should stay
    // on an enemy one stack short of the Chi Block stun.
    reg.add_player_input_ai(crate::ty_lee::player_ai::AimTyLee::default());

    // Toph's own hook is off while the freeze during progression is being
    // chased - see `toph::player_ai`. Her kit is unaffected: the hook only
    // ever re-aimed casts the base AI had already decided to make, so without
    // it she plays as the base AI drives her.

    reg
}

declare_stable_mod!(init);
