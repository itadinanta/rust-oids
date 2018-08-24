use num;

pub type Rgb<T = f32> = [T; 3];
pub type Rgba<T = f32> = [T; 4];

pub trait ToRgb<T: num::Float> {
	fn to_rgb(&self) -> Rgb<T>;

	fn to_rgba(&self) -> Rgba<T> {
		let rgb = self.to_rgb();
		[rgb[0], rgb[1], rgb[2], T::one()]
	}
}

pub trait FromRgb<T: num::Float>: Sized {
	fn from_rgb(c: &Rgb<T>) -> Self;

	fn from_rgba(c: &Rgba<T>) -> Self { Self::from_rgb(&[c[0], c[1], c[2]]) }
}

pub trait Fade<F, T>
where T: num::Float {
	fn fade(&self, other: F, alpha: T) -> F;
}

impl<T> Fade<Rgba<T>, T> for Rgba<T>
where T: num::Float
{
	fn fade(&self, other: Rgba<T>, alpha: T) -> Rgba<T> {
		let alpha1 = T::one() - alpha;
		[
			self[0] * alpha1 + other[0] * alpha,
			self[1] * alpha1 + other[1] * alpha,
			self[2] * alpha1 + other[2] * alpha,
			self[3] * alpha1 + other[3] * alpha,
		]
	}
}

#[derive(Debug, Copy, Clone)]
pub struct Hsl<T: num::Float> {
	h: T,
	s: T,
	l: T,
}

#[derive(Debug, Copy, Clone)]
pub struct YPbPr<T: num::Float> {
	// luma/chroma. y in [0,1], cb, cr in [-0.5, 0.5]
	y: T,
	pb: T,
	pr: T,
}

impl FromRgb<f32> for YPbPr<f32> {
	fn from_rgb(c: &Rgb<f32>) -> Self {
		let (r, g, b) = (c[0], c[1], c[2]);
		YPbPr {
			y: 0.299_000 * r + 0.587_000 * g + 0.114_000 * b,
			pb: -0.168_736 * r - 0.331_264 * g + 0.500_000 * b,
			pr: 0.500_000 * r - 0.418_688 * g - 0.081_312 * b,
		}
	}
}

impl<T> YPbPr<T>
where T: num::Float
{
	pub fn new(y: T, pb: T, pr: T) -> Self { YPbPr { y, pb, pr } }
}

impl ToRgb<f32> for YPbPr<f32> {
	fn to_rgb(&self) -> Rgb<f32> {
		let r = self.y + 1.402 * self.pr;
		let g = self.y - 0.344_136 * self.pb - 0.714_136 * self.pr;
		let b = self.y + 1.772 * self.pb;
		[r.max(0.).min(1.), g.max(0.).min(1.), b.max(0.).min(1.)]
	}
}

impl<T> Hsl<T>
where T: num::Float
{
	pub fn new(h: T, s: T, l: T) -> Self { Hsl { h, s, l } }
}

impl FromRgb<f32> for Hsl<f32> {
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
	#[allow(float_cmp)]
	#[allow(many_single_char_names)]
	fn from_rgb(c: &Rgb<f32>) -> Self {
		let (r, g, b) = (c[0], c[1], c[2]);
		let max = f32::max(r, f32::max(g, b));
		let min = f32::min(r, f32::min(g, b));
		let m = (max + min) / 2.;

		if max == min {
			Hsl { h: 0., s: 0., l: m }
		} else {
			let d = max - min;
			Hsl {
				h: if max == r {
					(g - b) / d + if g < b { 6. } else { 0. }
				} else if max == g {
					(b - r) / d + 2.
				} else {
					(r - g) / d + 4.
				} / 6.,
				s: if b > 0.5 { d / (2. - max - min) } else { d / (max + min) },
				l: m,
			}
		}
	}
}

impl ToRgb<f32> for Hsl<f32> {
	/// Converts an HSL color value to RGB. Conversion formula
	/// adapted from http://en.wikipedia.org/wiki/HSL_color_space.
	/// Assumes h, s, and l are contained in the set [0, 1] and
	/// returns r, g, and b in the set [0, 1].
	///
	/// @param   Number  h       The hue
	/// @param   Number  s       The saturation
	/// @param   Number  l       The lightness
	/// @return  Array           The RGB representation
	///
	fn to_rgb(&self) -> Rgb<f32> {
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

		match *self {
			Hsl { h, l, .. } if h == 0. => [l, l, l],
			Hsl { h, s, l } => {
				let q = if l < 0.5 { l * (1. + s) } else { l + s - l * s };
				let p = 2. * l - q;
				let r = hue2rgb(p, q, h + 1. / 3.);
				let g = hue2rgb(p, q, h);
				let b = hue2rgb(p, q, h - 1. / 3.);

				[r, g, b]
			}
		}
	}
}
