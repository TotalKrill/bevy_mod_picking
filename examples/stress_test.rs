use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
    window::WindowMode,
};

use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    math::clamp,
    prelude::*,
};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use bevy_mod_picking::*;

fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "bevy_mod_picking stress test".to_string(),
            width: 800.,
            height: 600.,
            vsync: false,
            ..Default::default()
        })
        //.add_resource(Msaa { samples: 4 })
        .init_resource::<MouseState>()
        .init_resource::<MouseWheelState>()
        .add_startup_system(set_highlight_params.system())
        .add_startup_system(setup.system())
        .add_plugins(DefaultPlugins)
        .add_plugin(PickingPlugin)
        .add_plugin(DebugPickingPlugin)
        .add_plugin(InteractablePickingPlugin)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(PrintDiagnosticsPlugin::default())
        .add_system(event_example.system())
        .add_system(mouse_middle_click_move_system.system())
        .add_system(mouse_right_click_aim_system.system())
        .add_system(mouse_scroll_zoom_system.system())
        .run();
}

pub fn deactivate_highlightable(commands: &mut Commands) {}

struct Monkey;

/// set up a simple 3D scene
fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let edge_length: u16 = 4;
    println!("Total tris: {}", 3936 * i32::from(edge_length).pow(3));

    // camera
    commands
        .spawn(
            Camera3dBundle::default(), // {
                                       // transform: Transform::from_matrix(Mat4::face_toward(
                                       //     Vec3::new(
                                       //         f32::from(edge_length) * -0.55,
                                       //         f32::from(edge_length) * 0.55,
                                       //         f32::from(edge_length) * 0.45,
                                       //     ),
                                       //     Vec3::new(
                                       //         f32::from(edge_length) * 0.1,
                                       //         0.0,
                                       //         -f32::from(edge_length) * 0.1,
                                       //     ),
                                       //     Vec3::new(0.0, 1.0, 0.0),
                                       // )),
                                       // ..Default::default()
                                       // }
        )
        .with(FlyCamera::default())
        .with(PickSource::default());

    let _scenes: Vec<HandleUntyped> = asset_server.load_folder("models/monkey").unwrap();
    let monkey_handle = asset_server.get_handle("models/monkey/Monkey.gltf#Mesh0/Primitive0");
    for i in 0..edge_length.pow(3) {
        let f_edge_length = edge_length as f32;
        commands
            .spawn(PbrBundle {
                mesh: monkey_handle.clone(),
                material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
                transform: Transform::from_translation(Vec3::new(
                    i as f32 % f_edge_length - f_edge_length / 2.0,
                    (i as f32 / f_edge_length).round() % f_edge_length - f_edge_length / 2.0,
                    (i as f32 / (f_edge_length * f_edge_length)).round() % f_edge_length
                        - f_edge_length / 2.0,
                )) * Transform::from_scale(Vec3::from([0.3, 0.3, 0.3])),
                ..Default::default()
            })
            .with(PickableMesh::default().with_bounding_sphere(monkey_handle.clone()))
            .with(SelectablePickMesh::default())
            .with(HighlightablePickMesh::default())
            .with(Monkey)
            .with(InteractableMesh::default());
    }

    commands.spawn(LightBundle {
        transform: Transform::from_translation(Vec3::new(
            0.0,
            f32::from(edge_length),
            f32::from(edge_length),
        )),
        ..Default::default()
    });
}
fn set_highlight_params(mut highlight_params: ResMut<PickHighlightParams>) {
    highlight_params.set_hover_color(Color::rgb(1.0, 0.0, 0.0));
    highlight_params.set_selection_color(Color::rgb(1.0, 0.0, 1.0));
}
fn event_example(query: Query<(&InteractableMesh, Entity)>) {
    for (interactable, entity) in &mut query.iter() {
        let is_hovered = interactable.hover(&Group::default()).unwrap();
        let hover_event = interactable.hover_event(&Group::default()).unwrap();
        let mouse_down_event = interactable
            .mouse_down_event(&Group::default(), MouseButton::Left)
            .unwrap();
        // Only print updates if at least one event has occured.
        if hover_event.is_none() && mouse_down_event.is_none() {
            continue;
        }
        println!(
            "ENTITY: {:?}, HOVER: {}, HOVER EVENT: {:?}, CLICK_EVENT: {:?}",
            entity, is_hovered, hover_event, mouse_down_event
        );
    }
}

#[derive(Default)]
struct MouseState {
    mouse_motion_event_reader: EventReader<MouseMotion>,
}
#[derive(Default)]
struct MouseWheelState {
    mouse_wheel_event_reader: EventReader<MouseWheel>,
}

fn disable_selectability_system(
    mousebutton_input: Res<Input<MouseButton>>,
    mut query: Query<(&mut Monkey)>,
) {
    if mousebutton_input.pressed(MouseButton::Middle)
        || mousebutton_input.pressed(MouseButton::Right)
    {
        for mut monkey in query.iter_mut() {}
    }
}

fn mouse_scroll_zoom_system(
    time: Res<Time>,
    mut state: ResMut<MouseWheelState>,
    mousebutton_input: Res<Input<MouseButton>>,
    mousewheel_input: Res<Events<MouseWheel>>,
    mut query: Query<(&mut FlyCamera, &mut Transform)>,
) {
    let mut delta: f32 = 0.0;
    for event in state.mouse_wheel_event_reader.iter(&mousewheel_input) {
        delta += event.y;
    }
    if delta.is_nan() {
        return;
    }

    for (mut options, mut transform) in query.iter_mut() {
        let rotation = transform.rotation;
        let accel: Vec3 = (strafe_vector(&rotation) * 0.0)
            + (forward_walk_vector(&rotation) * -delta)
            + (Vec3::unit_y() * 0.0);
        let accel: Vec3 = if accel.length() != 0.0 {
            accel.normalize() * options.speed
        } else {
            Vec3::zero()
        };

        let friction: Vec3 = if options.velocity.length() != 0.0 {
            options.velocity.normalize() * -1.0 * options.friction
        } else {
            Vec3::zero()
        };

        options.velocity += accel * time.delta_seconds() * 4.0;

        // clamp within max speed
        // if options.velocity.length() > options.max_speed {
        //     options.velocity = options.velocity.normalize() * options.max_speed;
        // }

        let delta_friction = friction * time.delta_seconds();

        options.velocity =
            if (options.velocity + delta_friction).signum() != options.velocity.signum() {
                Vec3::zero()
            } else {
                options.velocity + delta_friction
            };

        transform.translation += options.velocity;
    }
}

fn mouse_middle_click_move_system(
    time: Res<Time>,
    mut state: ResMut<MouseState>,
    mousebutton_input: Res<Input<MouseButton>>,
    mouse_motion_events: Res<Events<MouseMotion>>,
    mut query: Query<(&mut FlyCamera, &mut Transform)>,
) {
    if mousebutton_input.pressed(MouseButton::Middle) {
        let mut delta: Vec2 = Vec2::zero();
        for event in state.mouse_motion_event_reader.iter(&mouse_motion_events) {
            delta += event.delta;
        }
        if delta.is_nan() {
            return;
        }

        for (mut options, mut transform) in query.iter_mut() {
            let rotation = transform.rotation;
            let accel: Vec3 = (strafe_vector(&rotation) * -delta.x)
                + (forward_walk_vector(&rotation) * 0.0)
                + (Vec3::unit_y() * delta.y);
            let accel: Vec3 = if accel.length() != 0.0 {
                accel.normalize() * options.speed
            } else {
                Vec3::zero()
            };

            let friction: Vec3 = if options.velocity.length() != 0.0 {
                options.velocity.normalize() * -1.0 * options.friction
            } else {
                Vec3::zero()
            };

            options.velocity = accel * time.delta_seconds();

            // // clamp within max speed
            // if options.velocity.length() > options.max_speed {
            //     options.velocity = options.velocity.normalize() * options.max_speed;
            // }

            // let delta_friction = friction * time.delta_seconds();

            // options.velocity =
            //     if (options.velocity + delta_friction).signum() != options.velocity.signum() {
            //         Vec3::zero()
            //     } else {
            //         options.velocity + delta_friction
            //     };

            transform.translation += options.velocity;
        }
    }
}

fn mouse_right_click_aim_system(
    time: Res<Time>,
    mut state: ResMut<MouseState>,
    mousebutton_input: Res<Input<MouseButton>>,
    mouse_motion_events: Res<Events<MouseMotion>>,
    mut query: Query<(&mut FlyCamera, &mut Transform)>,
) {
    if mousebutton_input.pressed(MouseButton::Right) {
        let mut delta: Vec2 = Vec2::zero();
        for event in state.mouse_motion_event_reader.iter(&mouse_motion_events) {
            delta += event.delta;
        }
        if delta.is_nan() {
            return;
        }
        // Move the camera horizontally/vertically
        for (mut options, mut transform) in query.iter_mut() {
            if !options.enabled {
                continue;
            }
            options.yaw -= delta.x * options.sensitivity * 2.0 * time.delta_seconds();
            options.pitch += delta.y * options.sensitivity * 2.0 * time.delta_seconds();

            options.pitch = clamp(options.pitch, -89.9, 89.9);
            // println!("pitch: {}, yaw: {}", options.pitch, options.yaw);

            let yaw_radians = options.yaw.to_radians();
            let pitch_radians = options.pitch.to_radians();

            transform.rotation = Quat::from_axis_angle(Vec3::unit_y(), yaw_radians)
                * Quat::from_axis_angle(-Vec3::unit_x(), pitch_radians);
        }
    }
}

fn strafe_vector(rotation: &Quat) -> Vec3 {
    // Rotate it 90 degrees to get the strafe direction
    Quat::from_rotation_y(90.0f32.to_radians())
        .mul_vec3(forward_walk_vector(rotation))
        .normalize()
}
fn forward_vector(rotation: &Quat) -> Vec3 {
    rotation.mul_vec3(Vec3::unit_z()).normalize()
}

fn forward_walk_vector(rotation: &Quat) -> Vec3 {
    let f = forward_vector(rotation);
    let f_flattened = Vec3::new(f.x, 0.0, f.z).normalize();
    f_flattened
}
