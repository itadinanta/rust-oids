#version 150 core

uniform sampler2D t_Source;

in vec2 v_TexCoord;
out vec4 o_Color;

const float weight[5] = float[] (0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

void main() {
	vec2 tex_offset = 1.0 / textureSize(t_Source, 0); // gets size of single texel
	vec3 result = texture(t_Source, v_TexCoord).rgb * weight[0]; // current fragment's contribution

	result += texture(t_Source, v_TexCoord + vec2(tex_offset.x * 1, 0.0)).rgb
			* weight[1];
	result += texture(t_Source, v_TexCoord - vec2(tex_offset.x * 1, 0.0)).rgb
			* weight[1];

	result += texture(t_Source, v_TexCoord + vec2(tex_offset.x * 2, 0.0)).rgb
			* weight[2];
	result += texture(t_Source, v_TexCoord - vec2(tex_offset.x * 2, 0.0)).rgb
			* weight[2];

	result += texture(t_Source, v_TexCoord + vec2(tex_offset.x * 3, 0.0)).rgb
			* weight[3];
	result += texture(t_Source, v_TexCoord - vec2(tex_offset.x * 3, 0.0)).rgb
			* weight[3];

	result += texture(t_Source, v_TexCoord + vec2(tex_offset.x * 4, 0.0)).rgb
			* weight[4];
	result += texture(t_Source, v_TexCoord - vec2(tex_offset.x * 4, 0.0)).rgb
			* weight[4];

	o_Color = vec4(result, 1.0);
}
