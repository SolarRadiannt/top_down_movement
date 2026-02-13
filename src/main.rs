#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]

use bevy::{ecs::relationship::RelationshipSourceCollection, input::*, math::VectorSpace, prelude::*, sprite_render::Material2d, ui::Pressed};

const BALL_SPEED: f32 = 5.0;
const BALL_SIZE: f32 = 6.0;
const BALL_SHAPE: Circle = Circle::new(BALL_SIZE);
const BALL_COLOR: Color = Color::srgb(1.0, 0., 0.);
const IMPULSE_DECAY_RATE: f32 = 2.5;

const PLAYER_COLOR: Color = Color::srgb(0.0, 0.5, 1.0);

enum MovementState {
	Normal,
	Knockbacked,
}

#[derive(Component, Default)]
#[require(Transform)]
struct Position(Vec2);

#[derive(Component, Default)]
struct Velocity(Vec2);

#[derive(Component)]
struct MoveSpeed(f32);

#[derive(Component, Default)]
struct MoveDirection(Vec2);

#[derive(Component)]
struct Impulse(Vec2);

#[derive(Component)]
struct MoveState(MovementState);

#[derive(Component)]
#[require(
	Position,
	Velocity = Velocity(Vec2::ZERO),
	MoveDirection = MoveDirection(Vec2::ZERO),
	MoveSpeed = MoveSpeed(BALL_SPEED),
	MoveState = MoveState(MovementState::Normal),
)]
struct Ball;

#[derive(Component)]
struct Player;




fn spawn_camera(
	mut commands: Commands,
) {
	// commands.spawn((
	// 	Camera2d,
	// 	Transform::from_xyz(0., 0., 0.)
	// ));
	commands.spawn(Camera2d);
}

fn spawn_bawl<F>(
	commands: &mut Commands,
	mesh: Handle<Mesh>,
	material: Handle<ColorMaterial>,
	adjust_fc: F,
	position: Option<Vec2>,
) -> Entity
	where F: FnOnce(&mut EntityCommands)
{
	println!("Spawning bawls...");
	
	let mut entity_cmds = commands.spawn((
		Ball,
		Mesh2d(mesh),
		MeshMaterial2d(material),
		Position(position.unwrap_or(Vec2::ZERO)),
	));
	adjust_fc(&mut entity_cmds);
	entity_cmds.id()
}

fn spawn_player(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<ColorMaterial>>,
) {
	let mesh = meshes.add(BALL_SHAPE);
	let material = materials.add(PLAYER_COLOR);
	
	spawn_bawl(
		&mut commands,
		mesh,
		material,
		|cmds| {
			cmds.insert(Player);
		},
		Some(Vec2::ZERO),
	);
}

fn spawn_regular_bawl(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<ColorMaterial>>,
) {
	let mesh = meshes.add(BALL_SHAPE);
	let material = materials.add(BALL_COLOR);
	spawn_bawl(
		&mut commands,
		mesh,
		material,
		|_| { },
		Some(Vec2::ZERO),
	);
}

fn impulse(
	mut commands: Commands,
	entity: Entity,
	force: Vec2,
) {
	commands.entity(entity).insert(Impulse(force));
}

const _IMPULSE_INTERVAL: f32 = 1.0;
fn _impulse_experiment(
	
) {
	
}

fn main() {
	let mut app = App::new();
	app.add_plugins(DefaultPlugins);
	app.add_systems(Startup, (
		spawn_camera,
		// spawn_regular_bawl,
		spawn_player,
	));
	app.add_systems(FixedUpdate, (
		handle_input,
		handle_impulse.before(handle_move),
		handle_directional_move,
		handle_move.after(handle_directional_move),
		project_positions.after(handle_move),
	));
	app.run();
}



fn handle_input(
	input: Res<ButtonInput<KeyCode>>,
	mut player: Single<(&mut MoveDirection, &MoveState), With<Player>>
) {
	let (mut move_dir, move_state) = player.into_inner();
	let dir = match move_state.0 {
		MovementState::Normal => {
			[
				(KeyCode::KeyW, Vec2::Y),
				(KeyCode::KeyA, Vec2::NEG_X),
				(KeyCode::KeyS, Vec2::NEG_Y),
				(KeyCode::KeyD, Vec2::X),
			]
			.iter()
			.filter_map(|(key, direction)| input.pressed(*key).then_some(*direction))
			.sum::<Vec2>()
		}
		_ => {
			Vec2::ZERO
		}
	};
	
	move_dir.0 = dir.normalize_or_zero();
}

fn project_positions(
	mut positionables: Query<(&mut Transform, &Position)>
) {
	for (mut transform, position) in &mut positionables {
		transform.translation = position.0.extend(0.0);
	}
}

fn handle_impulse(
	mut commands: Commands,
	mut impulsed: Query<(Entity, &mut Velocity, &mut Impulse)>,
	time: Res<Time<Fixed>>,
) {
	let delta = time.delta_secs();
	for (entity, mut velocity, mut impulse) in impulsed {
		velocity.0 += impulse.0;
		
		let magnitude = impulse.0.length();
		let decay = IMPULSE_DECAY_RATE * delta;
		
		if magnitude <= decay { // remove if its less than decay meaning its near ZERO
			commands.entity(entity).remove::<Impulse>();
		} else {
			impulse.0 = impulse.0.normalize() * (magnitude - decay)
		}
	}
}

fn handle_directional_move(
	mut moveables: Query<(
		&mut Velocity, &MoveDirection, &MoveSpeed, &MoveState
	), Without<Impulse>> // directional move is not for entities impulsed
) {
	for (mut velocity, direction, speed, move_state) in &mut moveables {
		velocity.0 = direction.0 * speed.0;
	}
}

fn handle_move(
	moveables: Query<(
		&mut Position, &Velocity
	)>
){
	for (mut position, velocity) in moveables {
		position.0 += velocity.0
	}
}
