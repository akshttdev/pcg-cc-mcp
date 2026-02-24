uniform float time;
uniform float scanlineSpeed;
uniform float opacity;
uniform vec3 color;

varying vec2 vUv;
varying vec3 vNormal;
varying vec3 vPosition;

void main() {
  // Scanline effect
  float scanline = sin(vPosition.y * 10.0 - time * scanlineSpeed) * 0.5 + 0.5;

  // Fresnel effect (edge glow)
  vec3 viewDirection = normalize(cameraPosition - vPosition);
  float fresnel = pow(1.0 - abs(dot(viewDirection, vNormal)), 2.0);

  // Flickering
  float flicker = sin(time * 3.0) * 0.05 + 0.95;

  // Combine effects
  float alpha = (scanline * 0.3 + fresnel * 0.7) * opacity * flicker;
  vec3 finalColor = color * (1.0 + fresnel * 0.5);

  gl_FragColor = vec4(finalColor, alpha);
}
