use rand::{thread_rng, Rng};
use serde_json::Value;
use std::collections::HashMap;

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
#[serde(default)]
pub struct Stats {
    pub attack: i32,
    pub min_dmg: i32,
    pub max_dmg: i32,
    pub defense: i32,
    pub health: i32,
    pub luck: i32,
    pub leadership: i32,
    pub absorb: i32,
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
    let desc = value["description"].as_str()?.to_string();
    let (luck, leadership) = match (value["luck"].as_i64(), value["leadership"].as_i64()) {
        (Some(v1), Some(v2)) => (v1 as i32, v2 as i32),
        (None, None) => {
            let (v1, v2) = Calc::parse_old_luck_and_leadership(&desc);
            (v1.unwrap_or(0), v2.unwrap_or(0))
        }
        (Some(v1), None) => (v1 as i32, Calc::parse_old_luck_and_leadership(&desc).1.unwrap_or(0)),
        (None, Some(v2)) => (Calc::parse_old_luck_and_leadership(&desc).0.unwrap_or(0), v2 as i32),
    };
    Some(Stats {
        attack: value["attack"].as_i64()? as i32,
        min_dmg: value["min_dmg"].as_i64()? as i32,
        max_dmg: value["max_dmg"].as_i64()? as i32,
        defense: value["defence"].as_i64()? as i32,
        health: value["health"].as_i64()? as i32,
        luck,
        leadership,
        absorb: 0,
        desc,
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

    pub fn parse_old_luck_and_leadership(desc: &String) -> (Option<i32>, Option<i32>) {
        (
            if let Some(luck_start) = desc.find("Удача:") {
                let luck_start = luck_start + "Удача:".len();
                if let Some(luck_end) = desc[luck_start..].find(',') {
                    if let Ok(v) = desc[luck_start..luck_start+luck_end].trim().parse::<i32>() {
                        Some(v)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            },
            if let Some(leadership_start) = desc.find("Лидерство:") {
                let leadership_start = leadership_start + "Лидерство:".len();
                if let Some(leadership_end) = desc[leadership_start..].find(',') {
                    if let Ok(v) = desc[leadership_start..leadership_start+leadership_end].trim().parse::<i32>() {
                        Some(v)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            },
        )
    }

    pub fn calculate(
        &self,
        defender: &mut Unit,
        attacker: &mut Unit,
        percent: i32,
        retaliation: bool,
    ) -> (i32, Option<(i32, [String; 2])>, [String; 2]) {
        let mut strings: [String; 2] = [String::new(), String::new()];
        let astats = &self.classes[&attacker.name];
        let attacker_luck = attacker.stats.luck + astats.luck;
        let attacker_leadership = attacker.stats.leadership + astats.leadership;
        let estats = &self.classes[&defender.name];
        let attack = attacker.stats.attack + astats.attack;
        let defence = defender.stats.defense + estats.defense;

        let luck = thread_rng().gen_range(0..100);
        if luck < attacker_luck {
            strings[0] = "Ооо повезло-повезло!".to_string();
        }

        let leadership = thread_rng().gen_range(0..100);
        if leadership < attacker_leadership {
            strings[1] = "Ебаны рот погнали!".to_string();
        }

        let damage = thread_rng().gen_range(
            attacker.stats.min_dmg + astats.min_dmg..=attacker.stats.max_dmg + astats.max_dmg,
        );
        let health = defender.stats.health + estats.health;

        if attack > defence {
            let mut delta = (attack - defence) * 5;
            if delta >= 300 {
                delta = 300;
            }

            let damage_dealt = (damage * attacker.value) as f32
                * (1.0 + (delta as f32 / 100.0))
                * (percent as f32 / 100.0);

            if damage_dealt <= defender.stats.absorb as f32 {
                defender.stats.absorb -= damage_dealt as i32;
                if retaliation {
                    let (x, _, y) = self.calculate(attacker, defender, 100, false);
                    return (0, Some((x, y)), strings);
                }

                return (0, None, strings);
            }
            let damage_dealt = damage_dealt - defender.stats.absorb as f32;
            defender.stats.absorb = 0;

            let all_health = (defender.value * health) as f32;

            let creatures_left =
                (all_health - damage_dealt - defender.damage_left as f32) / health as f32;

            defender.value = creatures_left.ceil() as i32;

            defender.damage_left =
                ((creatures_left.ceil() - creatures_left) * health as f32) as i32;

            if retaliation {
                let (x, _, y) = self.calculate(attacker, defender, 100, false);
                return (damage_dealt as i32, Some((x, y)), strings);
            }
            return (damage_dealt as i32, None, strings);
        } else if attack < defence {
            let mut delta = (defence - attack) as f32 * 2.5;
            if delta >= 70.0 {
                delta = 70.0;
            }

            let damage_dealt = (damage * attacker.value) as f32
                * (1.0 - (delta as f32 / 100.0))
                * (percent as f32 / 100.0);

            if damage_dealt <= defender.stats.absorb as f32 {
                defender.stats.absorb -= damage_dealt as i32;
                if retaliation {
                    let (x, _, y) = self.calculate(attacker, defender, 100, false);
                    return (0, Some((x, y)), strings);
                }

                return (0, None, strings);
            }
            let damage_dealt = damage_dealt - defender.stats.absorb  as f32;
            defender.stats.absorb = 0;

            let all_health = (defender.value * health) as f32;

            let creatures_left =
                (all_health - damage_dealt - defender.damage_left as f32) / health as f32;

            defender.value = creatures_left.ceil() as i32;

            defender.damage_left =
                ((creatures_left.ceil() - creatures_left) * health as f32) as i32;

            if retaliation {
                let (x, _, y) = self.calculate(attacker, defender, 100, false);
                return (damage_dealt as i32, Some((x, y)), strings);
            }
            return (damage_dealt as i32, None, strings);
        } else {
            let damage_dealt = (damage * attacker.value) as f32 * (percent as f32 / 100.0);

            if damage_dealt <= defender.stats.absorb as f32 {
                defender.stats.absorb -= damage_dealt as i32;
                if retaliation {
                    let (x, _, y) = self.calculate(attacker, defender, 100, false);
                    return (0, Some((x, y)), strings);
                }

                return (0, None, strings);
            }
            let damage_dealt = damage_dealt - defender.stats.absorb as f32;
            defender.stats.absorb = 0;

            let all_health = (defender.value * health) as f32;

            let creatures_left =
                (all_health - damage_dealt - defender.damage_left as f32) / health as f32;

            defender.value = creatures_left.ceil() as i32;

            defender.damage_left =
                ((creatures_left.ceil() - creatures_left) * health as f32) as i32;

            if retaliation {
                let (x, _, y) = self.calculate(attacker, defender, 100, false);
                return (damage_dealt as i32, Some((x, y)), strings);
            }

            return (damage_dealt as i32, None, strings);
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
