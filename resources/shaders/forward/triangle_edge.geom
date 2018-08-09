// triangle_edge.geom
#version 150 core

layout(triangles) in;
layout(triangle_strip, max_vertices=9) out;

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

V read_vert(int i) {
	V result;
	result.GlPosition = gl_in[i].gl_Position;
	result.Position = v_In[i].Position;
	result.TexCoord = v_In[i].TexCoord;
	result.Normal = v_In[i].Normal;
	result.TBN = v_In[i].TBN;
	result.PrimIndex = v_In[i].PrimIndex;
	return result;
}

const float SCALE = 1.1;
const float SCALE_UV = 1.1;

void main() {
	V o = read_vert(0);
	V u = read_vert(1);
	V v = read_vert(2);

	V bc;
	bc.Position = (o.Position + u.Position + v.Position) / 3.0;
	bc.GlPosition = (o.GlPosition + u.GlPosition + v.GlPosition) / 3.0;
	bc.Normal = (o.Normal + u.Normal + v.Normal) / 3.0;
	bc.TBN = o.TBN;
	bc.TexCoord = (o.TexCoord + u.TexCoord + v.TexCoord) / 3.0;
	bc.PrimIndex = o.PrimIndex;

	u.GlPosition.xy = (u.GlPosition.xy - o.GlPosition.xy) * SCALE + o.GlPosition.xy;
	v.GlPosition.xy = (v.GlPosition.xy - o.GlPosition.xy) * SCALE + o.GlPosition.xy;

	u.TexCoord = (u.TexCoord - o.TexCoord) * SCALE_UV + o.TexCoord;
	v.TexCoord = (v.TexCoord - o.TexCoord) * SCALE_UV + o.TexCoord;

	o.BaryCoord = vec3(1,0,0);
	u.BaryCoord = vec3(0,1,0);
	v.BaryCoord = vec3(0,0,1);

	emit_vertex(o);
	emit_vertex(u);
	emit_vertex(v);
	EndPrimitive();
}
