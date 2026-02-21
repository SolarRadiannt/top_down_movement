#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]

const fn get_rect_corners(rect: Rect) -> [Vec2; 4] {
	[
		rect.min,							// bottom‑left
		Vec2::new(rect.min.x, rect.max.y),	// top‑left
		rect.max,							// top‑right
		Vec2::new(rect.max.x, rect.min.y),	// bottom‑right
	]
}
const fn rect_from_center_size(center: Vec2, size: Vec2) -> Rect {
	let half_size = Vec2::new(size.x / 2., size.y / 2.);
	Rect {
		min: Vec2::new(center.x - half_size.x, center.y - half_size.y),
		max: Vec2::new(center.x + half_size.x, center.y + half_size.y)
	}
}
const WORLD_SIZE: f32 = 10_000.0;
const WORLD_CENTER: Vec2 = Vec2::ZERO;
const WORLD_BOUNDARY: Rect = rect_from_center_size(WORLD_CENTER, Vec2::splat(WORLD_SIZE));

const WORLD_CORNERS: [Vec2; 4] = get_rect_corners(WORLD_BOUNDARY);
const BOUNDARY_COLOR: Color = Color::srgb(1.0, 0.0, 0.0);
const MAX_QUAD_DEPTH: i32 = 10;

const PLAYER_SPEED: f32 = 3.5;
const BALL_SPEED: f32 = 3.0;

const BALL_SIZE: f32 = 5.0;
const BALL_SHAPE: Circle = Circle::new(BALL_SIZE);
const IMPULSE_DECAY_RATE: f32 = 20.0;

const REGULAR_COLOR: Color = Color::srgb(0.7, 0.0, 0.0);
const SHOVER_COLOR: Color = Color::srgb(0.8, 0.0, 0.9);
const PLAYER_COLOR: Color = Color::srgb(0.0, 0.5, 1.0);

const MOVE_TO_REACHED_DIST: f32 = 3.5;

const WANDER_DURATION: f32 = 1.5;
const SPAWN_DURATION:f32 = 0.01;
const SHOVE_POWER: f32 = 10.0;

const WANDER_RANGE: RangeInclusive<f32> = -300.0..=300.0;
const SPAWN_RANGE: RangeInclusive<f32> = -1_000.0..=1_000.0;

use std::{any::Any, ops::RangeInclusive, str::Lines, time::Duration};
use bevy::{asset::uuid::timestamp::context, audio::Volume, ecs::{entity, relationship::*, system::SystemParam}, input::{mouse::MouseButtonInput, *}, math::{bounding::{BoundingCircle, BoundingVolume, IntersectsVolume}, *}, prelude::*, render::mesh::MeshRenderAssetPlugin, sprite_render::Material2d, transform::commands, ui_render::shader_flags::CORNERS, window::PrimaryWindow};
use bevy_pancam::*;
use rand::prelude::*;
use std::collections::HashMap;
use bevy_cursor::prelude::*;

/*
	things:
	ADD QUADTREE AND REPLACE THE NAIVE BOUNDING DETECTION!!!!
 */

// ---[ enums ]---

#[derive(PartialEq)]
enum MovementState {
	Normal,
	Knockbacked,
}

// ---[ components ]---

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
struct Boundary(Rect);

#[derive(Component)]
#[require(
	Position,
	Velocity = Velocity(Vec2::ZERO),
	MoveDirection = MoveDirection(Vec2::ZERO),
	MoveSpeed = MoveSpeed(BALL_SPEED),
	MoveState = MoveState(MovementState::Normal),
	WanderTimer = WanderTimer(Timer::from_seconds(WANDER_DURATION, TimerMode::Repeating)),
)]
struct Ball(Circle);

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Shover;

#[derive(Component)]
struct Counter;

#[derive(Component)]
struct RectToDraw([Vec2; 4], Color);

// ---[ resources ]---

#[derive(Resource)]
struct SpawnTimer(Timer);

#[derive(Resource)]
struct BawlCount(u32);


fn rect_intersect_circle(rect: Rect, circle: BoundingCircle) {
	
}

const QT_MAX_GEN: u32 = 5;
struct QuadTree {
	boundary: Rect,
	capacity: u32,
	points: Vec<(Rect, Entity)>,
	corners: [Vec2; 4],
	
	divided: bool,
	children: Option<[Box<QuadTree>; 4]>,
	generation: u32,
}
impl QuadTree {
	fn new(boundary: Rect, capacity: u32, generation: Option<u32>) -> Self {
		let next_gen = generation.unwrap_or(0) + 1;
		Self {
			boundary: boundary,
			capacity: capacity,
			points: Vec::default(),
			corners: get_rect_corners(boundary),
			children: None,
			divided: false,
			generation: next_gen,
		}
	}
	
	fn subdivide(&mut self) {
		// println!("qt subdivided");
		self.divided = true;
		if let Some(c) = &self.children { return };
		
		let boundary = self.boundary;
		let center = boundary.center();
		let root_size = boundary.half_size() / 2.0;
		let capacity = self.capacity;
		
		let x = root_size.x;
		let y = root_size.y;
		
		let ne = Rect::from_center_half_size(center + Vec2::new(x, y), root_size);
		let nw = Rect::from_center_half_size(center - Vec2::new(x, -y), root_size);
		let se = Rect::from_center_half_size(center + Vec2::new(x, -y), root_size);
		let sw = Rect::from_center_half_size(center - Vec2::new(x, y), root_size);
		
		let mut qt_childs = [
			Box::new(QuadTree::new(ne, self.capacity, Some(self.generation))),
			Box::new(QuadTree::new(nw, self.capacity, Some(self.generation))),
			Box::new(QuadTree::new(se, self.capacity, Some(self.generation))),
			Box::new(QuadTree::new(sw, self.capacity, Some(self.generation))),
		];
		
		for point in self.points.drain(..) {
			for child in &mut qt_childs {
				if child.insert(point) {
					break;
				}
			}
		}
		
		self.children = Some(qt_childs);
	}
	
	fn remove(&mut self, entity: Entity) -> bool {
		let mut removed = false;
		self.points.retain(|point| {
			if point.1 == entity {
				removed = true;
				false // Remove this element
			} else {
				true // Keep this element
			}
		});
		
		if removed {
			if self.is_subtree_empty() {
				self.un_subdivide();
			}
			true
		} else {
			let Some(children) = &mut self.children else { return false };
			
			if self.divided {
				for mut child in children {
					if child.remove(entity) {
						if self.is_subtree_empty() {
							self.un_subdivide();
						}
						return true
					};
				};
			}
			
			if self.is_subtree_empty() {
				self.un_subdivide();
			}
			
			false
		}
	}
	
	fn is_subtree_empty(&self) -> bool {
		// if !self.points.is_empty() {
		// 	return false;
		// }
		// if let Some(children) = &self.children {
		// 	for child in children {
		// 		if !child.is_subtree_empty() {
		// 			return false;
		// 		}
		// 	}
		// };
		// true
		false
	}
	
	fn un_subdivide(&mut self) {
		if self.divided {
			self.divided = false;
		}
	}
	
	fn insert(&mut self, point: (Rect, Entity)) -> bool {
		if self.boundary.intersect(point.0).is_empty() { return false };
		// print!("{} point inserted, current len: {}, capacity: {} ", point, self.points.len(), self.capacity);
		if self.generation >= QT_MAX_GEN || self.points.len() < self.capacity as usize {
			self.points.push(point);
			true
		} else {
			if !self.divided { self.subdivide() };
			if let Some(children) = &mut self.children {
				for mut child in children {
					if child.insert(point) {
						return true
					}
				};
			};
			
			false
		}
	}
	
	fn query_rect(&self, rect: Rect) -> Vec<&(Rect, Entity)> { // range/box
		let mut found = Vec::new();
		
		if self.boundary.intersect(rect).is_empty() {
			// println!("is not overlapping bounds");
			found
		} else {
			// println!("is overlapping, filling the found vec");
			for point in self.points.iter() {
				if !rect.intersect(point.0).is_empty() {
					found.push(point);
				}
			}
			if self.divided {
				// println!("is divided, checking children...");
				if let Some(children) = &self.children {
					for child in children {
						found.extend(child.query_rect(rect));
					}
				}
			}
			found
		}
	}
	
	fn draw(&self, gizmos: &mut Gizmos) {
		draw_rect(gizmos, self.corners, Color::WHITE);
		if self.divided && let Some(children) = &self.children {
			for child in children {
				child.draw(gizmos);
			};
		}
	}
}

#[derive(Resource)]
struct SolQT(QuadTree);

// ---[ system params ]---

#[derive(SystemParam)]
struct BallSpawnParams<'w> {
	meshes: ResMut<'w, Assets<Mesh>>,
	materials: ResMut<'w, Assets<ColorMaterial>>,
}

// ---[ Events ]---

#[derive(Event)]
struct BawlSpawnEvent(Entity);

#[derive(Event)]
struct BawlRemovedEvent(Entity);

#[derive(EntityEvent)]
struct EntityMoved {
	entity: Entity,
	old_pos: Vec2,
	new_pos: Vec2,
}

// --- setup fcs ---

fn spawn_camera(
	mut commands: Commands,
) {
	// commands.spawn((
	// 	Camera2d,
	// 	Transform::from_xyz(0., 0., 0.)
	// ));
	commands.spawn((
		Camera2d,
		PanCam {
			grab_buttons: vec![MouseButton::Middle, MouseButton::Right],
			move_keys: DirectionKeys {      // the keyboard buttons used to move the camera
				up:    vec![KeyCode::ArrowUp], // initalize the struct like this or use the provided methods for
				down:  vec![KeyCode::ArrowDown], // common key combinations
				left:  vec![KeyCode::ArrowLeft],
				right: vec![KeyCode::ArrowRight],
			},
    		..default()
		},
	));
}

fn spawn_bawl<F>(
	position: Option<Vec2>,
	color: Color,
	shape: Circle,
	adjust_fc: F,
	ball_spawn: &mut BallSpawnParams,
	commands: &mut Commands,
) -> Entity
	where F: FnOnce(&mut EntityCommands)
{
	// println!("Spawning bawls...");
	let mesh = ball_spawn.meshes.add(shape);
	let material = ball_spawn.materials.add(color);
	let origin = position.unwrap_or(Vec2::ZERO);
	
	let mut entity_cmds = commands.spawn((
		Ball(shape),
		Mesh2d(mesh),
		MeshMaterial2d(material),
		Position(origin),
		Boundary(Rect::from_center_size(origin, Vec2::splat(shape.radius)))
	));
	let entity_id = entity_cmds.id();
	adjust_fc(&mut entity_cmds);
	drop(entity_cmds);
	
	commands.trigger(BawlSpawnEvent(entity_id));
	// println!("bawl spawned");
	entity_id
}

fn spawn_player(
	mut ball_spawn: BallSpawnParams,
	mut commands: Commands,
) {
	spawn_bawl(
		None,
		PLAYER_COLOR,
		BALL_SHAPE,
		|cmds| {
			cmds.insert(Player)
				.entry::<MoveSpeed>()
				.and_modify(|mut speed| speed.0 = PLAYER_SPEED);
		},
		&mut ball_spawn,
		&mut commands,
	);
}

fn spawn_shove_bawl(
	position: Vec2,
	ball_spawn: &mut BallSpawnParams,
	commands: &mut Commands,
) {
	spawn_bawl(
		Some(position),
		SHOVER_COLOR,
		BALL_SHAPE,
		|cmds| { cmds.insert(Shover); },
		ball_spawn,
		commands,
	);
}

fn spawn_regular_bawl(
	position: Vec2,
	ball_spawn: &mut BallSpawnParams,
	commands: &mut Commands,
) {
	spawn_bawl(
		Some(position),
		REGULAR_COLOR,
		BALL_SHAPE,
		|_| { },
		ball_spawn,
		commands,
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

// ---[ helpers ]---

fn impulse(
	mut entity_cmd: EntityCommands,
	force: Vec2,
) {
	info!("impulsed with the force {}", force);
	entity_cmd
		.insert(Impulse(force))
		.entry::<MoveState>()
		.and_modify(|mut move_state| {
			let MoveState(ref mut state) = *move_state;
			if *state == MovementState::Normal {
				*state = MovementState::Knockbacked;
			}
		});
}

fn move_to(
	commands: &mut Commands,
	entity: Entity,
	goal: Vec2
) {
	commands.entity(entity)
		.entry::<MoveToPosition>()
		.and_modify(move |mut move_to| move_to.0 = goal)
		.or_insert(MoveToPosition(goal));
}

fn draw_rect(
	gizmos: &mut Gizmos,
	corners: [Vec2; 4],
	color: Color,
) {
	for i in 0..4 {
		let start = corners[i];
		let end = corners[(i + 1) % 4];
		gizmos.line_2d(start, end, color);
	}
}

// vvv ==> [ MAIN ] <== vvv
fn main() {
	let mut app = App::new();
	app.add_plugins((
		DefaultPlugins,
		PanCamPlugin,
		TrackCursorPlugin,
	));
	
	app.add_systems(Startup, (
		//draw_world_boundary
		spawn_camera,
		spawn_player,
		spawn_counter,
		// qt_testing,
		// check_boundary_qt.after(qt_testing)
	));
	
	app.add_systems(FixedUpdate, (
		draw_qt,
		update_ui_count,
		handle_drawing_for_rects,
		
		handle_npc_spawn,
		handle_npc_wander,
		handle_position_projection,
		handle_input,
		handle_move_to,
		// handle_shoving,
		handle_impulse,
		handle_velocity,
		handle_move,
	).chain());
	app.insert_resource(SpawnTimer(Timer::from_seconds(SPAWN_DURATION, TimerMode::Repeating)));
	app.insert_resource(BawlCount(0));
	app.insert_resource(SolQT(QuadTree::new(WORLD_BOUNDARY, 40, None)));
	app.add_observer(count_bawl_spawned);
	app.add_observer(count_bawl_removed);
	app.add_observer(handle_world_boundary);
	app.add_observer(insert_new_point_to_qt);
	app.add_observer(update_boundary);
	app.run();
	
	
}
// ^^^ ==> [ MAIN ] <== ^^^

// ---[ systems ]---
const AMOUNT: u32 = 500;
fn qt_testing(
	mut ball_spawn: BallSpawnParams,
	mut commands: Commands,
	qt: Res<SolQT>
) {
	for i in 0..AMOUNT {
		let position = Vec2::new(
			rand::thread_rng().gen_range(SPAWN_RANGE),
			rand::thread_rng().gen_range(SPAWN_RANGE));
		spawn_regular_bawl(position, &mut ball_spawn, &mut commands);
	};
}

fn random_vector(x_range: RangeInclusive<f32>, y_range: RangeInclusive<f32>) -> Vec2 {
	Vec2::new(
		rand::thread_rng().gen_range(x_range),
		rand::thread_rng().gen_range(y_range),
	)
}
#[derive(Component)]
struct InBounds;
fn check_boundary_qt(
	mut commands: Commands,
	qt: Res<SolQT>,
	mut ball_spawn: BallSpawnParams,
) {
	let check_rect = Rect::from_center_size(random_vector(-2_00.0..=2_00.0, -2_00.0..=2_00.0), Vec2::splat(500.0));
	let found = qt.0.query_rect(check_rect);
	commands.spawn(RectToDraw(get_rect_corners(check_rect), Color::srgb(0., 1.0, 0.)));
	println!("found withind check: {}", found.len());
	
	for point in found {
		commands.entity(point.1).insert(InBounds);
	}
}

fn draw_qt(
	mut gizmos: Gizmos,
	qt: Res<SolQT>,
) {
	qt.0.draw(&mut gizmos);
}

fn draw_world_boundary(
	mut commands: Commands
) {
	commands.spawn(RectToDraw(WORLD_CORNERS, BOUNDARY_COLOR));
}

fn handle_drawing_for_rects(
	mut gizmos: Gizmos,
	rects: Query<&RectToDraw>,
) {
	for to_draw in rects {
		draw_rect(&mut gizmos, to_draw.0, to_draw.1);
	}
}

fn handle_world_boundary(
	event: On<EntityMoved>,
	mut commands: Commands,
) {
	let went_out = !WORLD_BOUNDARY.contains(event.new_pos);
	if went_out {
		let e_id = commands.entity(event.entity).id();
		
		commands.trigger(BawlRemovedEvent(e_id));
		commands.entity(event.entity).despawn();
		println!("entity despawned")
	}
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
	mut b_count: ResMut<BawlCount>,
) {
	b_count.0 += 1;
}

fn insert_new_point_to_qt(
	event: On<BawlSpawnEvent>,
	mut commands: Commands,
	positions: Query<(&Boundary, &Ball)>,
	mut qt: ResMut<SolQT>,
) {
	let e = event.0;
	if let Ok((boundary, ball)) = positions.get(e) {
		qt.0.insert((boundary.0, e));
	}
}

// fn update_qt(
// 	event: On<EntityMoved>,
// 	mut qt: ResMut<SolQT>,
// ) {
// 	qt.0.remove(event.entity);
// 	qt.0.insert((event.new_pos, event.entity));
// }

fn count_bawl_removed(
	event: On<BawlRemovedEvent>,
	mut b_count: ResMut<BawlCount>,
) {
	b_count.0 -= 1;
}

fn update_boundary(
	event: On<EntityMoved>,
	mut to_update: Query<&mut Boundary>,
	mut qt: ResMut<SolQT>
) {
	if let Ok(mut boundary) = to_update.get_mut(event.entity) {
		boundary.0.min = event.new_pos;
		boundary.0.max = event.new_pos + boundary.0.size();
		
		qt.0.remove(event.entity);
		qt.0.insert((boundary.0, event.entity));
	}
}

fn handle_npc_spawn(
	time: Res<Time<Fixed>>,
	mut spawn_timer: ResMut<SpawnTimer>,
	mut ball_spawn: BallSpawnParams,
	mut commands: Commands,
) {
	spawn_timer.0.tick(time.delta());
	if spawn_timer.0.just_finished() {
		let position = Vec2::new(
			rand::thread_rng().gen_range(SPAWN_RANGE),
			rand::thread_rng().gen_range(SPAWN_RANGE));
		let random = rand::thread_rng().gen_range(1..=2);
		if random == 1 {
			spawn_regular_bawl(position, &mut ball_spawn, &mut commands);
		} else {
			spawn_shove_bawl(position, &mut ball_spawn, &mut commands)
		}
	}
}

fn handle_input(
	input: Res<ButtonInput<KeyCode>>,
	player: Single<(&mut MoveDirection, &MoveState), With<Player>>
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
			.clamp_length_max(0.5)
		}
		_ => {
			Vec2::ZERO
		}
	};
	
	move_dir.0 = dir.normalize_or_zero();
}

fn handle_shoving(
	mut commands: Commands,
	shovers: Query<(Entity, &Position, &Ball), With<Shover>>,
	targets: Query<&Position, With<Velocity>>,
	qt: Res<SolQT>,
) {
	for (entity, origin, ball) in shovers {
		let bounded_entities = qt.0.query_rect(Rect::from_center_size(origin.0, Vec2::splat(ball.0.radius)));
		for points in bounded_entities.iter() {
			let target_entity = points.1;
			if target_entity == entity { continue };
			
			if let Ok(targ_pos) = targets.get(target_entity) {
				
				let resultant = origin.0 - targ_pos.0;
				let dir = resultant.normalize();
				
				let force = dir * SHOVE_POWER;
				impulse(commands.entity(target_entity), -force);
			}
		}
	}
}

fn handle_npc_wander(
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

fn handle_position_projection(
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

fn handle_velocity(
	moveables: Query<(
		&mut Velocity, &MoveDirection, &MoveSpeed
	), Without<Impulse>> // directional move is not for entities impulsed
) {
	for (mut velocity, direction, speed) in moveables {
		velocity.0 = direction.0 * speed.0;
	}
}

fn handle_move(
	mut commands: Commands,
	moveables: Query<(
		Entity, &mut Position, &Velocity
	)>
) {
	for (entity, mut position, velocity) in moveables {
		let old_position = position.0.clone();
		let new_position = position.0 + velocity.0;
		
		position.0 = new_position;
		commands.trigger(EntityMoved {
			entity: entity,
			new_pos: new_position,
			old_pos: old_position,
		});
	}
}
