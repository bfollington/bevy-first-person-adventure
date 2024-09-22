mod post_processing;

use backend::HitData;
use bevy::{prelude::*, render::camera::Exposure};
use bevy_fps_controller::controller::*;
use bevy_mod_picking::backend::PointerHits;
use bevy_mod_picking::backends::raycast::RaycastBackend;
use bevy_mod_picking::picking_core::PickingPluginsSettings;
use bevy_mod_picking::prelude::*;
use bevy_rapier3d::prelude::*;
use post_processing::{PostProcessPlugin, PostProcessSettings};
use rand::Rng;
use std::f32::consts::TAU;

use bevy::window::CursorGrabMode;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(FpsControllerPlugin)
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(PostProcessPlugin)
        .insert_resource(PickingPluginsSettings {
            is_input_enabled: true,
            is_focus_enabled: true,
            ..default()
        })
        .add_plugins(DefaultPickingPlugins)
        .add_systems(Startup, (setup, setup_obstacle_course, setup_reticle))
        .add_systems(Update, (change_object_color, update_fps_camera))
        .add_systems(Update, manage_cursor)
        .run();
}

#[derive(Component)]
struct ClickableObject;

fn setup_obstacle_course(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Pillars
    for i in 0..5 {
        let x = i as f32 * 3.0 - 6.0;
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(0.5, 2.0 + i as f32 * 0.5, 0.5)),
                material: materials.add(Color::srgb(0.6, 0.6, 0.6)),
                transform: Transform::from_xyz(x, (2.0 + i as f32 * 0.5) / 2.0, -5.0),
                ..default()
            },
            Collider::cuboid(0.25, 1.0 + i as f32 * 0.25, 0.25),
        ));
    }

    // Ramps
    let ramp_sizes = [(5.0, 1.0), (5.0, 2.0), (5.0, 3.0)];
    for (i, (length, height)) in ramp_sizes.iter().enumerate() {
        let x = i as f32 * 6.0 - 6.0;
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(*length, *height, 2.0)),
                material: materials.add(Color::srgb(0.7, 0.5, 0.3)),
                transform: Transform::from_xyz(x, height / 2.0, 5.0)
                    .with_rotation(Quat::from_rotation_z(-f32::atan2(*height, *length))),
                ..default()
            },
            Collider::cuboid(length / 2.0, height / 2.0, 1.0),
        ));
    }

    // Bridge
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(10.0, 0.2, 2.0)),
            material: materials.add(Color::srgb(0.4, 0.4, 0.4)),
            transform: Transform::from_xyz(0.0, 3.0, 10.0),
            ..default()
        },
        Collider::cuboid(5.0, 0.1, 1.0),
    ));

    // Bridge supports
    for x in [-5.0, 5.0] {
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(0.5, 3.0, 0.5)),
                material: materials.add(Color::srgb(0.4, 0.4, 0.4)),
                transform: Transform::from_xyz(x, 1.5, 10.0),
                ..default()
            },
            Collider::cuboid(0.25, 1.5, 0.25),
        ));
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // Floor
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(50.0, 50.0)),
            material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        Collider::cuboid(25.0, 0.1, 25.0), // Add a collider to the floor
        PickableBundle::default(),
    ));

    // Clickable objects
    for i in 0..5 {
        let position = Vec3::new(i as f32 * 2.0 - 4.0, 0.5, 0.0);
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
                transform: Transform::from_translation(position),
                ..default()
            },
            Collider::cuboid(0.5, 0.5, 0.5), // Add colliders to the objects
            PickableBundle::default(),
            ClickableObject,
        ));
    }

    // FPS Controller
    let logical_entity = commands
        .spawn((
            TransformBundle::from_transform(Transform::from_xyz(0.0, 5.0, 5.0)),
            Collider::cylinder(3.0 / 2.0, 0.5),
            ActiveEvents::COLLISION_EVENTS,
            LockedAxes::ROTATION_LOCKED,
            AdditionalMassProperties::Mass(1.0),
            Velocity::zero(),
            RigidBody::Dynamic,
            Ccd { enabled: true },
            GravityScale(1.0), // Enable gravity
            Friction {
                coefficient: 0.5,
                combine_rule: CoefficientCombineRule::Average,
            },
            Restitution {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            LogicalPlayer,
            FpsControllerInput {
                pitch: -std::f32::consts::FRAC_PI_8,
                yaw: std::f32::consts::FRAC_PI_4,
                ..default()
            },
            FpsController {
                walk_speed: 10.0,
                run_speed: 20.0,
                jump_speed: 20.0,
                //gravity: -9.81,
                ..default()
            },
        ))
        .insert(CameraConfig {
            height_offset: 0.6, // Adjust this to change the camera height
        })
        .id();

    // Camera
    // commands.spawn((Camera3dBundle::default(), RenderPlayer { logical_entity }));

    commands.spawn((
        Camera3dBundle {
            exposure: Exposure::SUNLIGHT,
            projection: Projection::Perspective(PerspectiveProjection {
                fov: TAU / 5.0,
                ..default()
            }),
            ..default()
        },
        RenderPlayer { logical_entity },
        PostProcessSettings {
            pixel_size: 512.,     // Smaller value for less pixelation
            edge_threshold: 0.5,  // Higher value for less pronounced edges
            color_depth: 16.0,    // Higher value for more colors
            effect_strength: 1.0, // Adjust this to blend with the original image
        },
    ));
}

#[derive(Component)]
struct Reticle;

#[derive(Component)]
struct IgnoreRaycast;

fn setup_reticle(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Px(4.0),
                height: Val::Px(4.0),
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                ..default()
            },
            background_color: Color::rgba(1.0, 1.0, 1.0, 0.5).into(),
            ..default()
        },
        Reticle,
        Pickable::IGNORE, // This will make the reticle ignore picking,
    ));
}

fn change_object_color(
    mut events: EventReader<Pointer<Click>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clickable_objects: Query<(&Handle<StandardMaterial>, &mut ClickableObject)>,
) {
    let mut rng = rand::thread_rng();
    for event in events.read() {
        if let Ok((material_handle, _)) = clickable_objects.get_mut(event.target) {
            if let Some(material) = materials.get_mut(material_handle) {
                material.base_color = Color::srgb(
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                );
            }
        }
    }
}

fn update_fps_camera(
    logical_query: Query<(&Transform, &FpsController), With<LogicalPlayer>>,
    mut render_query: Query<&mut CameraConfig, With<RenderPlayer>>,
) {
    if let (Ok((logical_transform, fps_controller)), Ok(mut camera_config)) =
        (logical_query.get_single(), render_query.get_single_mut())
    {
        camera_config.height_offset = fps_controller.height;
    }
}

fn manage_cursor(
    mut windows: Query<&mut Window>,
    btn: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    let mut window = windows.single_mut();

    if btn.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
        // Center the cursor
        let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(center));
    }

    if key.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }
}
