#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

void main() {
	o_Color = texture(t_Source, v_TexCoord, 0);
}
