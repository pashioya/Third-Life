pub mod components;
use self::components::*;
pub mod events;
use self::events::*;
pub mod wheat_farming;
pub mod cow_farming;
use crate::worlds::food::{cow_farming::*, wheat_farming::*};

use std::usize;

use crate::{time::DateChanged, SimulationState};
use bevy::{prelude::*, reflect::List, utils::HashMap};
use bevy_egui::{
    egui::{Window},
    EguiContexts,
};

use super::{
    init_colonies,
    population::components::{CitizenOf},
    WorldColony,
};

pub struct FoodPlugin;
impl Plugin for FoodPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(SimulationState::Running),
            init_food.after(init_colonies),
        )
        .add_systems(
            Update,
            (
                check_farm_workers,
                get_farm_workers,
                work_farm,
                check_cow_farm_workers,
                get_cow_farm_workers,
                work_cow_farm,
                cook_food
            )
            .run_if(in_state(SimulationState::Running)),
        )
        .add_event::<WheatFarmNeedsWorker>()
        .add_event::<CowFarmNeedsWorker>()
        .add_event::<MeatCreated>()
        .add_event::<CarbCreated>()
        .add_event::<MeatConsumed>()
        .add_event::<CarbConsumed>()
        .add_event::<FoodCreated>();
    }
}


fn init_food(mut commands: Commands, colonies: Query<Entity, With<WorldColony>>) {
    for colony in colonies.iter() {
        commands.spawn((
            WheatFarm {
                size: 17.4,
                harvested: 0.0,
            },
            WheatFarmOf { colony },
        ));
        commands.spawn((
            WheatFarm {
                size: 17.4,
                harvested: 0.0,
            },
            WheatFarmOf { colony },
        ));
        commands.spawn((
            WheatFarm {
                size: 17.4,
                harvested: 0.0,
            },
            WheatFarmOf { colony },
        ));

        commands.spawn((
            WheatFarm {
                size: 17.4,
                harvested: 0.0,
            },
            WheatFarmOf { colony },
        ));
        commands.spawn((
            WheatFarm {
                size: 17.4,
                harvested: 0.0,
            },
            WheatFarmOf { colony },
        ));
        commands.spawn((
            WheatFarm {
                size: 17.4,
                harvested: 0.0,
            },
            WheatFarmOf { colony },
        ));
        commands.spawn((
            CowFarm {
                size: 500.0,
                harvested: 0.0,
            },
            CowFarmOf { colony },
        ));
        commands.spawn((
            CowFarm {
                size: 500.0,
                harvested: 0.0,
            },
            CowFarmOf { colony },
        ));
        commands.spawn((
            CowFarm {
                size: 500.0,
                harvested: 0.0,
            },
            CowFarmOf { colony },
        ));
        commands.spawn((FoodResource { amount: 0.0 }, ResourceOf { colony }));
        commands.spawn((CarbResource { amount: 0.0 }, ResourceOf { colony }));
        commands.spawn((MeatResource { amount: 0.0 }, ResourceOf { colony }));
    }
}

fn cook_food(
    mut food_resources: Query<(Entity, &mut FoodResource, &ResourceOf)>,
    mut carb_resources: Query<(Entity, &mut CarbResource, &ResourceOf)>,
    mut meat_resources: Query<(Entity, &mut MeatResource, &ResourceOf)>,
    mut food_created: EventWriter<FoodCreated>,
    mut carb_consumed: EventWriter<CarbConsumed>,
    mut meat_consumed: EventWriter<MeatConsumed>,
) {
    let mut colony_food_resources_map = food_resources.iter_mut().fold(
        HashMap::new(),
        |mut acc: HashMap<Entity, HashMap<&str, Entity>>, (entity, _, colony)| {
            acc.entry(colony.colony)
                .or_insert(HashMap::new())
                .entry("food")
                .or_insert(entity);
            acc
        },
    );

    for (entity, _, resource_of) in carb_resources.iter_mut() {
        colony_food_resources_map
            .get_mut(&resource_of.colony)
            .unwrap()
            .entry("carb")
            .or_insert(entity);
    }

    for (entity, _, resource_of) in meat_resources.iter_mut() {
        colony_food_resources_map
            .get_mut(&resource_of.colony)
            .unwrap()
            .entry("meat")
            .or_insert(entity);
    }

    for (colony, resource_entities) in colony_food_resources_map {
        let (_, mut food_resource, _) = food_resources
            .get_mut(*resource_entities.get("food").unwrap())
            .unwrap();
        let (_, mut carb_resource, _) = carb_resources
            .get_mut(*resource_entities.get("carb").unwrap())
            .unwrap();
        let (_, mut meat_resource, _) = meat_resources
            .get_mut(*resource_entities.get("meat").unwrap())
            .unwrap();

        let food_cook_multiplier = 5.0;

        if carb_resource.amount > 100.0  * food_cook_multiplier
            && meat_resource.amount > 100.0 * food_cook_multiplier {
            let carb_consumed_amount = 100.0 * food_cook_multiplier;
            let meat_consumed_amount = 100.0 * food_cook_multiplier;
            let food_created_amount = 100.0 * food_cook_multiplier;

            carb_resource.amount -= carb_consumed_amount;
            meat_resource.amount -= meat_consumed_amount;
            food_resource.amount += food_created_amount;

            carb_consumed.send(CarbConsumed { colony, amount: carb_consumed_amount });
            meat_consumed.send(MeatConsumed { colony, amount: meat_consumed_amount });
            food_created.send(FoodCreated { colony, amount: food_created_amount });
        }
    }
}
