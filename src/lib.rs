use mod_api_stable::*;

mod avatar_wan;
mod match_hook;
mod ty_lee;
mod util;

const MOD_ID: &str = "avatar_wan_tfm2";

fn init(host: &StableHost) -> StableMod {
    host.log(
        LogLevel::Info,
        "avatar_wan_tfm2: registering Avatar Wan and Ty Lee effects",
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
    reg.add_native_effect(ty_lee::THREE_POINT_STRIKE, ty_lee::ThreePointStrike);
    reg.add_native_effect(ty_lee::THREE_POINT_STRIKE_HIT, ty_lee::ThreePointStrikeHit);
    reg.add_native_effect(ty_lee::CIRCUS_FREAK, ty_lee::CircusFreak);
    reg.add_native_effect(ty_lee::BALANCING_ACT, ty_lee::BalancingAct);

    reg.set_match_hook(match_hook::ModTick);

    // Keeps him in fights his kit is built to win. `matches` limits it to
    // Wan's own athletes, so no other champion's AI is touched.
    reg.add_player_input_ai(avatar_wan::player_ai::AggressiveWan);

    // Balancing Act is a `Targeting` ult so the engine will dash her; this puts
    // the "highest health" pick back in front of the cast.
    reg.add_player_input_ai(crate::ty_lee::player_ai::AimBalancingAct);

    reg
}

declare_stable_mod!(init);
