use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use hot_reload::{hot_bevy_main, hot_reload_setup, *};
use serde::{Deserialize, Serialize};

#[hot_bevy_main]
pub fn bevy_main(initial: InitialPlugins) {
    println!("Creating app");
    let mut app = App::new();
    app.add_state::<AppState>()
        .add_plugins(initial.with_default_plugins())
        .add_plugins(WorldInspectorPlugin::new())
        .add_systems(Startup, setup)
        .setup_reloadable_elements::<reloadable>();

    println!("Run App: {:?}", std::thread::current().id());

    app.run();
}

#[derive(States, PartialEq, Eq, Clone, Copy, Debug, Hash, Default)]
pub enum AppState {
    #[default]
    State,
    AnotherState,
}

#[hot_reload_setup]
fn reloadable(app: &mut ReloadableAppContents) {
    println!("Setting up reloadables");
    app.add_systems(Update, (move_cube, toggle))
        .insert_replacable_resource::<VelocityMultiplier>()
        .reset_setup::<Cube, _>(setup_cube)
        .reset_setup_in_state::<Sphere, AppState, _>(AppState::AnotherState, setup_sphere);
}

#[derive(Component, Serialize, Deserialize)]
struct Cube(Vec3);

impl Default for Cube {
    fn default() -> Self {
        Self(Vec3::NEG_X * 1.)
    }
}

impl ReplacableComponent for Cube {
    fn get_type_name() -> &'static str {
        "cube"
    }
}

#[derive(Component, Default)]
pub struct Sphere;

#[derive(Resource, serde::Serialize, serde::Deserialize, Debug)]
struct VelocityMultiplier(Vec3);

impl Default for VelocityMultiplier {
    fn default() -> Self {
        Self(Vec3::new(0., 3., 0.))
    }
}

impl ReplacableResource for VelocityMultiplier {
    fn get_type_name() -> &'static str {
        "VelocityMultiplier"
    }
}

fn setup_cube(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // cube
    commands.spawn((
        Cube::default(),
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
    ));
    commands.spawn((
        Cube(Vec3::Z * 2.),
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
    ));
}

fn setup_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Sphere,
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.1,
                ..Default::default()
            })),
            material: materials.add(Color::ORANGE.into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
    ));
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn move_cube(
    mut cubes: Query<(&mut Transform, &Cube)>,
    time: Res<Time>,
    multiplier: Res<VelocityMultiplier>,
) {
    let position = time.elapsed_seconds() * multiplier.0;
    let position = Vec3 {
        x: position.x.sin(),
        y: position.y.sin(),
        z: position.z.sin(),
    };

    for (mut transform, base) in cubes.iter_mut() {
        transform.translation = position + base.0;
    }
}

fn toggle(input: Res<Input<KeyCode>>, mut commands: Commands, current: Res<State<AppState>>) {
    if input.just_pressed(KeyCode::Space) {
        let next = match current.get() {
            AppState::State => AppState::AnotherState,
            AppState::AnotherState => AppState::State,
        };
        commands.insert_resource(NextState(Some(next)));
    }
}
