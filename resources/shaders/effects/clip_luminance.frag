#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

const float MAX_LUM = 10.0;

vec4 lum_clip(float x, float y) {
	vec4 src = texture(t_Source, vec2(x, y), 0);
	float l = max((dot(vec3(0.2126, 0.7152, 0.0722), src.rgb) - 1.), 0.);
//	return l <= 0 ? src : vec4(src.rgb / l * min(MAX_LUM, l), src.a);
	return vec4(src.rgb * min(MAX_LUM, l), src.a);
}

void main() {
	vec2 d = 1.0 / textureSize(t_Source, 0);
	float x1 = v_TexCoord.x - d.x / 2;
	float x2 = x1 + d.x;
	float y1 = v_TexCoord.y - d.y / 2.;
	float y2 = y1 + d.y;

	o_Color = (lum_clip(x1, y1) + lum_clip(x2, y1) + lum_clip(x1, y2)
			+ lum_clip(x2, y2)) / 4.0;
}
