use gfx;

pub static VERTEX_SRC: &'static [u8] = b"

#version 150 core

in ivec2 a_Pos;
out vec2 v_TexCoord;

void main() {
    v_TexCoord = ivec2(a_Pos);
    gl_Position = vec4(a_Pos, 0.0, 1.0);
}

";

pub static SIMPLE_BLIT_SRC: &'static [u8] = b"

#version 150 core

uniform sampler2D t_Source;

in ivec2 v_TexCoord;
out vec4 o_Color;

void main() {
    o_Color = texelFetch(t_source, v_TexCoord, 0);
}

";

pub static DOWNSAMPLE_COLOR_SRC: &'static [u8] = b"

#version 150 core

uniform sampler2D t_Source;

in ivec2 v_TexCoord;
out vec4 o_Color;

void main() {
	int x1 = v_TexCoord.x * 2;
	int x2 = x1 + 1;
	int y1 = v_TexCoord.y * 2;
	int y2 = x1 + 1;
    o_Color = (texelFetch(t_source, ivec2(x1, y1), 0) 
	         + texelFetch(t_source, ivec2(x1, y2), 0)
	         + texelFetch(t_source, ivec2(x2, y1), 0)
	         + texelFetch(t_source, ivec2(x2, y2), 0)) / 4.0;
}

";

pub type HDRColorFormat = (gfx::format::R16_G16_B16_A16, gfx::format::Float);

gfx_defines!{
	vertex BlitVertex {
		pos: [i16; 2] = "a_Pos",
	}
	
	pipeline blit {
		vbuf: gfx::VertexBuffer<BlitVertex> = (),
		src: gfx::TextureSampler<[f32; 4]> = "t_Source",
		dst: gfx::RenderTarget<HDRColorFormat> = "Target0",
	}
}
