#version 150 core

#define MAX_NUM_SHAPES 256

layout (std140) uniform cb_CameraArgs {
	uniform mat4 u_Proj;
	uniform mat4 u_View;
};

struct Model {
    mat4 transform;
};

layout (std140) uniform u_ModelArgs {
	Model u_Model[MAX_NUM_SHAPES];
};

in vec3 a_Pos;
in vec3 a_Normal;
in vec3 a_Tangent;
in vec2 a_TexCoord;
in int a_PrimIndex;

out VertexData {
	vec4 Position;
	vec3 Normal;
	mat3 TBN;
	vec2 TexCoord;
	flat int PrimIndex;
}v_Out;

void main() {
	mat4 model4 = u_Model[a_PrimIndex].transform;
	mat3 model = mat3(model4);
	v_Out.Position = model4 * vec4(a_Pos, 1.0);
	vec3 normal = normalize(model * a_Normal);

	v_Out.Normal = normal;
	vec3 tangent = normalize(model * a_Tangent);
	vec3 bitangent = cross(normal, tangent);

	v_Out.TBN = mat3(tangent, bitangent, normal);

	v_Out.TexCoord = a_TexCoord;
	gl_Position = u_Proj * u_View * v_Out.Position;
}

