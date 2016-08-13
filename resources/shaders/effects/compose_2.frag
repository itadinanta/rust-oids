#version 150 core

uniform sampler2D t_Source1;
uniform sampler2D t_Source2;

in vec2 v_TexCoord;
out vec4 o_Color;

void main() {
	o_Color = texture(t_Source1, v_TexCoord, 0)
			+ texture(t_Source2, v_TexCoord, 0);
}
