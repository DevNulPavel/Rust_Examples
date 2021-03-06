
cbuffer ConstantBuffer : register(b0)
{
	matrix u_transform;
	float u_flip;
};

struct sVSInput
{
    float4 pos : POSITION;
	float4 nor : NORMAL;
    float2 tex : TEXCOORD0;
};

struct sPSInput
{
    float4 pos : SV_POSITION;
    float2 tex : TEXCOORD0;
};

sPSInput main(sVSInput input)
{
    sPSInput output;
	output.pos = mul(u_transform, float4(input.pos.x * u_flip, input.pos.yzw));
	output.tex = input.tex;
    return output;
}

