use super::*;
use frontend::render;
use frontend::render::Style;
use frontend::render::Draw;

impl App {
	pub fn environment(&self) -> Environment {
		Environment {
			light_color: self.lights.get(),
			background_color: self.backgrounds.get(),
			light_positions: self.world
				.feeders()
				.iter()
				.map(|e| e.transform().position)
				.collect::<Vec<_>>()
				.into_boxed_slice(),
		}
	}

	fn paint_particles<R>(&self, renderer: &mut R) where R: render::DrawBuffer {
		let mut batch = render::PrimitiveBuffer::new();
		for particle in self.world.particles() {
			let appearance = render::Appearance::new(particle.color(), particle.effect());
			let transform = Self::from_transform(&particle.transform()) * Matrix4::from_scale(particle.scale());
			batch.draw_quad(Some(Style::Particle), transform, 1.0, appearance);
		}
		renderer.draw_buffer(batch);
	}

	fn paint_particles_trails<R>(&self, renderer: &mut R) where R: render::DrawBuffer {
		let mut batch = render::PrimitiveBuffer::new();
		for particle in self.world.particles() {
			let appearance = render::Appearance::new(particle.color(), particle.effect());
			batch.draw_lines(None, Matrix4::identity(), particle.trail(), appearance);
		}
		renderer.draw_buffer(batch);
	}

	fn paint_minions<R>(&self, renderer: &mut R) where R: render::DrawBuffer {
		for (_, swarm) in self.world.swarms().iter() {
			let mut batch_buffer = render::PrimitiveBuffer::new();
			for (_, agent) in swarm.agents().iter() {
				let energy_left = agent.state.energy_ratio();
				let phase = agent.state.phase();
				for segment in agent.segments() {
					let body_transform = Self::from_transform(&segment.transform());

					let mesh = &segment.mesh();
					let fixture_scale = Matrix4::from_scale(mesh.shape.radius());
					let transform = body_transform * fixture_scale;

					let appearance = render::Appearance::new(segment.color(), [energy_left, phase, 0., 0.]);

					match mesh.shape {
						obj::Shape::Ball { .. } => {
							batch_buffer.draw_ball(None, transform, appearance);
						}
						obj::Shape::Star { .. } => {
							batch_buffer.draw_star(None, transform, &mesh.vertices[..], appearance);
						}
						obj::Shape::Poly { .. } => {
							batch_buffer.draw_star(None, transform, &mesh.vertices[..], appearance);
						}
						obj::Shape::Box { ratio, .. } => {
							batch_buffer.draw_quad(Some(Style::Wireframe), transform, ratio, appearance);
						}
						obj::Shape::Triangle { .. } => {
							batch_buffer.draw_triangle(None, transform, &mesh.vertices[0..3], appearance);
						}
					}
				}
			}
			renderer.draw_buffer(batch_buffer);
		}
	}

	fn paint_extent<R>(&self, renderer: &mut R)
		where R: render::Draw {
		let extent = &self.world.extent;
		let points = &[
			extent.min,
			Position::new(extent.min.x, extent.max.y),
			extent.max,
			Position::new(extent.max.x, extent.min.y),
			extent.min,
		];
		renderer.draw_lines(
			None,
			Matrix4::identity(),
			points,
			render::Appearance::rgba(self.lights.get()),
		);
		renderer.draw_quad(
			None,
			Matrix4::from_scale(extent.max.x - extent.min.x),
			1.,
			render::Appearance::rgba(self.backgrounds.get()),
		);
	}

	fn paint_hud<R>(&self, renderer: &mut R)
		where R: render::DrawBuffer {
		let mut batch_buffer = render::PrimitiveBuffer::new();
		for e in self.world.feeders() {
			let transform = Self::from_position(&e.transform().position);
			batch_buffer.draw_ball(None, transform, render::Appearance::rgba(self.lights.get()));
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
					batch_buffer.draw_lines(
						Some(Style::DebugLines),
						Matrix4::identity(),
						&[p0, p1],
						render::Appearance::rgba([1., 1., 0., 1.]),
					);

					let t0 = p1 - p0;
					let t = t0.normalize_to(t0.magnitude().min(radar_range));
					let m = Matrix2::from_angle(Rad(a0));

					let v = m * (-Position::unit_y());
					let p2 = p0 + v.normalize_to(t.dot(v));
					batch_buffer.draw_lines(
						Some(Style::DebugLines),
						Matrix4::identity(),
						&[p0, p2],
						render::Appearance::rgba([0., 1., 0., 1.]),
					);

					let u = m * (-Position::unit_x());
					let p3 = p0 + u.normalize_to(t.perp_dot(v));
					batch_buffer.draw_lines(
						Some(Style::DebugLines),
						Matrix4::identity(),
						&[p0, p3],
						render::Appearance::rgba([0., 1., 0., 1.]),
					);

					let trajectory = agent.state.trajectory();
					let appearance = render::Appearance::new(sensor.color(), [2.0, 1.0, 0., 0.]);
					batch_buffer.draw_lines(Some(Style::DebugLines), Matrix4::identity(), &trajectory, appearance);

					for segment in agent.segments().iter() {
						match segment.state.intent {
							segment::Intent::Brake(v) => {
								let p0 = segment.transform.position;
								let p1 = p0 + v * DEBUG_DRAW_BRAKE_SCALE;
								batch_buffer.draw_lines(
									Some(Style::DebugLines),
									Matrix4::identity(),
									&[p0, p1],
									render::Appearance::rgba([2., 0., 0., 1.]),
								);
							}
							segment::Intent::Move(v) => {
								let p0 = segment.transform.position;
								let p1 = p0 + v * DEBUG_DRAW_MOVE_SCALE;
								batch_buffer.draw_lines(
									Some(Style::DebugLines),
									Matrix4::identity(),
									&[p0, p1],
									render::Appearance::rgba([0., 0., 2., 1.]),
								);
							}
							_ => {}
						}
					}
				}
			}
		};
		renderer.draw_buffer(batch_buffer)
	}

	pub fn paint<R>(&self, renderer: &mut R)
		where R: render::Draw + render::DrawBatch + render::DrawBuffer {
		self.paint_minions(renderer);
		self.paint_extent(renderer);
		self.paint_particles(renderer);
		self.paint_particles_trails(renderer);
		self.paint_hud(renderer);
	}
}