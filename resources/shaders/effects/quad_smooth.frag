#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

void main() {
	vec2 d = 1.0 / textureSize(t_Source, 0); // gets size of single texel
	float x1 = v_TexCoord.x - d.x / 2;
	float x2 = x1 + d.x;
	float y1 = v_TexCoord.y - d.y / 2.;
	float y2 = y1 + d.y;
	o_Color = (texture(t_Source, vec2(x1, y1), 0)
			+ texture(t_Source, vec2(x1, y2), 0)
			+ texture(t_Source, vec2(x2, y1), 0)
			+ texture(t_Source, vec2(x2, y2), 0)) / 4.0;
}
