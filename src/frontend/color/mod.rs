use num;
use std::f32::consts;


struct Hsl<T: num::Float> {
	h: T,
	s: T,
	l: T,
}

impl<T> Hsl<T>
    where T: num::Float
{
	fn new(h: T, s: T, l: T) -> Self {
		Hsl { h: h, s: s, l: l }
	}
}

impl Hsl<f32> {
	/// http://axonflux.com/handy-rgb-to-hsl-and-rgb-to-hsv-color-model-c
	///
	/// Converts an RGB color value to HSL. Conversion formula
	/// adapted from http://en.wikipedia.org/wiki/HSL_color_space.
	/// Assumes r, g, and b are contained in the set [0, 255] and
	/// returns h, s, and l in the set [0, 1].
	///
	/// @param   Number  r       The red color value
	/// @param   Number  g       The green color value
	/// @param   Number  b       The blue color value
	/// @return  Array           The HSL representation
	///
	fn from_rgb(r: f32, g: f32, b: f32) -> Self {
		let max = f32::max(r, f32::max(g, b));
		let min = f32::min(r, f32::min(g, b));
		let b = (max + min) / 2.;
		let (h0, s0, l0) = (b, b, b);

		if max == min {
			Hsl {
				h: 0.,
				s: 0.,
				l: l0,
			}
		} else {
			let d = max - min;
			let s = if l0 > 0.5 {
				d / (2. - max - min)
			} else {
				d / (max + min)
			};
			let h = if max == r {
				(g - b) / d +
				if g < b {
					6.
				} else {
					0.
				}
			} else if max == g {
				(b - r) / d + 2.
			} else {
				(r - g) / d + 4.
			};
			Hsl {
				h: (h / 6.) * 2. * consts::PI,
				s: s0,
				l: l0,
			}
		}
	}

	/// Converts an HSL color value to RGB. Conversion formula
	/// adapted from http://en.wikipedia.org/wiki/HSL_color_space.
	/// Assumes h, s, and l are contained in the set [0, 1] and
	/// returns r, g, and b in the set [0, 255].
	///
	/// @param   Number  h       The hue
	/// @param   Number  s       The saturation
	/// @param   Number  l       The lightness
	/// @return  Array           The RGB representation
	///
	pub fn to_rgb(&self) -> [f32; 3] {

		fn hue2rgb(p: f32, q: f32, t0: f32) -> f32 {
			let t = if t0 < 0. {
				t0 + 1.
			} else if t0 > 1. {
				t0 - 1.
			} else {
				t0
			};
			if t < 1. / 6. {
				p + (q - p) * 6. * t
			} else if t < 1. / 2. {
				q
			} else if t < 2. / 3. {
				p + (q - p) * (2. / 3. - t) * 6.
			} else {
				p
			}
		}

		match self {
			&Hsl { h: 0., l, .. } => [l, l, l],
			&Hsl { h, s, l } => {
				let q = if l < 0.5 {
					l * (1. + s)
				} else {
					l + s - l * s
				};
				let p = 2. * l - q;
				let r = hue2rgb(p, q, h + 1. / 3.);
				let g = hue2rgb(p, q, h);
				let b = hue2rgb(p, q, h - 1. / 3.);

				[r, g, b]
			}
		}
	}
}
