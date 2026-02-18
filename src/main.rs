use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, car_physics)
        .run();
}

#[derive(Component)]
struct Car {
    velocity: Vec2,
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::GREEN,
                custom_size: Some(Vec2::new(30.0, 60.0)),
                ..default()
            },
            ..default()
        },
        Car {
            velocity: Vec2::ZERO,
        },
    ));
}

fn car_physics(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Car)>,
) {
    let (mut transform, mut car) = query.single_mut();
    let dt = time.delta_seconds();

    // --- Tunable constants ---
    let acceleration = 800.0;
    let max_speed = 600.0;
    let turn_speed = 3.0; // radians/sec
    let drag = 2.0;

    // Forward direction from rotation
    let forward = transform.rotation * Vec3::Y;
    let forward2 = Vec2::new(forward.x, forward.y);

    // Throttle
    if keyboard.pressed(KeyCode::KeyW) {
        car.velocity += forward2 * acceleration * dt;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        car.velocity -= forward2 * acceleration * dt;
    }

    // Clamp speed
    if car.velocity.length() > max_speed {
        car.velocity = car.velocity.normalize() * max_speed;
    }

    // Steering (only if moving)
    let speed = car.velocity.length();
    if speed > 5.0 {
        let steer_dir = if keyboard.pressed(KeyCode::KeyA) {
            1.0
        } else if keyboard.pressed(KeyCode::KeyD) {
            -1.0
        } else {
            0.0
        };

        let rotation_amount = steer_dir * turn_speed * dt;
        transform.rotate_z(rotation_amount);

        // Rotate velocity vector too (this is the magic part)
        car.velocity = car.velocity.rotate(Vec2::from_angle(rotation_amount));
    }

    // Drag / friction (inertia)
    let v = car.velocity;
    car.velocity -= v*drag*dt;

    // Integrate position
    transform.translation += (car.velocity * dt).extend(0.0);
}
