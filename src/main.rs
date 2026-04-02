use bevy::prelude::*;
use rusqlite::{Connection, params};

// ==================== CONFIGURATION ====================
const CAR_COUNT: usize = 1;
const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 720.0;

const BOUNDARY_THICKNESS: f32 = 12.0;
const CAR_WIDTH: f32 = 30.0;
const CAR_LENGTH: f32 = 60.0;
const CAR_RADIUS: f32 = 25.0;

const START_POSITION: Vec2 = Vec2::new(-440.0, 0.0);
const START_ROTATION: f32 = 0.0;
// =======================================================

#[derive(Component)]
struct Checkpoint {
    id: usize,
}

struct Db {
    conn: Connection,
}

#[derive(Resource)]
struct RewardState {
    last_checkpoint: Vec<usize>,
}

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
    let conn = Connection::open_in_memory().unwrap();

    conn.execute(
        "CREATE TABLE ai_state (
            car_id INTEGER,
            vel_x REAL,
            vel_y REAL,
            forward_x REAL,
            forward_y REAL,
            reward REAL
        )",
        [],
    ).unwrap();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                title: "Car AI Training - Complex Track".to_string(),
                ..default()
            }),
            ..default()
        }))
        .insert_non_send_resource(Db { conn })
        .insert_resource(RewardState {
            last_checkpoint: vec![0; CAR_COUNT],
        })
        .add_systems(Update, (
            car_physics,
            handle_boundary_collisions,
            reward_system,
            log_ai_state,
        ))
        .add_systems(Startup, setup)
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

        

        let _car_entity = commands.spawn((
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
        )        ).id();
    }

    spawn_complex_track(&mut commands);
    spawn_starting_line(&mut commands);
    spawn_checkpoint(&mut commands, Vec2::new(0.0, -215.0), 0);
    spawn_checkpoint(&mut commands, Vec2::new(0.0, 230.0), 1);
}

fn spawn_checkpoint(commands: &mut Commands, pos: Vec2, id: usize) {
    commands.spawn((
        Checkpoint { id },
        SpriteBundle {
            sprite: Sprite {
                color: Color::BLUE,
                custom_size: Some(Vec2::new(10.0, 100.0)),
                ..default()
            },
            transform: Transform::from_translation(pos.extend(0.0)),
            ..default()
        }
    ));
}

fn spawn_boundary_loop(commands: &mut Commands, points: &[Vec2], color: Color) {
    let count = points.len();

    for i in 0..count {
        let p1 = points[i];
        let p2 = points[(i + 1) % count];

        let segment = p2 - p1;
        let length = segment.length();
        let midpoint = (p1 + p2) * 0.5;
        let angle = segment.y.atan2(segment.x);

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::new(length, BOUNDARY_THICKNESS)),
                    ..default()
                },
                transform: Transform::from_translation(midpoint.extend(0.0))
                    .with_rotation(Quat::from_rotation_z(angle)),
                ..default()
            },
            TrackBoundary {
                size: Vec2::new(length, BOUNDARY_THICKNESS),
            },
        ));
    }
}

fn spawn_complex_track(commands: &mut Commands) {
    let track_width = 100.0;

    let centerline = vec![
        Vec2::new(-300.0, -200.0),
        Vec2::new(-50.0, -220.0),
        Vec2::new(200.0, -200.0),
        Vec2::new(340.0, -166.0),
        Vec2::new(425.0, -67.0),
        Vec2::new(450.0, 0.0),
        Vec2::new(415.0, 50.0),
        Vec2::new(280.0, 170.0),
        Vec2::new(200.0, 200.0),
        Vec2::new(-100.0, 250.0),
        Vec2::new(-120.0, 245.0),
        Vec2::new(-170.0, 222.0),
        Vec2::new(-300.0, 250.0),
        Vec2::new(-400.0, 200.0),
        Vec2::new(-450.0, 0.0),
    ];

    let mut inner_points = Vec::new();
    let mut outer_points = Vec::new();

    let count = centerline.len();

    for i in 0..count {
        let prev = centerline[(i + count - 1) % count];
        let curr = centerline[i];
        let next = centerline[(i + 1) % count];

        let dir1 = (curr - prev).normalize();
        let dir2 = (next - curr).normalize();

        let normal1 = Vec2::new(-dir1.y, dir1.x);
        let normal2 = Vec2::new(-dir2.y, dir2.x);

        // Average normals for mitered corner
        let mut miter = (normal1 + normal2).normalize();

        // Prevent extreme stretching on sharp corners
        if miter.length_squared() < 0.001 {
            miter = normal1;
        }

        let scale = track_width / 2.0 / miter.dot(normal1);

        outer_points.push(curr + miter * scale);
        inner_points.push(curr - miter * scale);
    }

    spawn_boundary_loop(commands, &outer_points, Color::DARK_GRAY);
    spawn_boundary_loop(commands, &inner_points, Color::GRAY);
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
            0.01 * ((car.velocity.x).powf(2.0) + (car.velocity.y).powf(2.0)).sqrt()
        } else if keyboard.pressed(KeyCode::KeyD) {
            -0.01 * ((car.velocity.x).powf(2.0) + (car.velocity.y).powf(2.0)).sqrt()
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
                let penalty = -car.velocity.length() * 0.2;
                println!("Penalty {}", penalty);
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

fn reward_system(
    mut reward_state: ResMut<RewardState>,
    mut car_query: Query<(&Transform, &mut Car)>,
    checkpoint_query: Query<(&Transform, &Checkpoint)>,
) {
    for (car_transform, car) in car_query.iter_mut() {
        let car_pos = car_transform.translation.truncate();
        let _speed = car.velocity.length();

        for (cp_transform, cp) in checkpoint_query.iter() {
            if car_pos.distance(cp_transform.translation.truncate()) < 60.0 {
                if reward_state.last_checkpoint[car.car_id] != cp.id {
                    reward_state.last_checkpoint[car.car_id] = cp.id;

                    println!("Reward +100");
                }
            }
        }
    }
}

fn log_ai_state(
    db: NonSend<Db>,
    car_query: Query<(&Transform, &Car)>,
) {
    for (transform, car) in car_query.iter() {
        let forward = (transform.rotation * Vec3::Y).truncate();

        db.conn.execute(
            "INSERT INTO ai_state
            (car_id, vel_x, vel_y, forward_x, forward_y, reward)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                car.car_id as i32,
                car.velocity.x,
                car.velocity.y,
                forward.x,
                forward.y,
                0.0
            ],
        ).unwrap();
    }
}