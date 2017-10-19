use gfx;
use core::color;

pub type Rgba = color::Rgba<f32>;
pub type HdrColorFormat = (gfx::format::R16_G16_B16_A16, gfx::format::Float);
pub type ColorFormat = gfx::format::Rgba8; // Srgba8;
pub type DepthFormat = gfx::format::Depth;

pub type HdrRenderSurface<R> = (gfx::handle::Texture<R, gfx::format::R16_G16_B16_A16>,
                                gfx::handle::ShaderResourceView<R, [f32; 4]>,
                                gfx::handle::RenderTargetView<R, HdrColorFormat>);

pub type DepthSurface<R> = (gfx::handle::Texture<R, gfx::format::D24>,
                            gfx::handle::ShaderResourceView<R, f32>,
                            gfx::handle::DepthStencilView<R, DepthFormat>);

pub type HdrRenderSurfaceWithDepth<R> = (gfx::handle::ShaderResourceView<R, [f32; 4]>,
                                         gfx::handle::RenderTargetView<R, HdrColorFormat>,
                                         gfx::handle::DepthStencilView<R, DepthFormat>);

pub const MSAA_MODE: gfx::texture::AaMode = gfx::texture::AaMode::Multi(4);