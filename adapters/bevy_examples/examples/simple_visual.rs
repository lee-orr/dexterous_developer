use bevy::prelude::*;
use bevy_dexterous_developer::*;

reloadable_main!( bevy_main(initial_plugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<DefaultPlugins>())
        .setup_reloadable_elements::<reloadable>()
        .run();
});

#[derive(Component)]
struct Resetabble;

reloadable_scope!(reloadable(app) {
    println!("Adding Setup To List");
    app.reset_setup::<Resetabble, _>(setup);
});

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("Running Setup");
    // circular base
    commands.spawn((
        Resetabble,
        PbrBundle {
            mesh: meshes.add(Circle::new(5.0)),
            material: materials.add(Color::LinearRgba(LinearRgba::GREEN)),
            transform: Transform::from_rotation(Quat::from_rotation_x(
                -std::f32::consts::FRAC_PI_2,
            )),
            ..default()
        },
    ));
    // cube
    commands.spawn((
        Resetabble,
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(Color::srgb_u8(124, 144, 255)),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
    ));
    // light
    commands.spawn((
        Resetabble,
        PointLightBundle {
            point_light: PointLight {
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 8.0, 4.0),
            ..default()
        },
    ));
    // camera
    commands.spawn((
        Resetabble,
        Camera3dBundle {
            transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
    ));
}
