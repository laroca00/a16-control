// Animated backdrop: a domain-warped noise field read as a lit, folded
// surface and coloured across three tones by height.

precision highp float;

uniform vec2 uResolution;
uniform float uTime;
uniform float uLoad;
uniform vec3 uColor1;
uniform vec3 uColor2;
uniform vec3 uColor3;
uniform float uDensity;
uniform float uStrength;
uniform float uRotation;
uniform vec2 uOffset;
uniform float uBrightness;
uniform float uReflection;

float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float valueNoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float fbm(vec2 p) {
  float sum = 0.0;
  float amp = 0.5;
  mat2 turn = mat2(1.6, 1.2, -1.2, 1.6);
  for (int k = 0; k < 4; k++) {
    sum += amp * valueNoise(p);
    p = turn * p;
    amp *= 0.5;
  }
  return sum;
}

// Height of the surface; uLoad eases it in from flat.
float surface(vec2 p) {
  vec2 warp = vec2(fbm(p + vec2(0.0, uTime)), fbm(p + vec2(5.2, -0.7 * uTime)));
  return fbm(p + 1.8 * warp) * uStrength * uLoad;
}

void main() {
  vec2 uv = gl_FragCoord.xy / uResolution;
  vec2 p = (uv - 0.5) * vec2(uResolution.x / uResolution.y, 1.0);
  float c = cos(uRotation);
  float s = sin(uRotation);
  p = mat2(c, -s, s, c) * p * uDensity * 1.5 + uOffset * 0.25;

  float h = surface(p);
  float e = 0.01;
  float dx = (surface(p + vec2(e, 0.0)) - surface(p - vec2(e, 0.0))) / (2.0 * e);
  float dy = (surface(p + vec2(0.0, e)) - surface(p - vec2(0.0, e))) / (2.0 * e);
  vec3 normal = normalize(vec3(-dx * 0.25, -dy * 0.25, 1.0));
  vec3 light = normalize(vec3(-0.4, 0.5, 0.8));
  float diffuse = clamp(dot(normal, light), 0.0, 1.0);
  float spec = pow(max(dot(reflect(-light, normal), vec3(0.0, 0.0, 1.0)), 0.0), 24.0);

  float t = clamp(h / max(uStrength, 0.001) + (uv.x - 0.5) * 0.35, 0.0, 1.0);
  vec3 color = mix(uColor1, uColor2, smoothstep(0.2, 0.6, t));
  color = mix(color, uColor3, smoothstep(0.55, 0.95, t));
  color *= 0.65 + 0.5 * diffuse;
  color += spec * uReflection * 3.0;
  gl_FragColor = vec4(color * uBrightness, 1.0);
}
