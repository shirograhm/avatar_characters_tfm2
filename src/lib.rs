use mod_api_stable::*;

mod avatar_wan;
mod match_hook;
mod ty_lee;
mod util;
mod vfx_refresh;

const MOD_ID: &str = "avatar_characters_tfm2";

fn init(host: &StableHost) -> StableMod {
    host.log(
        LogLevel::Info,
        "avatar_characters_tfm2: registering Avatar Wan and Ty Lee effects",
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

    reg.set_match_hook(match_hook::ModTick);

    // Keeps him in fights his kit is built to win. `matches` limits it to
    // Wan's own athletes, so no other champion's AI is touched.
    reg.add_player_input_ai(avatar_wan::player_ai::AggressiveWan);

    // Puts her own picks in front of the base AI's: the two dashes need a cast
    // target before the engine will move her, and her basic attack should stay
    // on an enemy one stack short of the Chi Block stun.
    reg.add_player_input_ai(crate::ty_lee::player_ai::AimTyLee::default());

    reg
}

declare_stable_mod!(init);
