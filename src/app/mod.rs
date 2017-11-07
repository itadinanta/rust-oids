mod main;
mod ev;

use std::rc::Rc;
use std::cell::RefCell;

use core::util::Cycle;
use core::geometry::*;
use core::geometry::Transform;
use core::clock::*;
use core::math;
use core::math::Directional;
use core::math::Relative;
use core::math::Smooth;

use core::resource::ResourceLoader;
use num::Zero;

use backend::obj;
use backend::obj::*;
use backend::world;
use backend::world::segment;
use backend::world::agent;
use backend::systems;
use backend::systems::System;

use frontend::input;
use frontend::render;

use getopts::Options;
use std::ffi::OsString;

use num;
use cgmath;
use cgmath::{Matrix4, SquareMatrix};

pub enum Event {
    CamUp,
    CamDown,
    CamLeft,
    CamRight,

    CamReset,

    NextLight,
    PrevLight,

    NextBackground,
    PrevBackground,

    NextSpeedFactor,
    PrevSpeedFactor,

    Reload,
    DumpToFile,
    ToggleDebug,

    TogglePause,

    AppQuit,

    NewMinion(Position),
    RandomizeMinion(Position),

    SelectMinion(Position, Id),
    DeselectAll,

    BeginDrag(Position, Position),
    Drag(Position, Position),
    EndDrag(Position, Position, Velocity),
}

pub fn run(args: &[OsString]) {
    let options = Options::new()
        .optflag("t", "terminal", "Headless mode")
        .parse(args)
        .unwrap();

    let pool_file_name = options.free.get(1).map(|n| n.as_str()).unwrap_or(
        "minion_gene_pool.csv",
    );
    if options.opt_present("t") {
        main::main_loop_headless(pool_file_name);
    } else {
        main::main_loop(pool_file_name);
    }
}

pub struct Viewport {
    width: u32,
    height: u32,
    pub ratio: f32,
    pub scale: f32,
}

impl Viewport {
    fn rect(w: u32, h: u32, scale: f32) -> Viewport {
        Viewport {
            width: w,
            height: h,
            ratio: (w as f32 / h as f32),
            scale,
        }
    }

    fn to_world(&self, pos: &Position) -> Position {
        let dx = self.width as f32 / self.scale;
        let tx = (pos.x - (self.width as f32 * 0.5)) / dx;
        let ty = ((self.height as f32 * 0.5) - pos.y) / dx;
        Position::new(tx, ty)
    }
}

#[derive(Default)]
pub struct Systems {
    physics: systems::PhysicsSystem,
    animation: systems::AnimationSystem,
    game: systems::GameSystem,
    ai: systems::AiSystem,
    alife: systems::AlifeSystem,
}

impl Systems {
    fn systems(&mut self) -> Vec<&mut systems::System> {
        vec![
            &mut self.animation as &mut systems::System,
            &mut self.game as &mut systems::System,
            &mut self.ai as &mut systems::System,
            &mut self.alife as &mut systems::System,
            &mut self.physics as &mut systems::System,
        ]
    }

    fn for_each(&mut self, apply: &Fn(&mut systems::System)) {
        for r in self.systems().as_mut_slice() {
            apply(*r);
        }
    }

    fn from_world(&mut self, world: &world::World, apply: &Fn(&mut systems::System, &world::World)) {
        for r in self.systems().as_mut_slice() {
            apply(*r, &world);
        }
    }

    fn to_world(&mut self, mut world: &mut world::World, apply: &Fn(&mut systems::System, &mut world::World)) {
        for r in self.systems().as_mut_slice() {
            apply(*r, &mut world);
        }
    }
}

bitflags! {
	pub struct DebugFlags: u32 {
		const DEBUG_TARGETS = 0x1;
	}
}

pub struct App {
    pub viewport: Viewport,
    input_state: input::InputState,
    wall_clock: SharedTimer<SystemTimer>,
    simulations_count: usize,
    frame_count: usize,
    frame_stopwatch: TimerStopwatch<SystemTimer>,
    frame_elapsed: SimulationTimer,
    frame_smooth: math::MovingAverage<Seconds>,
    is_running: bool,
    is_paused: bool,
    //
    camera: math::Inertial<f32>,
    lights: Cycle<Rgba>,
    backgrounds: Cycle<Rgba>,
    speed_factors: Cycle<f32>,
    //
    world: world::World,
    systems: Systems,
    //
    debug_flags: DebugFlags,
}

pub struct Environment {
    pub light_color: Rgba,
    pub light_positions: Box<[Position]>,
    pub background_color: Rgba,
}


pub struct SimulationUpdate {
    pub timestamp: Seconds,
    pub dt: Seconds,
    pub count: usize,
    pub elapsed: Seconds,
    pub population: usize,
    pub extinctions: usize,
}

pub struct FrameUpdate {
    pub timestamp: Seconds,
    pub dt: Seconds,
    pub count: usize,
    pub elapsed: Seconds,
    pub duration_smooth: Seconds,
    pub fps: f32,
    pub simulation: SimulationUpdate,
}


impl App {
    pub fn new<R>(w: u32, h: u32, scale: f32, resource_loader: &R, minion_gene_pool: &str) -> Self
        where
            R: ResourceLoader<u8>, {
        let system_timer = Rc::new(RefCell::new(SystemTimer::new()));
        App {
            viewport: Viewport::rect(w, h, scale),
            input_state: input::InputState::default(),

            camera: Self::init_camera(),
            lights: Self::init_lights(),
            backgrounds: Self::init_backgrounds(),
            speed_factors: Self::init_speed_factors(),

            world: world::World::new(resource_loader, minion_gene_pool),
            // subsystems
            systems: Systems::default(),
            // runtime and timing
            simulations_count: 0usize,
            frame_count: 0usize,
            frame_elapsed: SimulationTimer::new(),
            frame_stopwatch: TimerStopwatch::new(system_timer.clone()),
            wall_clock: system_timer,
            frame_smooth: math::MovingAverage::new(120),
            is_running: true,
            is_paused: false,
            // debug
            debug_flags: DebugFlags::empty(),
        }
    }

    fn init_camera() -> math::Inertial<f32> {
        math::Inertial::new(10.0, 0.5, 0.5)
    }

    fn init_lights() -> Cycle<[f32; 4]> {
        Cycle::new(
            &[
                [1.0, 1.0, 1.0, 1.0],
                [3.1, 3.1, 3.1, 1.0],
                [10.0, 10.0, 10.0, 1.0],
                [31.0, 31.0, 31.0, 1.0],
                [100.0, 100.0, 100.0, 1.0],
                [0.001, 0.001, 0.001, 1.0],
                [0.01, 0.01, 0.01, 1.0],
                [0.1, 0.1, 0.1, 1.0],
                [0.31, 0.31, 0.31, 0.5],
            ],
        )
    }

    fn init_speed_factors() -> Cycle<f32> {
        Cycle::new(
            &[
                1.0f32,
                0.5f32,
                0.2f32,
                0.1f32,
                2.0f32,
                5.0f32,
            ]
        )
    }

    fn init_backgrounds() -> Cycle<[f32; 4]> {
        Cycle::new(
            &[
                [0.05, 0.07, 0.1, 1.0],
                [0.5, 0.5, 0.5, 0.5],
                [1.0, 1.0, 1.0, 1.0],
                [3.1, 3.1, 3.1, 1.0],
                [10.0, 10.0, 10.0, 1.0],
                [0., 0., 0., 1.0],
                [0.01, 0.01, 0.01, 1.0],
            ],
        )
    }

    pub fn pick_minion(&self, pos: Position) -> Option<Id> {
        self.systems.physics.pick(pos)
    }

    fn randomize_minion(&mut self, pos: Position) {
        self.world.randomize_minion(pos, None);
    }

    fn new_minion(&mut self, pos: Position) {
        self.world.new_minion(pos, None);
    }

    fn deselect_all(&mut self) {
        self.world.for_all_agents(
            &mut |agent| agent.state.deselect(),
        );
    }

    fn select_minion(&mut self, id: Id) {
        self.debug_flags |= DebugFlags::DEBUG_TARGETS;
        self.world.agent_mut(id).map(|a| a.state.toggle_selection());
    }

    fn register_all(&mut self) {
        for id in self.world.registered().into_iter() {
            if let Some(found) = self.world.agent_mut(*id) {
                self.systems.physics.register(found);
            }
        }
    }

    pub fn on_app_event(&mut self, e: Event) {
        match e {
            Event::CamUp => self.camera.push(math::Direction::Up),
            Event::CamDown => self.camera.push(math::Direction::Down),
            Event::CamLeft => self.camera.push(math::Direction::Left),
            Event::CamRight => self.camera.push(math::Direction::Right),

            Event::CamReset => {
                self.camera.reset();
            }
            Event::NextLight => {
                self.lights.next();
            }
            Event::PrevLight => {
                self.lights.prev();
            }
            Event::NextBackground => {
                self.backgrounds.next();
            }
            Event::PrevBackground => {
                self.backgrounds.prev();
            }
            Event::NextSpeedFactor => {
                self.speed_factors.next();
            }
            Event::PrevSpeedFactor => {
                self.speed_factors.prev();
            }
            Event::ToggleDebug => self.debug_flags.toggle(DebugFlags::DEBUG_TARGETS),
            Event::Reload => {}

            Event::AppQuit => self.quit(),
            Event::TogglePause => self.is_paused = !self.is_paused,

            Event::DumpToFile => {
                match self.world.dump() {
                    Err(_) => error!("Failed to dump log"),
                    Ok(name) => info!("Saved {}", name),
                }
            }
            Event::BeginDrag(_, _) => {
                self.camera.zero();
            }
            Event::Drag(start, end) => {
                self.camera.set_relative(start - end);
            }
            Event::EndDrag(start, end, vel) => {
                self.camera.set_relative(start - end);
                self.camera.velocity(vel);
            }
            Event::SelectMinion(_, id) => self.select_minion(id),
            Event::DeselectAll => self.deselect_all(),
            Event::NewMinion(pos) => self.new_minion(pos),
            Event::RandomizeMinion(pos) => self.randomize_minion(pos),
        }
    }

    pub fn quit(&mut self) {
        self.is_running = false;
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn on_input_event(&mut self, e: &input::Event) {
        self.input_state.event(e);
    }

    fn update_input(&mut self, dt: Seconds) {
        let mut events = Vec::new();

        macro_rules! on_key_held {
			[$($key:ident -> $app_event:ident),*] => (
				$(if self.input_state.key_pressed(input::Key::$key) { events.push(Event::$app_event); })
				*
			)
		}
        macro_rules! on_key_pressed_once {
			[$($key:ident -> $app_event:ident),*] => (
				$(if self.input_state.key_once(input::Key::$key) { events.push(Event::$app_event); })
				*
			)
		}
        on_key_held![
			Up -> CamUp,
			Down -> CamDown,
			Left -> CamLeft,
			Right-> CamRight
		];

        on_key_pressed_once![
			F5 -> Reload,
			N0 -> CamReset,
			Home -> CamReset,
			KpHome -> CamReset,
			F6 -> DumpToFile,
			D -> ToggleDebug,
			Z -> DeselectAll,
			L -> NextLight,
			B -> NextBackground,
			K -> PrevLight,
			V -> PrevBackground,
			G -> PrevSpeedFactor,
			H -> NextSpeedFactor,
			P -> TogglePause,
			Esc -> AppQuit
		];

        let mouse_window_pos = self.input_state.mouse_position();
        let mouse_view_pos = self.to_view(&mouse_window_pos);
        let mouse_world_pos = self.to_world(&mouse_view_pos);

        let picked_id = if self.input_state.key_once(input::Key::MouseLeft) {
            self.pick_minion(mouse_world_pos)
        } else {
            None
        };

        if self.input_state.key_once(input::Key::MouseRight) {
            if self.input_state.any_ctrl_pressed() {
                events.push(Event::RandomizeMinion(mouse_world_pos));
            } else {
                events.push(Event::NewMinion(mouse_world_pos));
            }
        }

        if let Some(picked) = picked_id {
            events.push(Event::SelectMinion(mouse_world_pos, picked));
        } else {
            match self.input_state.dragging(
                input::Key::MouseLeft,
                mouse_view_pos,
            ) {
                input::Dragging::Begin(_, from) => {
                    let from = self.to_world(&from);
                    events.push(Event::BeginDrag(from, from));
                }
                input::Dragging::Dragging(_, from, to) => {
                    events.push(Event::Drag(self.to_world(&from), self.to_world(&to)));
                }
                input::Dragging::End(_, from, to, prev) => {
                    let mouse_vel = (self.to_view(&prev) - to) / dt.into();
                    events.push(Event::EndDrag(
                        self.to_world(&from),
                        self.to_world(&to),
                        mouse_vel,
                    ));
                }
                _ => {}
            }
        }


        for e in events {
            self.on_app_event(e)
        }
    }

    fn to_view(&self, pos: &Position) -> Position {
        self.viewport.to_world(pos)
    }

    fn to_world(&self, t: &Position) -> Position {
        t + self.camera.position()
    }

    pub fn on_resize(&mut self, width: u32, height: u32) {
        self.viewport = Viewport::rect(width, height, self.viewport.scale);
    }

    fn from_transform(transform: &Transform) -> Matrix4<f32> {
        use cgmath::Rotation3;
        let position = transform.position;
        let angle = transform.angle;
        let rot = Matrix4::from(cgmath::Quaternion::from_axis_angle(
            cgmath::Vector3::unit_z(),
            cgmath::Rad(angle),
        ));
        let trans = Matrix4::from_translation(cgmath::Vector3::new(position.x, position.y, 0.0));

        trans * rot
    }

    fn from_position(position: &Position) -> Matrix4<f32> {
        Matrix4::from_translation(cgmath::Vector3::new(position.x, position.y, 0.0))
    }

    fn render_minions(&self, renderer: &mut render::Draw) {
        for (_, swarm) in self.world.swarms().iter() {
            for (_, agent) in swarm.agents().iter() {
                let energy_left = agent.state.energy_ratio();
                let age = agent.state.lifecycle().elapsed();
                for segment in agent.segments() {
                    let body_transform = Self::from_transform(&segment.transform());

                    let mesh = &segment.mesh();
                    let fixture_scale = Matrix4::from_scale(mesh.shape.radius());
                    let transform = body_transform * fixture_scale;

                    let appearance = render::Appearance::new(segment.color(), [energy_left, age.into(), 0., 0.]);

                    match mesh.shape {
                        obj::Shape::Ball { .. } => {
                            renderer.draw_ball(&transform, &appearance);
                        }
                        obj::Shape::Star { .. } => {
                            renderer.draw_star(&transform, &mesh.vertices[..], &appearance);
                        }
                        obj::Shape::Poly { .. } => {
                            renderer.draw_star(&transform, &mesh.vertices[..], &appearance);
                        }
                        obj::Shape::Box { ratio, .. } => {
                            renderer.draw_quad(&transform, ratio, &appearance);
                        }
                        obj::Shape::Triangle { .. } => {
                            renderer.draw_triangle(&transform, &mesh.vertices[0..3], &appearance);
                        }
                    }
                }
            }
        }
    }

    fn render_extent(&self, renderer: &mut render::Draw) {
        let extent = &self.world.extent;
        let points = &[
            extent.min,
            Position::new(extent.min.x, extent.max.y),
            extent.max,
            Position::new(extent.max.x, extent.min.y),
            extent.min,
        ];
        renderer.draw_lines(
            &Matrix4::identity(),
            points,
            &render::Appearance::rgba(self.lights.get()),
        );
        renderer.draw_quad(
            &Matrix4::from_scale(extent.max.x - extent.min.x),
            1.,
            &render::Appearance::rgba(self.backgrounds.get()),
        );
    }

    fn render_hud(&self, renderer: &mut render::Draw) {
        for e in self.world.emitters() {
            let transform = Self::from_position(&e.transform().position);
            renderer.draw_ball(&transform, &render::Appearance::rgba(self.lights.get()));
        }
        if self.debug_flags.contains(DebugFlags::DEBUG_TARGETS) {
            use cgmath::*;
            for (_, agent) in self.world.agents(world::agent::AgentType::Minion).iter() {
                if agent.state.selected() {
                    let sensor = agent.first_segment(segment::Flags::HEAD).unwrap();
                    let p0 = sensor.transform.position;
                    let a0 = sensor.transform.angle;
                    let radar_range = sensor.mesh.shape.radius() * 10.;
                    let p1 = *agent.state.target_position();
                    renderer.draw_debug_lines(
                        &Matrix4::identity(),
                        &[p0, p1],
                        &render::Appearance::rgba([1., 1., 0., 1.]),
                    );

                    let t0 = p1 - p0;
                    let t = t0.normalize_to(t0.magnitude().min(radar_range));
                    let m = Matrix2::from_angle(Rad(a0));

                    let v = m * (-Position::unit_y());
                    let p2 = p0 + v.normalize_to(t.dot(v));
                    renderer.draw_debug_lines(
                        &Matrix4::identity(),
                        &[p0, p2],
                        &render::Appearance::rgba([0., 1., 0., 1.]),
                    );

                    let u = m * (-Position::unit_x());
                    let p3 = p0 + u.normalize_to(t.perp_dot(v));
                    renderer.draw_debug_lines(
                        &Matrix4::identity(),
                        &[p0, p3],
                        &render::Appearance::rgba([0., 1., 0., 1.]),
                    );

                    let trajectory = agent.state.trajectory();
                    let appearance = render::Appearance::new(sensor.color(), [2.0, 1.0, 0., 0.]);
                    renderer.draw_debug_lines(&Matrix4::identity(), &trajectory, &appearance);

                    for segment in agent.segments().iter() {
                        match segment.state.intent {
                            segment::Intent::Brake(v) => {
                                let p0 = segment.transform.position;
                                let p1 = p0 + v * 0.05;
                                renderer.draw_debug_lines(
                                    &Matrix4::identity(),
                                    &[p0, p1],
                                    &render::Appearance::rgba([2., 0., 0., 1.]),
                                );
                            }
                            segment::Intent::Move(v) => {
                                let p0 = segment.transform.position;
                                let p1 = p0 + v * 0.05;
                                renderer.draw_debug_lines(
                                    &Matrix4::identity(),
                                    &[p0, p1],
                                    &render::Appearance::rgba([0., 0., 2., 1.]),
                                );
                            }
                            segment::Intent::Idle => {}
                            segment::Intent::RunAway(_) => {}
                        }
                    }
                }
            }
        }
    }

    pub fn render(&self, renderer: &mut render::Draw) {
        self.render_minions(renderer);
        self.render_extent(renderer);
        self.render_hud(renderer);
    }

    pub fn environment(&self) -> Environment {
        Environment {
            light_color: self.lights.get(),
            background_color: self.backgrounds.get(),
            light_positions: self.world
                .emitters()
                .iter()
                .map(|e| e.transform().position)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    pub fn init(&mut self) {
        self.init_systems();
    }

    fn init_systems(&mut self) {
        self.systems.from_world(
            &self.world,
            &|s, world| s.init(&world),
        );
    }

    fn cleanup(&mut self) {
        let freed = self.world.sweep();
        self.systems.for_each(&|s| for freed_agent in freed.iter() {
            s.unregister(freed_agent);
        });
    }

    fn tick(&mut self, dt: Seconds) {
        self.world.tick(dt);
    }

    fn update_systems(&mut self, dt: Seconds) {
        self.systems.to_world(&mut self.world, &|s, mut world| {
            s.update_world(&mut world, dt)
        });
    }

    pub fn audio_events(&mut self) {}

    pub fn update(&mut self) -> FrameUpdate {
        const MIN_FRAME_LENGTH: f32 = 1.0f32 / 1000.0f32;
        const MAX_FRAME_LENGTH: f32 = 1.0f32 / 15.0f32;

        let frame_time = self.frame_stopwatch.restart();
        self.frame_elapsed.tick(frame_time);

        let frame_time_smooth = self.frame_smooth.smooth(frame_time);
        self.camera.update(frame_time_smooth);

        let target_duration = frame_time_smooth.get();
        self.update_input(frame_time_smooth);

        let dt = if self.is_paused {
            Seconds::zero()
        } else {
            Seconds::new(self.speed_factors.get() *
                    num::clamp(target_duration,
                               MIN_FRAME_LENGTH,
                               MAX_FRAME_LENGTH))
        };

        let simulation_update = self.simulate(dt);

        self.frame_count += 1;

        FrameUpdate {
            timestamp: self.wall_clock.borrow().seconds(),
            dt: frame_time,
            count: self.frame_count,
            elapsed: self.frame_elapsed.seconds(),
            duration_smooth: frame_time_smooth,
            fps: 1.0f32 / target_duration,
            simulation: simulation_update,
        }
    }

    pub fn simulate(&mut self, dt: Seconds) -> SimulationUpdate {
        self.cleanup();
        self.update_systems(dt);
        self.register_all();
        self.tick(dt);

        self.simulations_count += 1;

        SimulationUpdate {
            timestamp: self.wall_clock.borrow().seconds(),
            dt,
            count: self.simulations_count,
            elapsed: self.world.seconds(),
            population: self.world.agents(agent::AgentType::Minion).len(),
            extinctions: self.world.extinctions(),
        }
    }
}
