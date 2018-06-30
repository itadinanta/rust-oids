#version 150 core

layout(triangles) in;
layout(triangle_strip, max_vertices = 30) out;

in VertexData {
	vec4 Position;
	vec3 Normal;
	mat3 TBN;
	vec2 TexCoord;
	vec3 BaryCoord;
	flat int PrimIndex;
}v_In[3];

out VertexData {
	vec4 Position;
	vec3 Normal;
	mat3 TBN;
	vec2 TexCoord;
	vec3 BaryCoord;
	flat int PrimIndex;
}v_Out;

struct V {
	vec4 GlPosition;
	vec4 Position;
	vec3 Normal;
	mat3 TBN;
	vec2 TexCoord;
	vec3 BaryCoord;
	int PrimIndex;
};

void emit_vertex(V v) {
	gl_Position = v.GlPosition;
	v_Out.Position = v.Position;
	v_Out.Normal = v.Normal;
	v_Out.TBN = v.TBN;
	v_Out.TexCoord = v.TexCoord;
	v_Out.BaryCoord = v.BaryCoord;
	v_Out.PrimIndex = v.PrimIndex;
	EmitVertex();
}

const float PI = 3.1415926535897932384626433832795;
const int N = 15;
const float D = 2 * PI / N;
const mat2 R = mat2(cos(D), -sin(D), sin(D), cos(D));

V read_vert(int i) {
	V result;
	result.GlPosition = gl_in[i].gl_Position;
	result.Position = v_In[i].Position;
	result.TexCoord = v_In[i].TexCoord;
	result.Normal = v_In[i].Normal;
	result.TBN = v_In[i].TBN;
	result.BaryCoord = v_In[i].BaryCoord;
	result.PrimIndex = v_In[i].PrimIndex;
	return result;
}

void main() {
	// triangle o, u, v -> (o, u) and (o, v) are a vector
	// basis for local "ball space"
	V o = read_vert(0);
	V u = read_vert(1);
	V v = read_vert(2);

	V u0 = u;
	u.GlPosition -= o.GlPosition;
	u.Position -= o.Position;
	u.TexCoord -= o.TexCoord;

	v.GlPosition -= o.GlPosition;
	v.Position -= o.Position;
	v.TexCoord -= o.TexCoord;

	// we use a clock "hand" and go round the circle,
	// rotating the "hand" counterclockwise
	// of D radians, D = 2*pi/N, each step
	vec2 unit = vec2(1, 0);
	V prev;
	V hand = u0;
	// we build a triangle fan of N triangles using (o, hand(i), hand(i-1))
	// as the vertices.
	for (int i = 0; i < N; ++i) {
		prev = hand;
		unit = R * unit;

		hand.GlPosition = unit.x * u.GlPosition + unit.y * v.GlPosition
				+ o.GlPosition;
		hand.Position = unit.x * u.Position + unit.y * v.Position + o.Position;
		hand.TexCoord = unit.x * u.TexCoord + unit.y * v.TexCoord + o.TexCoord;

		if (i % 3 != 0) {
			emit_vertex(o);
			emit_vertex(prev);
			emit_vertex(hand);
			EndPrimitive();
		}
	}
}
