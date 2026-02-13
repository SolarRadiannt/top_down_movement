#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]

use bevy::{input::*, math::VectorSpace, prelude::*, sprite_render::Material2d, ui::Pressed};

const BALL_SPEED: f32 = 5.0;
const BALL_SIZE: f32 = 6.0;
const BALL_SHAPE: Circle = Circle::new(BALL_SIZE);
const BALL_COLOR: Color = Color::srgb(1.0, 0., 0.);
const IMPULSE_DECAY_RATE: f32 = 2.5;

const PLAYER_COLOR: Color = Color::srgb(0.0, 1.0, 0.0);

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
#[require(
	Position,
	Velocity = Velocity(Vec2::ZERO),
	MoveDirection = MoveDirection(Vec2::ZERO),
	MoveSpeed = MoveSpeed(BALL_SPEED),
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

fn main() {
	let mut app = App::new();
	app.add_plugins(DefaultPlugins);
	app.add_systems(Startup, (
		spawn_camera,
		// spawn_regular_bawl,
		spawn_player,
	));
	app.add_systems(FixedUpdate, (
		project_positions.after(handle_move),
		handle_input,
		handle_move_velocity,
		handle_move.after(handle_move_velocity),
	));
	app.run();
}



fn handle_input(
	input: Res<ButtonInput<KeyCode>>,
	mut move_dir: Single<&mut MoveDirection, With<Player>>
) {
	let mut dir = [
		(KeyCode::KeyW, Vec2::Y),
		(KeyCode::KeyA, Vec2::NEG_X),
		(KeyCode::KeyS, Vec2::NEG_Y),
		(KeyCode::KeyD, Vec2::X),
	]
	.iter()
	.filter_map(|(key, direction)| input.pressed(*key).then_some(*direction))
	.sum::<Vec2>();
	
	move_dir.0 = dir.normalize_or_zero();
}

fn project_positions(
	mut positionables: Query<(&mut Transform, &Position)>
) {
	for (mut transform, position) in &mut positionables {
		transform.translation = position.0.extend(0.0);
	}
}

fn handle_impulse( // soon after the player movement is polished
	mut impulsed: Query<(&mut Velocity, &mut Impulse)>
) {
	
}

fn handle_move_velocity(
	mut moveables: Query<(
		&mut Velocity, &MoveDirection, &MoveSpeed
	)>
) {
	for (mut velocity, direction, speed) in &mut moveables {
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
