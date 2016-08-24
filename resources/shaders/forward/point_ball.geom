#version 150 core

layout(triangles) in;
layout(triangle_strip, max_vertices = 3) out;

in VertexData {
	vec4 Position;
	vec3 Normal;
	mat3 TBN;
	vec2 TexCoord;
}v_In[3];

out VertexData {
	vec4 Position;
	vec3 Normal;
	mat3 TBN;
	vec2 TexCoord;
}v_Out;

void main() {
	for (int i = 0; i < 3; ++i) {
		gl_Position = gl_in[i].gl_Position;
		v_Out.Position = v_In[i].Position;
		v_Out.Normal = v_In[i].Normal;
		v_Out.TBN = v_In[i].TBN;
		v_Out.TexCoord = v_In[i].TexCoord;
		EmitVertex();
	}
	EndPrimitive();
}
