#version 150 core

uniform sampler2DMS t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

void main() {
	vec2 d = textureSize(t_Source); // gets size of texture - is it in pixels or fragments?
	ivec2 i = ivec2(d * v_TexCoord);
	o_Color = (texelFetch(t_Source, i, 0) + texelFetch(t_Source, i, 1)
			+ texelFetch(t_Source, i, 2) + texelFetch(t_Source, i, 3)) / 4.0;
}
