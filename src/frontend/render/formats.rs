use gfx;
use core::color;

pub type Rgba = color::Rgba<f32>;
pub type Float4 = [f32; 4];
pub type Float = f32;
pub type RenderColorChannels = gfx::format::R16_G16_B16_A16;
pub type RenderColorFormat = (RenderColorChannels, gfx::format::Float);
pub type RenderDepthFormat = gfx::format::Depth;
pub type ScreenColorFormat = gfx::format::Rgba8; // Srgba8;
pub type ScreenDepthFormat = gfx::format::Depth;

pub type RenderSurface<R> = (gfx::handle::Texture<R, RenderColorChannels>,
                             gfx::handle::ShaderResourceView<R, Float4>,
                             gfx::handle::RenderTargetView<R, RenderColorFormat>);

pub type DepthSurface<R> = (gfx::handle::Texture<R, gfx::format::D24>,
                            gfx::handle::ShaderResourceView<R, Float>,
                            gfx::handle::DepthStencilView<R, RenderDepthFormat>);

pub type RenderSurfaceWithDepth<R> = (gfx::handle::ShaderResourceView<R, Float4>,
                                      gfx::handle::RenderTargetView<R, RenderColorFormat>,
                                      gfx::handle::DepthStencilView<R, RenderDepthFormat>);

pub const MSAA_MODE: gfx::texture::AaMode = gfx::texture::AaMode::Multi(4);
