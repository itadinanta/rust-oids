#version 150 core

layout(triangles) in;
layout(triangle_strip, max_vertices = 100) out;

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

struct V {
	vec4 GlPosition;
	vec4 Position;
	vec3 Normal;
	mat3 TBN;
	vec2 TexCoord;
};

void emit_vertex(V v) {
	gl_Position = v.GlPosition;
	v_Out.Position = v.Position;
	v_Out.Normal = v.Normal;
	v_Out.TBN = v.TBN;
	v_Out.TexCoord = v.TexCoord;
	EmitVertex();
}

const float PI = 3.1415926535897932384626433832795;
const float PI_2 = PI / 2;

void main() {
	V verts[4];
	for (int i = 0; i < 3; ++i) {
		verts[i].GlPosition = gl_in[i].gl_Position;
		verts[i].Position = v_In[i].Position;
		verts[i].TexCoord = v_In[i].TexCoord;
		verts[i].Normal = v_In[i].Normal;
		verts[i].TBN = v_In[i].TBN;
	}

	V o = verts[0];
	V u = verts[1];
	V v = verts[2];

	V u0 = u;
	u.GlPosition -= o.GlPosition;
	u.Position -= o.Position;
	u.TexCoord -= o.TexCoord;

	V v0 = v;
	v.GlPosition -= o.GlPosition;
	v.Position -= o.Position;
	v.TexCoord -= o.TexCoord;

	int n = 32;
	float d = 2 * PI / n;

	vec2 unit = vec2(1, 0);
	V prev;
	V hand = u0;
	mat2 r = mat2(cos(d), -sin(d), sin(d), cos(d));
	for (int i = 0; i < n; ++i) {
		prev = hand;
		unit = r * unit;

		hand.GlPosition = unit.x * u.GlPosition + unit.y * v.GlPosition
				+ o.GlPosition;
		hand.Position = unit.x * u.Position + unit.y * v.Position + o.Position;
		hand.TexCoord = unit.x * u.TexCoord + unit.y * v.TexCoord + o.TexCoord;

		emit_vertex(o);
		emit_vertex(prev);
		emit_vertex(hand);

		EndPrimitive();
	}
}
