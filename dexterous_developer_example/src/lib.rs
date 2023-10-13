use bevy::prelude::*;

#[allow(unused_imports)]
use dexterous_developer::{
    dexterous_developer_setup, hot_bevy_main, InitialPlugins, ReloadableApp, ReloadableAppContents,
    ReloadableElementsSetup, ReplacableComponent, ReplacableResource,
};
use dexterous_developer::{ReloadMode, ReloadSettings, ReloadableSetup, ReplacableState};
use serde::{Deserialize, Serialize};

#[hot_bevy_main]
pub fn bevy_main(initial_plugins: impl InitialPlugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<DefaultPlugins>())
        .add_systems(Startup, setup)
        .setup_reloadable_elements::<reloadable>()
        .setup_reloadable_elements::<reloadable_2>()
        .insert_resource(ReloadSettings {
            display_update_time: true,
            manual_reload: Some(KeyCode::F2),
            toggle_reload_mode: Some(KeyCode::F1),
            reload_mode: ReloadMode::Full,
            reloadable_element_policy: dexterous_developer::ReloadableElementPolicy::OneOfList(
                KeyCode::F3,
                vec![
                    reloadable::setup_function_name(),
                    reloadable_2::setup_function_name(),
                ],
            ),
            reloadable_element_selection: None,
        })
        .run();
}

#[derive(States, PartialEq, Eq, Clone, Copy, Debug, Hash, Default, Serialize, Deserialize)]
pub enum AppState {
    #[default]
    AnotherState,
    State,
    TwoSpheres,
}

impl ReplacableState for AppState {
    fn get_type_name() -> &'static str {
        "app-state"
    }

    fn get_next_type_name() -> &'static str {
        "next-app-state"
    }
}

#[dexterous_developer_setup(first_reloadable)]
fn reloadable(app: &mut ReloadableAppContents) {
    app.add_state::<AppState>();
    println!("Setting up reloadabless #1");
    app.add_systems(Update, (move_cube, toggle));
    println!("Reset Setup");
    app.reset_setup::<Cube, _>(setup_cube);
    println!("Reset Setup In State");
    app.reset_setup_in_state::<Sphere, AppState, _>(AppState::AnotherState, setup_sphere);
    app.reset_setup_in_state::<Sphere, AppState, _>(AppState::TwoSpheres, setup_two_spheres);
    println!("Done");
}

#[dexterous_developer_setup(second_reloadable)]
fn reloadable_2(app: &mut ReloadableAppContents) {
    println!("Setting up reloadabless #2");
    app.add_systems(Update, advance_time);
    println!("Reset Resource");
    app.reset_resource::<VelocityMultiplier>();
    println!("Done");
}

#[derive(Component, Serialize, Deserialize)]
struct Cube(Vec3);

impl Default for Cube {
    fn default() -> Self {
        Self((Vec3::NEG_X + Vec3::Y) * 0.89)
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
struct VelocityMultiplier(Vec3, f32);

impl Default for VelocityMultiplier {
    fn default() -> Self {
        Self(Vec3::new(0.5, 0., 2.5), 0.)
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
    #[cfg(feature = "orange")]
    let cube_color = Color::ORANGE;

    #[cfg(not(feature = "orange"))]
    let cube_color = Color::YELLOW;

    // cube
    commands.spawn((
        Cube::default(),
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(cube_color.into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
    ));
    commands.spawn((
        Cube(Vec3::Z * 2. + Vec3::Y),
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(cube_color.into()),
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
                radius: 0.2,
                ..Default::default()
            })),
            material: materials.add(Color::PINK.into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
    ));
}

fn setup_two_spheres(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Sphere,
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.2,
                ..Default::default()
            })),
            material: materials.add(Color::PINK.into()),
            transform: Transform::from_xyz(1.0, 0.5, 0.0),
            ..default()
        },
    ));
    commands.spawn((
        Sphere,
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.2,
                ..Default::default()
            })),
            material: materials.add(Color::PINK.into()),
            transform: Transform::from_xyz(-1.0, 0.5, 0.0),
            ..default()
        },
    ));
}

#[allow(unused)]
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
        y: position.y.sin() + multiplier.1 / 10.,
        z: position.z.sin(),
    };

    for (mut transform, base) in cubes.iter_mut() {
        transform.translation = position + base.0;
    }
}

fn advance_time(mut multiplier: ResMut<VelocityMultiplier>, time: Res<Time>) {
    multiplier.1 += time.delta_seconds();
}

fn toggle(input: Res<Input<KeyCode>>, mut commands: Commands, current: Res<State<AppState>>) {
    if input.just_pressed(KeyCode::Space) {
        let next = match current.get() {
            AppState::State => AppState::AnotherState,
            AppState::AnotherState => AppState::TwoSpheres,
            AppState::TwoSpheres => AppState::State,
        };
        commands.insert_resource(NextState(Some(next)));
    }
}
