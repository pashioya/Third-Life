use std::ops::Deref;

use bevy::prelude::*;
use bevy_async_task::{AsyncTaskRunner, AsyncTaskStatus};
use chrono::Local;
use sqlx::postgres::PgPoolOptions;

use crate::{config::ThirdLifeConfig, worlds::config::WorldsConfig, SimulationState};

use super::components::{
    LoadedDatabaseEvent, LoadingDatabase, LoadingDatabases, PostgresDB, RegisterDatabseEvent,
    SimulationUuid,
};

const POSTGRES_DB: &'static str = "postgres_db";

pub struct PostgresDbPlugin;

impl Plugin for PostgresDbPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(
            Startup,
            register_postgres
        ).add_systems(
            Update,
            init_postgres_db.run_if(in_state(SimulationState::LoadingDatabases)),
        );
    }
}

pub fn register_postgres(mut register_influx: EventWriter<RegisterDatabseEvent>) {
    info!("Registering Postgres DB.");
    register_influx.send(RegisterDatabseEvent(String::from(POSTGRES_DB)));
}

fn init_postgres_db(
    mut commands: Commands,
    mut task_executor: AsyncTaskRunner<PostgresDB>,
    mut loaded_database: EventWriter<LoadedDatabaseEvent>,
    general_config: Res<ThirdLifeConfig>,
    worlds_config: Res<WorldsConfig>,
    simulation_uuid: Res<SimulationUuid>,
    loading_databases: Res<LoadingDatabases>,
) {
    let clear = false;
    let general_config = general_config.deref().clone();
    let worlds_config = worlds_config.deref().clone();
    let simulation_uuid = simulation_uuid.deref().deref().clone();

    match task_executor.poll() {
        AsyncTaskStatus::Idle => {
            let Some(status) = loading_databases.0.get(POSTGRES_DB) else {
                panic!("postgres db was never registered!");
            };
            if *status != LoadingDatabase::Waiting {
                return;
            };
            info!("Starting Postgres DB connection.");
            task_executor.start(async move {
                let pool = PgPoolOptions::new()
                    .max_connections(5)
                    .connect("postgres://postgres:postgres@localhost:5433/simulation")
                    .await
                    .unwrap();

                if clear {
                    let _ = sqlx::query(
                        r#"drop table if exists public.simulation_runs;"#,
                        )
                        .execute(&pool)
                        .await
                        .unwrap();
                }


                let _ = sqlx::query(
                    r#"
                    create table if not exists public.simulation_runs (
                        uuid varchar(255) not null primary key,
                        time_created timestamptz not null,
                        description text not null,
                        general_config text not null,
                        worlds_config text not null
                    );
                "#,
                )
                .execute(&pool)
                .await
                .unwrap();

                if clear {
                    let _ = sqlx::query(
                        r#"drop table if exists public.simulation_records;"#,
                        )
                        .execute(&pool)
                        .await
                        .unwrap();
                }

                let _ = sqlx::query(
                    r#"
                    create table if not exists public.simulation_records (
                        uuid varchar(255) not null,
                        date date not null,
                        colony INT8 not null,
                        total_pop INT4 not null,
                        average_age FLOAT4 not null,
                        younglings INT4 not null,
                        working_pop INT4 not null,
                        retirees INT4 not null,
                        average_children_per_mother FLOAT4 not null,
                        infant_deaths INT4 not null,
                        starvation_deaths INT4 not null,
                        old_age_death INT4 not null,
                        citizen_created INT4 not null,
                        meat_resources FLOAT4 not null,
                        meat_quality FLOAT4 not null,
                        meat_consumed FLOAT4 not null,
                        carb_resources FLOAT4 not null,
                        carb_quality FLOAT4 not null,
                        carb_consumed FLOAT4 not null,
                        meat_produced FLOAT4 not null,
                        carb_produced FLOAT4 not null
                    );
                "#)
                    .execute(&pool)
                    .await
                    .unwrap();

                let _ = sqlx::query(
                    r#"
                    insert into simulation_runs(
                        uuid, time_created, description, general_config, worlds_config
                    ) values (
                        $1, $2, $3, $4, $5
                    );
                "#,
                )
                .bind(simulation_uuid)
                .bind(Local::now())
                .bind("test")
                .bind(serde_json::to_string(&general_config).unwrap())
                .bind(serde_json::to_string(&worlds_config).unwrap())
                .execute(&pool)
                .await
                .unwrap();

                PostgresDB::new(pool)
            });
        }
        AsyncTaskStatus::Pending => (),
        AsyncTaskStatus::Finished(postgres) => {
            info!("Finished Postgres DB connection.");
            commands.insert_resource(postgres);
            loaded_database.send(LoadedDatabaseEvent(String::from(POSTGRES_DB)));
        }
    }
}