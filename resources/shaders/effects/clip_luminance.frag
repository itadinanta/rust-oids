#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

const float MAX_LUM = 10.0;

void main() {
	vec4 src = texture(t_Source, v_TexCoord, 0);
	float l = max((dot(vec3(0.2126, 0.7152, 0.0722), src.rgb) - 1.), 0.);

//	o_Color = l <= 0 ? src : vec4(src.rgb / l * min(MAX_LUM, l), src.a);
	o_Color = vec4(src.rgb * min(MAX_LUM, l), src.a);

}
