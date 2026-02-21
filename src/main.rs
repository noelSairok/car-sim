use bevy::prelude::*;
use std::f32::consts::PI;

// ==================== CONFIGURATION ====================
const CAR_COUNT: usize = 1;
const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 720.0;

const BOUNDARY_THICKNESS: f32 = 20.0;
const CAR_WIDTH: f32 = 30.0;
const CAR_LENGTH: f32 = 60.0;
const CAR_RADIUS: f32 = 25.0;

const START_POSITION: Vec2 = Vec2::new(-350.0, 0.0);
const START_ROTATION: f32 = 0.0;
// =======================================================

#[derive(Component)]
struct Car {
    velocity: Vec2,
    is_skidding: bool,
    skid_timer: f32,
    car_id: usize,
}

#[derive(Component)]
struct TrackBoundary {
    size: Vec2,
}

#[derive(Component)]
struct StartingLine;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                title: "Car AI Training - Complex Track".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (car_physics, handle_boundary_collisions))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    for i in 0..CAR_COUNT {
        let offset = if CAR_COUNT > 1 {
            let spacing = 50.0;
            let start_idx = i as isize - (CAR_COUNT as isize / 2);
            Vec2::new(0.0, start_idx as f32 * spacing)
        } else {
            Vec2::ZERO
        };

        let color = if CAR_COUNT == 1 {
            Color::GREEN
        } else {
            Color::hsl((i as f32 / CAR_COUNT as f32) * 360.0, 1.0, 0.5)
        };

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::new(CAR_WIDTH, CAR_LENGTH)),
                    ..default()
                },
                transform: Transform::from_translation((START_POSITION + offset).extend(0.0))
                    .with_rotation(Quat::from_rotation_z(START_ROTATION)),
                ..default()
            },
            Car {
                velocity: Vec2::ZERO,
                is_skidding: false,
                skid_timer: 0.0,
                car_id: i,
            },
        ));
    }

    spawn_complex_track(&mut commands);
    spawn_starting_line(&mut commands);
}

fn spawn_complex_track(commands: &mut Commands) {
    // Create a complex stadium-like track with varied sections
    // This track has: straight sections, tight turns, wide turns, and chicanes
    
    let mut boundaries = Vec::new();
    
    // Track parameters - creates an interesting oval with complexity
    let track_width = 120.0; // Width of the drivable area
    
    // Define track centerline points (counter-clockwise)
    // This creates a complex shape with varied turns
    let track_points = vec![
        // Bottom straight
        Vec2::new(-400.0, -150.0),
        Vec2::new(-200.0, -150.0),
        // Right side - gentle curve up
        Vec2::new(0.0, -100.0),
        Vec2::new(150.0, 0.0),
        Vec2::new(200.0, 150.0),
        // Top straight with chicane
        Vec2::new(100.0, 250.0),
        Vec2::new(-100.0, 250.0),
        Vec2::new(-150.0, 200.0), // Dip inward
        Vec2::new(-200.0, 250.0), // Back out
        Vec2::new(-350.0, 200.0),
        // Left side curve down
        Vec2::new(-450.0, 100.0),
        Vec2::new(-450.0, -50.0),
        // Back to start
        Vec2::new(-400.0, -150.0),
    ];
    
    // Generate boundaries from track centerline
    for i in 0..track_points.len() {
        let p1 = track_points[i];
        let p2 = track_points[(i + 1) % track_points.len()];
        
        // Calculate segment direction and length
        let direction = (p2 - p1).normalize();
        let length = (p2 - p1).length();
        
        // Calculate perpendicular (for boundary offset)
        let perpendicular = Vec2::new(-direction.y, direction.x);
        
        // Calculate angle for rotation
        let angle = direction.y.atan2(direction.x);
        
        // Outer boundary
        let outer_pos = p1 + perpendicular * (track_width / 2.0 + BOUNDARY_THICKNESS / 2.0);
        boundaries.push((outer_pos, Vec2::new(length + 5.0, BOUNDARY_THICKNESS), angle));
        
        // Inner boundary
        let inner_pos = p1 - perpendicular * (track_width / 2.0 + BOUNDARY_THICKNESS / 2.0);
        boundaries.push((inner_pos, Vec2::new(length + 5.0, BOUNDARY_THICKNESS), angle));
    }
    
    // Spawn boundary segments
    for (pos, size, rotation) in boundaries {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::DARK_GRAY,
                    custom_size: Some(size),
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(0.0))
                    .with_rotation(Quat::from_rotation_z(rotation)),
                ..default()
            },
            TrackBoundary { size },
        ));
    }
}

fn spawn_starting_line(commands: &mut Commands) {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(100.0, 10.0)),
                ..default()
            },
            transform: Transform::from_translation(START_POSITION.extend(0.0)),
            ..default()
        },
        StartingLine,
    ));
}

fn car_physics(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Car)>,
) {
    for (mut transform, mut car) in query.iter_mut() {
        let dt = time.delta_seconds();

        let acceleration = 800.0;
        let max_speed = 600.0;
        let turn_speed = 3.0;
        let drag = 2.0;
        let lateral_drag = 5.0;
        let skid_angle_threshold = 0.5;
        let skid_speed_threshold = 150.0;
        let skid_duration = 0.5;
        let skid_traction_mult = 0.3;
        let skid_steering_mult = 0.4;

        let forward = (transform.rotation * Vec3::Y).truncate();
        let right = Vec2::new(-forward.y, forward.x);

        if keyboard.pressed(KeyCode::KeyW) {
            car.velocity += forward * acceleration * dt;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            car.velocity -= forward * acceleration * dt;
        }

        if car.velocity.length() > max_speed {
            car.velocity = car.velocity.normalize() * max_speed;
        }

        let steer_dir = if keyboard.pressed(KeyCode::KeyA) {
            1.0
        } else if keyboard.pressed(KeyCode::KeyD) {
            -1.0
        } else {
            0.0
        };

        let speed = car.velocity.length();

        let velocity_angle = if speed > 1.0 {
            car.velocity.angle_between(forward).abs()
        } else {
            0.0
        };

        let should_skid = speed > skid_speed_threshold
            && steer_dir != 0.0
            && (velocity_angle > skid_angle_threshold || car.is_skidding);

        if should_skid && !car.is_skidding {
            car.is_skidding = true;
            car.skid_timer = skid_duration;
        }

        if car.is_skidding {
            car.skid_timer -= dt;
            if car.skid_timer <= 0.0 {
                car.is_skidding = false;
            }
        }

        if speed > 5.0 && steer_dir != 0.0 {
            let rotation_amount = steer_dir * turn_speed * dt *
                if car.is_skidding { skid_steering_mult } else { 1.0 };

            transform.rotate_z(rotation_amount);

            if !car.is_skidding {
                let lateral_velocity = car.velocity.dot(right);
                car.velocity -= right * lateral_velocity * lateral_drag * dt;
            } else {
                let slip = right * steer_dir * 0.2 * speed * dt;
                car.velocity += slip;
            }
        }

        let current_velocity = car.velocity;
        let effective_drag = if car.is_skidding { drag * skid_traction_mult } else { drag };
        car.velocity = current_velocity - current_velocity * effective_drag * dt;

        transform.translation += (car.velocity * dt).extend(0.0);
    }
}

fn handle_boundary_collisions(
    mut params: ParamSet<(
        Query<(&mut Transform, &mut Car), With<Car>>,
        Query<(&Transform, &TrackBoundary), With<TrackBoundary>>,
    )>,
) {
    let boundary_data: Vec<(Transform, Vec2)> = params
        .p1()
        .iter()
        .map(|(t, b)| (*t, b.size))
        .collect();

    let mut car_query = params.p0();
    for (mut car_transform, mut car) in car_query.iter_mut() {
        let car_pos = car_transform.translation.truncate();

        for (boundary_transform, b_size) in &boundary_data {
            let b_pos = boundary_transform.translation.truncate();
            let half_b = *b_size / 2.0;

            let rotation = boundary_transform.rotation.to_euler(EulerRot::XYZ).2;
            let cos = rotation.cos();
            let sin = rotation.sin();

            let dx = car_pos.x - b_pos.x;
            let dy = car_pos.y - b_pos.y;
            let local_x = dx * cos + dy * sin;
            let local_y = -dx * sin + dy * cos;

            let closest_local_x = local_x.clamp(-half_b.x, half_b.x);
            let closest_local_y = local_y.clamp(-half_b.y, half_b.y);

            let closest_x = b_pos.x + closest_local_x * cos - closest_local_y * sin;
            let closest_y = b_pos.y + closest_local_x * sin + closest_local_y * cos;
            let closest_point = Vec2::new(closest_x, closest_y);

            let to_car = car_pos - closest_point;
            let distance = to_car.length();

            if distance < CAR_RADIUS {
                car_transform.translation = START_POSITION.extend(0.0);
                car_transform.rotation = Quat::from_rotation_z(START_ROTATION);
                car.velocity = Vec2::ZERO;
                car.is_skidding = false;
                car.skid_timer = 0.0;
                break;
            }
        }
    }
}