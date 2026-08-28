//! Toph as the engine sees her: the `.data_champion` she used to be, written
//! as code.
//!
//! `StableChampion` and `StableAction` cover everything the JSON's top level
//! did - stats, growth, tags, icons and the four action shapes - and each
//! action hands back the effect struct from `super::effects` directly instead
//! of naming it through a registry. What the traits have no room for is the
//! asset half: the sprite is resolved by convention from her id (see
//! `mod.override_info`) and the animations are drawn by `crate::vfx`.

use mod_api_stable::*;

use super::effects::{BlindBandit, FirstMetalbender, RockPillar, SeismicSense};
use super::*;

/// Her skill text, which stays in `text/champion.i18n` exactly as it was - the
/// data file referenced these same four keys.
fn description(key: &str) -> String {
    format!("#asset/base/text/champion?description.{CHAMPION_KEY}.{key}")
}

pub struct Toph;

impl StableChampion for Toph {
    fn id(&self) -> String {
        CHAMPION_KEY.to_string()
    }

    /// Left at the default, which is the id. The data file named no display
    /// name either: the engine reads it out of `description.toph.name` in the
    /// i18n, and that entry has not moved.
    fn skill_icon(&self, skill_index: usize) -> (String, String) {
        let icon = match skill_index {
            0 => "base_attack",
            1 => "skill",
            2 => "skill2",
            _ => "ult",
        };
        (
            format!("asset/avatar_characters_tfm2/icons/{CHAMPION_KEY}/{icon}"),
            String::new(),
        )
    }

    fn category(&self) -> ChampionCategoryV1 {
        ChampionCategoryV1::Melee
    }

    fn tags(&self) -> Vec<ChampionTagV1> {
        vec![
            ChampionTagV1::Ap,
            ChampionTagV1::Magic,
            ChampionTagV1::Melee,
            ChampionTagV1::Tank,
            ChampionTagV1::Cc,
        ]
    }

    fn stat(&self) -> StatV1 {
        BASE_STAT
    }

    fn growth(&self) -> StatV1 {
        GROWTH_STAT
    }

    fn attack(&self) -> Box<dyn StableAction> {
        Box::new(SeismicSenseAction)
    }

    fn skill(&self) -> Box<dyn StableAction> {
        Box::new(RockPillarAction)
    }

    fn skill2(&self) -> Box<dyn StableAction> {
        Box::new(FirstMetalbenderAction)
    }

    fn ult(&self) -> Option<Box<dyn StableAction>> {
        Some(Box::new(BlindBanditAction))
    }
}

/// Seismic Sense. A plain targeted swing; everything that makes it hers
/// happens inside the effect.
struct SeismicSenseAction;

impl StableAction for SeismicSenseAction {
    fn clone_box(&self) -> Box<dyn StableAction> {
        Box::new(Self)
    }

    fn action_name(&self) -> String {
        "attack".to_string()
    }

    fn duration(&self) -> usize {
        ATTACK_ACTION_TICKS
    }

    fn cooltime(&self, _caster_stat: &StatV1, _caster_level: usize) -> usize {
        ATTACK_COOLTIME
    }

    fn casting_target(&self) -> CastingTargetV1 {
        CastingTargetV1::Enemy
    }

    fn description(&self) -> String {
        description("attack")
    }

    fn effect(&self) -> Option<StableEffectSpec> {
        Some(StableEffectSpec {
            range: ATTACK_RANGE,
            growth_range: 0,
            start_timing: ATTACK_START_TIMING,
            casting: CastingTypeV1::Targeting,
            target: CastingTargetV1::Enemy,
            attack_type: AttackTypeV1::BaseAttack,
            effect: Box::new(SeismicSense),
        })
    }
}

/// Rock Pillar. The data file paired the native effect with a `ViewEffect` in
/// a `Combine`; the effect now raises its own animation, so the action is a
/// single effect again - see `RockPillar::apply`.
struct RockPillarAction;

impl StableAction for RockPillarAction {
    fn clone_box(&self) -> Box<dyn StableAction> {
        Box::new(Self)
    }

    fn action_name(&self) -> String {
        "skill1".to_string()
    }

    fn duration(&self) -> usize {
        PILLAR_ACTION_TICKS
    }

    fn cooltime(&self, _caster_stat: &StatV1, _caster_level: usize) -> usize {
        PILLAR_COOLTIME
    }

    fn casting_target(&self) -> CastingTargetV1 {
        CastingTargetV1::Enemy
    }

    fn description(&self) -> String {
        description("skill")
    }

    fn effect(&self) -> Option<StableEffectSpec> {
        Some(StableEffectSpec {
            range: PILLAR_RANGE,
            growth_range: 0,
            start_timing: PILLAR_START_TIMING,
            casting: CastingTypeV1::Targeting,
            target: CastingTargetV1::Enemy,
            attack_type: AttackTypeV1::Skill,
            effect: Box::new(RockPillar),
        })
    }
}

/// The First Metalbender. The two charges are the recast - see
/// `METAL_ARMED_BUFF` for what each one spends - and she can keep walking
/// while she bends the suit on.
struct FirstMetalbenderAction;

impl StableAction for FirstMetalbenderAction {
    fn clone_box(&self) -> Box<dyn StableAction> {
        Box::new(Self)
    }

    fn action_name(&self) -> String {
        "skill2".to_string()
    }

    fn duration(&self) -> usize {
        METAL_ACTION_TICKS
    }

    fn cooltime(&self, _caster_stat: &StatV1, _caster_level: usize) -> usize {
        METAL_COOLTIME
    }

    fn cooltime_use_count(&self, _caster_stat: &StatV1) -> usize {
        METAL_USE_COUNT
    }

    fn can_use_with_move(&self) -> bool {
        true
    }

    fn casting_target(&self) -> CastingTargetV1 {
        CastingTargetV1::AllyOnlySelf
    }

    fn description(&self) -> String {
        description("skill2")
    }

    fn effect(&self) -> Option<StableEffectSpec> {
        Some(StableEffectSpec {
            range: 0,
            growth_range: 0,
            start_timing: METAL_START_TIMING,
            casting: CastingTypeV1::None,
            target: CastingTargetV1::AllyOnlySelf,
            attack_type: AttackTypeV1::Skill,
            effect: Box::new(FirstMetalbender),
        })
    }
}

/// Blind Bandit. Centred on her, so it needs no target and no range.
struct BlindBanditAction;

impl StableAction for BlindBanditAction {
    fn clone_box(&self) -> Box<dyn StableAction> {
        Box::new(Self)
    }

    fn action_name(&self) -> String {
        "ult".to_string()
    }

    fn duration(&self) -> usize {
        BANDIT_ACTION_TICKS
    }

    fn cooltime(&self, _caster_stat: &StatV1, _caster_level: usize) -> usize {
        BANDIT_COOLTIME
    }

    fn casting_target(&self) -> CastingTargetV1 {
        CastingTargetV1::AllyOnlySelf
    }

    fn description(&self) -> String {
        description("ult")
    }

    fn effect(&self) -> Option<StableEffectSpec> {
        Some(StableEffectSpec {
            range: 0,
            growth_range: 0,
            start_timing: BANDIT_START_TIMING,
            casting: CastingTypeV1::None,
            target: CastingTargetV1::AllyOnlySelf,
            attack_type: AttackTypeV1::Skill,
            effect: Box::new(BlindBandit),
        })
    }
}
