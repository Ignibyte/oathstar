# Progression System

Oathstar should use a hybrid progression model:

- Character levels for broad growth and satisfying milestones
- Percentage-based skills that improve through use
- Skill points for unlocking or enabling specific skills
- Oaths, region standing, and gear for world-facing progression

## Core Direction

The skill model should be inspired by classic MUD systems such as ROM/ROT-style percentage skills.

Skills are represented as percentages.

Example:

- `firstAttack: 82%`
- `secondAttack: 41%`
- `thirdAttack: 8%`
- `parry: 55%`
- `persuade: 34%`
- `bindOath: 22%`

Using a skill gives it a chance to improve.

This means mastery comes from practice, not only from spending points.

## Character Levels

Leveling should still exist because it feels good and supports grinding.

Levels can grant:

- Health increases
- Focus increases
- Skill points
- Access to trainers
- Access to skill tiers
- Broad combat survivability

Levels should not be the only source of character identity.

## Skill Percentages

Skills should improve through use.

Possible rules:

- A skill has a percentage from 0 to 100.
- Skill checks roll against the percentage.
- Success and/or failure can trigger improvement chances.
- Lower skills may improve faster.
- Higher skills may require harder encounters, trainers, oaths, or region access.

Open tuning questions:

- Can skills improve on failure?
- Should improvement be capped by level?
- Should trainers be required past certain thresholds?
- Should combat skills improve faster against stronger enemies?
- Should skills decay? Initial recommendation: no.

## Skill Points

Skill points should mostly unlock skills rather than directly increase raw percentages.

Possible uses:

- Learn a new skill
- Unlock a higher-tier skill
- Unlock a new combat technique
- Raise a skill cap
- Open a specialization path
- Buy initial training so the skill can improve through use

Examples:

- Spend a point to learn `second attack`.
- Once learned, `second attack` starts at a low percentage and improves by triggering in combat.
- Spend a point to unlock `bind oath`.
- `bind oath` then improves through use in oath conflicts.

This keeps skill points meaningful without replacing practice-based mastery.

## Skill Categories

Possible first categories:

- Weapon skills
- Defensive skills
- Combat rhythm skills
- Oath/ritual skills
- Social skills
- Exploration/lore skills

Example combat skills:

- First attack
- Second attack
- Third attack
- Parry
- Dodge
- Riposte
- Ward

Example oath/social skills:

- Persuade
- Intimidate
- Bind oath
- Invoke witness
- Read intent
- Remember name

## Progression Sources

XP can come from:

- Combat
- Bosses
- Oaths fulfilled
- Regional objectives
- Discovery milestones

Skill improvement comes from:

- Using the skill
- Training
- Special oaths
- Boss rewards
- Region standing unlocks

World progression comes from:

- Region standing
- Oaths fulfilled or broken
- Important NPC memory
- Routes opened
- New verbs or rituals

## Design Guardrail

Grinding should feel useful, but not replace authored progression.

Combat can make the player stronger. Oaths, regions, and story choices should still decide what kind of person the player is becoming and what parts of the world open to them.
