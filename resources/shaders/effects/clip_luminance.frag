#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

void main() {
	vec4 src = texture(t_Source, v_TexCoord, 0);
	float l = max((dot(vec3(0.2126, 0.7152, 0.0722), src.rgb) - 1.), 0.);

	o_Color = vec4(src.rgb * l, src.a);
}
