use bevy::{prelude::*, utils::HashMap};

use crate::{time::DateChanged, worlds::population::components::CitizenOf};

use super::{
    CowFarm, CowFarmNeedsWorker, CowFarmOf, CowFarmer, Employed, MeatCreated, MeatResource,
    ResourceOf,
};

pub fn check_cow_farm_workers(
    mut day_changed_event_reader: EventReader<DateChanged>,
    mut event_writer: EventWriter<CowFarmNeedsWorker>,
    cow_farms: Query<(Entity, &CowFarmOf), With<CowFarm>>,
    farmers: Query<(&CowFarmer, &CitizenOf)>,
) {
    for _ in day_changed_event_reader.read() {
        let mut farms_map = cow_farms.iter().fold(
            HashMap::new(),
            |mut acc: HashMap<Entity, HashMap<Entity, usize>>, (farm_entity, cow_farm_of)| {
                acc.entry(cow_farm_of.colony)
                    .or_insert(HashMap::new())
                    .entry(farm_entity)
                    .or_insert(0);
                acc
            },
        );

        for (cow_farmer, colony_of) in farmers.iter() {
            farms_map
                .get_mut(&colony_of.colony)
                .unwrap()
                .entry(cow_farmer.farm)
                .and_modify(|count| *count += 1);
        }

        for (colony, farms) in farms_map {
            for (farm, farmer_count) in farms {
                if farmer_count < 4 {
                    for _ in 0..(4 - farmer_count) {
                        event_writer.send(CowFarmNeedsWorker { colony, farm });
                    }
                }
            }
        }
    }
}

pub fn get_cow_farm_workers(
    mut commands: Commands,
    mut event_reader: EventReader<CowFarmNeedsWorker>,
    free_citizens: Query<(Entity, &CitizenOf), Without<Employed>>,
) {
    for needs_worker_event in event_reader.read() {
        for (citizen, citizen_of) in free_citizens.iter() {
            if citizen_of.colony == needs_worker_event.colony {
                commands.get_entity(citizen).map(|mut c| {
                    c.try_insert((
                        CowFarmer {
                            farm: needs_worker_event.farm,
                        },
                        Employed,
                    ));
                });
                break;
            }
        }
    }
}

pub fn work_cow_farm(
    mut day_changed_event_reader: EventReader<DateChanged>,
    mut cow_farms: Query<(Entity, &mut CowFarm, &CowFarmOf)>,
    farmers: Query<(&CowFarmer, &CitizenOf)>,
    mut meat_resources: Query<(&mut MeatResource, &ResourceOf)>,
    mut meat_created: EventWriter<MeatCreated>,
) {
    for _ in day_changed_event_reader.read() {
        let mut farms_map = cow_farms.iter_mut().fold(
            HashMap::new(),
            |mut acc: HashMap<Entity, HashMap<Entity, usize>>, (farm_entity, _, wheat_farm_of)| {
                acc.entry(wheat_farm_of.colony)
                    .or_insert(HashMap::new())
                    .entry(farm_entity)
                    .or_insert(0);
                acc
            },
        );

        for (cow_farmer, colony_of) in farmers.iter() {
            farms_map
                .get_mut(&colony_of.colony)
                .unwrap()
                .entry(cow_farmer.farm)
                .and_modify(|count| *count += 1);
        }

        for (colony, farms) in farms_map {
            for (farm_entity, farmer_count) in farms {
                let (_, mut cow_farm, _) = cow_farms.get_mut(farm_entity).unwrap();
                // 1.0 signifies multiplier for 1 8 hour work day
                // harvested_amount is in ha
                let mut harvested_amount = 1.0 * (farmer_count as f32);
                if harvested_amount > (cow_farm.size / 2.0) - cow_farm.harvested {
                    harvested_amount = (cow_farm.size / 2.0) - cow_farm.harvested;
                }
                cow_farm.harvested += harvested_amount;
                if harvested_amount > 0.0 {
                    for (mut meat_resource, resource_of) in meat_resources.iter_mut() {
                        if resource_of.colony == colony {
                            //todo: need to figure out 1 day of work= how many kilos meat.
                            let amount = harvested_amount * 2000.0;
                            meat_resource.amount += amount;
                            meat_created.send(MeatCreated { colony, amount });
                        }
                    }
                }
            }
        }
    }
}
