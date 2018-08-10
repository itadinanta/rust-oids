// unlit.vert
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
	vec3 BaryCoord;
	flat int PrimIndex;
}v_Out;

void main() {
	mat4 model4 = u_Model[a_PrimIndex].transform;
	v_Out.Position = model4 * vec4(a_Pos, 1.0);

	v_Out.Normal = vec3(1,0,0);
	v_Out.TBN =  mat3(1);
	v_Out.BaryCoord = vec3(1/3.,1/3.,1/3.);
	v_Out.TexCoord = a_TexCoord;
	v_Out.PrimIndex = a_PrimIndex;

	gl_Position = u_Proj * u_View * v_Out.Position;
}
