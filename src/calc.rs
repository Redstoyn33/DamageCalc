use std::collections::HashMap;
use rand::{Rng, thread_rng};
use serde_json::Value;

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
#[serde(default)]
pub struct Stats {
    pub attack: i32,
    pub min_dmg: i32,
    pub max_dmg: i32,
    pub defense: i32,
    pub health: i32,
    pub desc: String,
}
#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
#[serde(default)]
pub struct Unit {
    pub name: String,
    pub stats: Stats,
    pub value: i32,
    pub damage_left: i32,
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Calc {
    pub classes: HashMap<String, Stats>,
}

fn map_json(json: &str) -> Option<(HashMap<String, Stats>, i32)> {
    let mut result = HashMap::new();
    let mut errs = 0;

    if let Ok(value) = serde_json::from_str::<Value>(&json) {
        for (key, value) in value.as_object()? {
            if let Some(stats) = deser_stats(value) {
                result.insert(key.to_string(), stats);
            } else {
                errs += 1;
            }
        }

        Some((result, errs))
    } else {
        None
    }
}

pub fn deser_stats(value: &Value) -> Option<Stats> {
    Some(Stats {
        attack: value["attack"].as_i64()? as i32,
        min_dmg: value["min_dmg"].as_i64()? as i32,
        max_dmg: value["max_dmg"].as_i64()? as i32,
        defense: value["defence"].as_i64()? as i32,
        health: value["health"].as_i64()? as i32,
        desc: value["description"].as_str()?.to_string(),
    })
}

impl Calc {
    pub fn update(&mut self, json: &str) -> i32 {
        if let Some((new_classes, errs)) = map_json(json) {
            self.classes = new_classes;
            errs
        } else {
            -1
        }
    }

    pub fn calculate(
        &self,
        defender: &mut Unit,
        attacker: &mut Unit,
        percent: i32,
        retaliation: bool,
    ) -> (i32, Option<i32>) {
        let astats = &self.classes[&attacker.name];
        let estats = &self.classes[&defender.name];
        let attack = attacker.stats.attack + astats.attack;
        let defence = defender.stats.defense + estats.defense;

        let damage = thread_rng().gen_range((attacker.stats.min_dmg + astats.min_dmg) as f32 * attacker.value as f32 * (percent as f32 / 100.0)..=(attacker.stats.max_dmg + astats.max_dmg) as f32 * attacker.value as f32 * (percent as f32 / 100.0));
        let health = defender.stats.health + estats.health;

        if attack > defence {
            let mut delta = (attack - defence) * 5;
            if delta >= 300 {
                delta = 300;
            }

            let damage_dealt = damage as f32
                * (1.0 + (delta as f32 / 100.0));

            let all_health = (defender.value * health) as f32;

            let creatures_left =
                (all_health - damage_dealt - defender.damage_left as f32) / health as f32;

            defender.value = creatures_left.ceil() as i32;

            defender.damage_left =
                ((creatures_left.ceil() - creatures_left) * health as f32) as i32;

            if retaliation {
                return (
                    damage_dealt as i32,
                    Some(self.calculate(attacker, defender, 100, false).0),
                );
            }
            return (damage_dealt as i32, None);
        } else if attack < defence {
            let mut delta = (defence - attack) as f32 * 2.5;
            if delta >= 70.0 {
                delta = 70.0;
            }

            let damage_dealt = damage as f32
                * (1.0 - (delta as f32 / 100.0));

            let all_health = (defender.value * health) as f32;

            let creatures_left =
                (all_health - damage_dealt - defender.damage_left as f32) / health as f32;

            defender.value = creatures_left.ceil() as i32;

            defender.damage_left =
                ((creatures_left.ceil() - creatures_left) * health as f32) as i32;

            if retaliation {
                return (
                    damage_dealt as i32,
                    Some(self.calculate(attacker, defender, 100, false).0),
                );
            }
            return (damage_dealt as i32, None);
        } else {
            let damage_dealt = damage as f32;

            let all_health = (defender.value * health) as f32;

            let creatures_left =
                (all_health - damage_dealt - defender.damage_left as f32) / health as f32;

            defender.value = creatures_left.ceil() as i32;

            defender.damage_left =
                ((creatures_left.ceil() - creatures_left) * health as f32) as i32;

            if retaliation {
                return (
                    damage_dealt as i32,
                    Some(self.calculate(attacker, defender, 100, false).0),
                );
            }

            return (damage_dealt as i32, None);
        }
    }
}
impl Default for Calc {
    fn default() -> Self {
        Self {
            classes: HashMap::default(),
        }
    }
}
