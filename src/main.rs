#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]

const BALL_SPEED: f32 = 2.0;
const BALL_SIZE: f32 = 6.0;
const BALL_SHAPE: Circle = Circle::new(BALL_SIZE);
const REGULAR_COLOR: Color = Color::srgb(1.0, 0., 0.);
const IMPULSE_DECAY_RATE: f32 = 50.0;

const PLAYER_COLOR: Color = Color::srgb(0.0, 0.5, 1.0);

const MOVE_TO_REACHED_DIST: f32 = 5.0;

const WANDER_DURATION: f32 = 1.5;
const SPAWN_DURATION:f32 = 0.1;

const WANDER_RANGE: RangeInclusive<f32> = -100.0..=100.0;
const SPAWN_RANGE: RangeInclusive<f32> = -50.0..=50.0;

use std::{ops::RangeInclusive, time::Duration};
use bevy::{ecs::{entity, relationship::*, system::SystemParam}, input::*, math::*, pbr::resources, prelude::*, sprite_render::*, ui::*};
use rand::prelude::*;

#[derive(PartialEq)]
enum MovementState {
	Normal,
	Knockbacked,
}

#[derive(Component)]
struct  MoveToPosition(Vec2);

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
struct WanderTimer(Timer);

#[derive(Component)]
#[require(
	Position,
	Velocity = Velocity(Vec2::ZERO),
	MoveDirection = MoveDirection(Vec2::ZERO),
	MoveSpeed = MoveSpeed(BALL_SPEED),
	MoveState = MoveState(MovementState::Normal),
	WanderTimer = WanderTimer(Timer::from_seconds(WANDER_DURATION, TimerMode::Repeating))
)]
struct Ball;

#[derive(Component)]
struct Player;

#[derive(Resource)]
struct SpawnTimer(Timer);

#[derive(Resource)]
struct BawlCount(u32);

#[derive(SystemParam)]
struct BallSpawnParams<'w, 's> {
	commands: Commands<'w, 's>,
	meshes: ResMut<'w, Assets<Mesh>>,
	materials: ResMut<'w, Assets<ColorMaterial>>,
}

#[derive(Component)]
struct Counter;

#[derive(Event)]
struct BawlSpawnEvent(Entity);

#[derive(EntityEvent)]
struct BawlTouchedEvent {
	entity: Entity,
	touched: Entity,
}

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
	position: Option<Vec2>,
	color: Color,
	shape: Circle,
	adjust_fc: F,
	mut ball_spawn: BallSpawnParams,
) -> Entity
	where F: FnOnce(&mut EntityCommands)
{
	println!("Spawning bawls...");
	let mut commands = ball_spawn.commands;
	let mesh = ball_spawn.meshes.add(shape);
	let material = ball_spawn.materials.add(color);
	
	let mut entity_cmds = commands.spawn((
		Ball,
		Mesh2d(mesh),
		MeshMaterial2d(material),
		Position(position.unwrap_or(Vec2::ZERO)),
	));
	let entity_id = entity_cmds.id();
	adjust_fc(&mut entity_cmds);
	drop(entity_cmds);
	
	commands.trigger(BawlSpawnEvent(entity_id));
	entity_id
}

fn spawn_player(
	ball_spawn: BallSpawnParams
) {
	spawn_bawl(
		Some(Vec2::ZERO),
		PLAYER_COLOR,
		BALL_SHAPE,
		|cmds| {
			cmds.insert(Player);
		},
		ball_spawn,
	);
}

fn spawn_regular_bawl(
	position: Vec2,
	ball_spawn: BallSpawnParams,
) {
	spawn_bawl(
		Some(position),
		REGULAR_COLOR,
		BALL_SHAPE,
		|_| { },
		ball_spawn,
	);
}

fn spawn_counter(mut commands: Commands,) {
	let container = Node {
		width: percent(100.0),
		height: percent(50.0),
		justify_content: JustifyContent::Center,
		..default()
	};
	
	let header = Node {
		width: px(200.0),
		height: px(100.0),
		..default()
	};
	
	let counter = (
		Counter,
		Text::new("0"),
		TextFont::from_font_size(50.0),
		TextColor(Color::WHITE),
		TextLayout::new_with_justify(Justify::Center),
		Node {
			position_type: PositionType::Absolute,
			..default()
		}
	);
	
	commands.spawn((
			container,
			children![(header, children![counter])]
		));
}

fn impulse(
	mut commands: Commands,
	entity: Entity,
	force: Vec2,
) {
	println!("impulsed");
	commands.entity(entity)
		.insert(Impulse(force))
		.entry::<MoveState>()
		.and_modify(|mut move_state| {
			let MoveState(ref mut state) = *move_state;
			if *state == MovementState::Normal {
				*state = MovementState::Knockbacked;
			}
		});
}

fn main() {
	let mut app = App::new();
	app.add_plugins(DefaultPlugins);
	app.add_systems(Startup, (
		spawn_camera,
		spawn_player,
		spawn_counter,
	));
	app.add_systems(FixedUpdate, (
		update_ui_count,
		bawl_npc_spawn,
		npc_wander,
		project_positions,
		handle_input,
		handle_move_to,
		handle_directional_move,
		handle_impulse,
		handle_move,
	).chain());
	app.insert_resource(SpawnTimer(Timer::from_seconds(SPAWN_DURATION, TimerMode::Repeating)));
	app.insert_resource(BawlCount(0));
	app.add_observer(count_bawl_spawned);
	
	app.run();
}

fn update_ui_count(
	mut counter: Single<&mut Text, With<Counter>>,
	count: Res<BawlCount>,
) {
	if count.is_changed() {
		counter.0 = count.0.to_string();
	}
}

fn count_bawl_spawned(
	event: On<BawlSpawnEvent>,
	mut b_count: ResMut<BawlCount>
) {
	b_count.0 += 1;
}
fn bawl_npc_spawn(
	time: Res<Time<Fixed>>,
	mut spawn_timer: ResMut<SpawnTimer>,
	ball_spawn: BallSpawnParams,
) {
	spawn_timer.0.tick(time.delta());
	if spawn_timer.0.just_finished() {
		let position = Vec2::new(
			rand::thread_rng().gen_range(SPAWN_RANGE),
			rand::thread_rng().gen_range(SPAWN_RANGE));
		spawn_regular_bawl(position, ball_spawn);
	}
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

fn move_to(
	mut commands: &mut Commands,
	entity: Entity,
	goal: Vec2
) {
	commands.entity(entity)
		.entry::<MoveToPosition>()
		.and_modify(move |mut move_to| move_to.0 = goal)
		.or_insert(MoveToPosition(goal));
}

fn npc_wander(
	mut commands: Commands,
	time: Res<Time<Fixed>>,
	to_move: Query<
		(Entity, &Position, &mut WanderTimer),
		(With<Ball>, Without<Player>, Without<MoveToPosition>)
	>,
) {
	for (entity, current_position, mut wander_timer) in to_move {
		wander_timer.0.tick(time.delta());
		if wander_timer.0.just_finished() {
			let offset = Vec2::new(
				rand::thread_rng().gen_range(WANDER_RANGE),
				rand::thread_rng().gen_range(WANDER_RANGE));
			
			let goal = current_position.0 + offset;
			move_to(&mut commands, entity, goal);
		}
	}
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
	mut impulsed: Query<(Entity, &mut Velocity, &mut Impulse, &mut MoveState)>,
	time: Res<Time<Fixed>>,
) {
	let delta = time.delta_secs();
	for (entity, mut velocity, mut impulse, mut move_state) in impulsed {
		velocity.0 = impulse.0;
		
		let magnitude = impulse.0.length();
		let decay = IMPULSE_DECAY_RATE * delta;
		
		if magnitude <= decay { // remove if its less than decay meaning its near ZERO
			let mut cmd = commands.entity(entity);
			cmd.remove::<Impulse>();
			
			let MoveState(ref mut state) = *move_state;
			if *state == MovementState::Knockbacked {
				*state = MovementState::Normal;
			}
		} else {
			impulse.0 = impulse.0.normalize() * (magnitude - decay)
		}
	}
}

fn handle_move_to( // for npcs bawls
	mut commands: Commands,
	moving_w_move_to: Query<
		(Entity, &Position, &mut MoveDirection, &MoveToPosition),
		Without<Player> // incompatible with player for now
	>,
) {
	for (entity, position, mut move_direction, move_to_pos) in moving_w_move_to {
		let resultant = move_to_pos.0 - position.0;
		let distance = resultant.length();
		
		if distance <= MOVE_TO_REACHED_DIST {
			commands.entity(entity).remove::<MoveToPosition>();
			move_direction.0 = Vec2::ZERO;
		} else {
			let direction = resultant.normalize();
			move_direction.0 = direction;
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
) {
	for (mut position, velocity) in moveables {
		position.0 += velocity.0
	}
}
