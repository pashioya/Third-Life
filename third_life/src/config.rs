//! Crate that does the loading of configurations. 
//!
//! For more detailed info look at [`ConfigurationPlugin`] , [`ConfigurationLoader`]
//! and the [`proc_macros`] crate.
//!
//! But for the basics of the usage, implement [`ConfigurationLoader`] for a 
//! struct that represents a json file, and add the [`proc_macros::ConfigFile`]
//! and the [`proc_macros::Config`] derive macros on it and only the [`proc_macros::Config`]
//! on any nested structs. 
//! 
//! Any fields that can be omitted should be optional but then have a default 
//! provided with the `#[def(..)]` attribute level macro. If all the fields in 
//! a struct are optional then a `def_conf` function will be created for the struct
//! which makes providing defaults easier.
//!
//! ```
//!#[derive(Resource, Debug, Deserialize, Clone, ConfigFile, Config)]
//! pub struct ThirdLifeConfig {
//!     #[def(1.)]
//!     real_time_day_length: Option<f32>,
//!     #[def(StartingDate::def_conf())]
//!     starting_day: Option<StartingDate>
//! }
//!
//! impl ConfigurationLoader for ThirdLifeConfig {
//!     fn path_with_name() -> &'static str {
//!         "config"
//!     }
//! }
//!
//! #[derive(Config, Debug, Deserialize, Clone)]
//! pub struct StartingDate {
//!     #[def(1.)]
//!     day: Option<f32>,
//!     #[def(1.)]
//!     month: Option<f32>,
//!     #[def(2050.)]
//!     year: Option<f32>
//! }
//! ```


extern crate proc_macro;
use core::panic;
use std::{collections::HashMap, fs, fmt::Debug};


use bevy::{prelude::*, asset::{AssetLoader, io::Reader, LoadContext, AsyncReadExt}, utils::{thiserror::Error, BoxedFuture}};
use bevy_egui::{egui::{Window}, EguiContexts};
use proc_macros::{Config, ConfigFile};
use serde::{Deserialize, de::DeserializeOwned};

use crate::SimulationState;

/// Takes care of registering any configuration that needs to be loaded and then
/// waits for all of them to load before letting the Simulation begin.
///
/// This system is based on the [`ConfigurationLoader`] trait and the related
/// [`RegisterConfigReaderEvent`]/[`ConfigReaderFinishedEvent`] events. Through
/// the traits the first event is cast out which tells the plugins which configurations
/// are to be waited for and the second one then notifies the end of the loading.
/// All of this needs to be done since we only know which exact files need to be
/// loaded once a configuration is selected.
///
/// This struct should not be touched itself, If you want to add configuration
/// look at the [`ConfigurationLoader`] trait and the [`proc_macros::Config`] and
/// [`proc_macros::ConfigFile`] macros
pub struct ConfigurationPlugin;

impl Plugin for ConfigurationPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<AllConfigReaders>()
            .init_resource::<LoadingConfigFileAssets>()
            .add_event::<RegisterConfigReaderEvent>()
            .add_event::<ConfigReaderFinishedEvent>()
            .init_asset_loader::<ConfigFileAssetLoader>()
            .init_asset::<ConfigFileAsset>()
            .add_systems(Update, (show_config_selection).run_if(
                    in_state(SimulationState::ConfigSelection)
            ))
            .add_systems(Update, (register_readers).run_if(
                in_state(SimulationState::ConfigSelection).or_else(in_state(SimulationState::LoadingConfig))
            ))
            .add_systems(Update, (recive_config_loaded_events).run_if(
                in_state(SimulationState::LoadingConfig)
            ))
            .add_plugins(ThirdLifeConfigPlugin);
    }
}

/// Displayes all folders in `assets/config` as selectable configurations to
/// the user.
fn show_config_selection(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut sim_state: ResMut<NextState<SimulationState>>
) {
    let config_options = fs::read_dir("assets/config").unwrap();
    Window::new("Select a config file").show(contexts.ctx_mut(), |ui| {
        for dir in config_options.into_iter() {
            if dir.is_err() || !dir.as_ref().unwrap().metadata().unwrap().is_dir() {
                continue;
            };
            let name = dir.unwrap().file_name().to_string_lossy().to_string();
            if ui.button(name.clone()).clicked() {
                sim_state.set(SimulationState::LoadingConfig);
                commands.insert_resource(SelectedConfigPath::new_std(name));
            }
        }
    });
}

#[derive(Resource)]
pub struct SelectedConfigPath(pub String);

impl SelectedConfigPath {
    pub fn new_std(folder: String) -> Self {
        Self(format!("config/{folder}"))
    }
}

/// Hashmap that contains all of the registered config loaders and whether they
/// have finished loading.
#[derive(Resource, Default)]
struct AllConfigReaders(HashMap<String, LoadingReader>);

/// Loading state of the configuration Loaders.
#[derive(Debug, PartialEq, Eq)]
enum LoadingReader {
    Waiting,
    Recived
}

/// Event to register a Configuration Loader.
#[derive(Event)]
pub struct RegisterConfigReaderEvent(String);
/// Event to notify a Configuration Loader has finished loading.
#[derive(Event)]
pub struct ConfigReaderFinishedEvent(String);

/// Recives registration events
fn register_readers(
    mut all: ResMut<AllConfigReaders>,
    mut events: EventReader<RegisterConfigReaderEvent>
) {
    for event in events.read() {
        println!("registering {}", event.0);
        if all.contains_key(&event.0) {
            panic!(r#"
                Two `RegisterConfigReaderEvent` with the same name were fired.
                This should not happen. Every Config Reader should have its own
                unique name.

                Consider that this error could also happen if an event with the
                same name gets fired twice.
            "#);
        }
        all.insert(event.0.clone(), LoadingReader::Waiting);
    }
}

/// Recives finished loading events
fn recive_config_loaded_events(
    mut all: ResMut<AllConfigReaders>,
    mut events: EventReader<ConfigReaderFinishedEvent>,
    mut sim_state: ResMut<NextState<SimulationState>>
) {
    for event in events.read() {
        println!("finished loading {}", event.0);
        let Some(val) = all.get_mut(&event.0) else {
            panic!(r#"
                A `ConfigReaderFinishedEvent` was recived but the 
                `RegisterConfigReaderEvent` was never sent out. Always make sure
                that both sides are sent out.
            "#);
        };
        match &val {
            LoadingReader::Waiting => { *val = LoadingReader::Recived },
            LoadingReader::Recived => {
                let str = &event.0;
                panic!(r#"
                The hashmap already has a field regarding {str} which could mean
                that an `ConfigReaderFinishedEvent` was already sent out.
                "#);
            }
        }
    }

    if all.iter().all(|(_, e)|e.eq(&LoadingReader::Recived)) {
        sim_state.set(SimulationState::FinishedLoadingConfig);
    }
}

#[derive(Resource, Default)]
struct LoadingConfigFileAssets {
    files: HashMap<String, Handle<ConfigFileAsset>>
}

#[derive(Asset, TypePath, Debug, Deserialize)]
struct ConfigFileAsset {
    file: String
}

#[derive(Default)]
struct ConfigFileAssetLoader;

impl AssetLoader for ConfigFileAssetLoader {
    type Asset = ConfigFileAsset;
    type Settings = ();
    type Error = std::io::Error;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut str = String::new();
            reader.read_to_string(&mut str).await.unwrap();
            let asset = ConfigFileAsset { file: str };
            Ok(asset)
        })
    }
    fn extensions(&self) -> &[&str] {
        &["json"]
    }

}


/// This trait allows any struct to be loaded as a configuration file before the
/// simulation is started. The only thing that needs to be implemented is the 
/// [`Self::path_with_name`] function which provides the trait with a name and path in 
/// which to find the related documentaiton, it is automatically postfixed
/// with `.json`. This string is also used for the registering of the config loader.
///
///
/// The trait should be used in combination with the [`proc_macros::ConfigFile`]
/// derive macro.
pub trait ConfigurationLoader: Sized + DeserializeOwned + Debug + Resource {
    fn path_with_name() -> &'static str;

    fn add_configuration(app: &mut App) {
        app
            .add_systems(Startup, Self::register())
            .add_systems(OnEnter(SimulationState::LoadingConfig), Self::start_loading())
            .add_systems(Update,  (Self::notify_done()).run_if(in_state(SimulationState::LoadingConfig)));
    }

    /// Registers the loader so that [`crate::SimulationState`] is only changed
    /// if all registerd loaders have answered back
    fn register() -> impl Fn(
        EventWriter<RegisterConfigReaderEvent>
    ) + Send + Sync {
        |mut writer: EventWriter<RegisterConfigReaderEvent>| {
            writer.send(
                RegisterConfigReaderEvent::new(Self::path_with_name())
            );
        }
    }

    /// Tells bevy to start loading the asset through the [`bevy_asset::server::AssetServer`]
    /// and stores the handle to  the [`LoadingConfigFileAssets`] resource 
    fn start_loading() -> impl Fn(
        Res<SelectedConfigPath>, Res<AssetServer>, ResMut<LoadingConfigFileAssets>
    ) + Send + Sync {
        |
            selected_config: Res<SelectedConfigPath>,
            asset_server: Res<AssetServer>,
            mut loading_assets: ResMut<LoadingConfigFileAssets>
        | {
            let handle = asset_server.load(format!(
                    "{}/{}.json",
                    selected_config.0.clone(),
                    Self::path_with_name()
            ));
            let name = Self::path_with_name().to_string();
            let None = loading_assets.as_mut()
                .files.insert(name.clone(), handle)
            else {
                panic!(r#"\n
                       The file {name} is already beeing loaded, please check
                       why its beeing loaded for a second time.\n
                "#);
            };

        }
    }

    /// Checks wheter the respective asset has finished loading
    ///
    /// Does this by getting the handle from the [`LoadingConfigFileAssets`] resource,
    /// then looking at the [`ConfigFileAsset`] assets and finding the right one
    /// if [`LoadingConfigFileAssets`] contains the key and the asset is loaded 
    /// the handle is removed from [`LoadingConfigFileAssets`] and a resource of 
    /// the respective type is added to the Simulation.
    ///
    /// Lastly the finished event is cast out.
    fn notify_done() -> impl Fn(
        Commands, EventWriter<ConfigReaderFinishedEvent>, 
        ResMut<LoadingConfigFileAssets>, Res<Assets<ConfigFileAsset>>
    ) + Send + Sync {
        |
            mut commands: Commands,
            mut writer: EventWriter<ConfigReaderFinishedEvent>,
            mut loading_assets: ResMut<LoadingConfigFileAssets>,
            config_assets: Res<Assets<ConfigFileAsset>>,
        | {
            let conf_name = Self::path_with_name().to_string();
            
            let Some(handle) = loading_assets.files.get(&conf_name) else {
                return;
            };
            
            let Some(ConfigFileAsset{ file }) = config_assets.get(handle) else {
                return;
            };

            loading_assets.as_mut().files.remove(&conf_name);

            let config_resource = serde_json::from_str::<Self>(&file).expect(r#"\n
                The file parsed file contains a mistake and could thus not be
                parsed plase check that the formatting of the file is correct and
                matches the type you are trying to parse it to!\n
            "#);

            commands.insert_resource(config_resource);
            writer.send(ConfigReaderFinishedEvent::new(Self::path_with_name()));
        }
    }

}


#[derive(Resource, Debug, Deserialize, Clone, ConfigFile, Config)]
pub struct ThirdLifeConfig {
    #[def(1.)]
    real_time_day_length: Option<f32>,
    #[def(StartingDate::def_conf())]
    starting_day: Option<StartingDate>
}

impl ConfigurationLoader for ThirdLifeConfig {
    fn path_with_name() -> &'static str {
        "config"
    }
}

#[derive(Config, Debug, Deserialize, Clone)]
pub struct StartingDate {
    #[def(1)]
    day: Option<u32>,
    #[def(1)]
    month: Option<u32>,
    #[def(2050)]
    year: Option<i32>
}

impl std::ops::Deref for AllConfigReaders {
    type Target = HashMap<String, LoadingReader>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for AllConfigReaders {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RegisterConfigReaderEvent {
    pub fn new(str: &str) -> Self {
        Self(str.to_string())
    }
}

impl ConfigReaderFinishedEvent {
    pub fn new(str: &str) -> Self {
        Self(str.to_string())
    }
}

