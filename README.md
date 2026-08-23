# Avatar Wan

Adds Avatar Wan, the first Avatar, as a draftable champion to Teamfight Manager 2.

**Wan comes with his own sprite, skill icons, projectiles, and per-element effects. Some animations are still WIP.**

## Wan's Kit

**Soul of Raava** (Basic Attack)

Wan's basic attacks deal 50% AD physical damage and 50% AD magic damage, and apply a different on-hit effect based on his current element.

**The Avatar Cycle** (Skill 1)

Wan swaps to the next element in-sequence.

- **Fire:** Burn for 12 (+30% AP) magic damage over 3 seconds.
- **Air:** Gain 6% movement speed for 4 seconds (max 4 stacks).
- **Water:** Heal for 3 + 3% of your missing health.
- **Earth:** Basic attacks deal 40% of their damage to nearby enemies.

**Spirit Step** (Skill 2)

Wan dashes a short distance in any direction. For the next 3 seconds, he stores 80% of his damage taken and gains 10% (+0.03% AD) bonus attack speed. When Spirit Step expires, Wan heals himself for 30 + 80% of the stored damage.

**Harmonic Convergence** (Ultimate)

Wan channels Raava's full power, gaining a 300 (+60% AP) (+6% HP) health shield for 6 seconds. For the duration, Wan's basic attacks apply the effects of all 4 elements. Harmonic Convergence's duration is extended by 1.5 seconds whenever Wan gets a takedown.

## Ty Lee's Kit

**Ty Lee's sprite is a starter pass built from the base game's pole_warrior - she still carries his staff, and her skill icons are Wan's placeholders.**

**Chi Blocking** (Basic Attack)

Strike the target for 100% AD physical damage, applying a stack of Chi Block, up to 4 times. The fourth stack consumes them all to deal 20 bonus magic damage and stun the target for 0.5 seconds.

**Three Point Strike** (Skill 1)

Strike the target 3 times in quick succession, each dealing 45 (+45% AD) physical damage and applying Chi Block. Each strike has a chance (based on crit chance) to critically strike for 20% bonus damage.

**Circus Freak** (Skill 2)

For the next 2 seconds, Ty Lee takes (crit chance)% less damage from basic attacks. At the end of the duration, she shields herself for 120 (+20% AD) health for 2 seconds.

**Balancing Act** (Ultimate)

Ty Lee dashes to the highest health enemy champion within 80 range, dealing 180 (+80% AD) physical damage and silencing them for 1.5 seconds. Her next attack always critically strikes and applies Chi Block twice.


## Important

This mod currently supports the English locale only. You can use it with other languages, but Wan's name and skill descriptions will be broken.

If you would like to provide translations, feel free to shoot me a message on Discord @shirograhm.

## Known Issues

The AI does not always understand when to rotate elements, so it may sit on a suboptimal element longer than a human player would.

Circus Freak's mitigation is a flat percentage off basic-attack damage for the window, not a roll to negate individual hits — the engine does not let a mod cancel an incoming hit, and across a fight the two come to the same thing. Its reduction is read off Ty Lee's crit chance once, when the ability is cast, so crit bought mid-window only counts from the next cast.

Balance numbers are still being tuned. Feedback is very welcome.

### Credits

Thank you to the people in the modding discord for their help with the mod-sdk setup, documentation, and general coolness.

### Legalese

This is a free fan-made mod. I am not affiliated with Nickelodeon, Paramount, or the creators of the Avatar: The Last Airbender franchise in any way. Character concepts and names are property of their respective owners.

**[Code Mod Notice]**

This Workshop item contains native/executable code files. Enabling it allows code to run inside the game process. Use only mods from creators you trust.

Files: `avatar_wan_tfm2.dll`

Runs on: Windows
