import { useEffect, useRef } from "react";
import fragmentSource from "./gradient.frag?raw";

type Rgb = [number, number, number];

export type ShaderGradientProps = {
  colors: [string, string, string];
  density?: number;
  strength?: number;
  speed?: number;
  /** Plane rotation in degrees. */
  rotation?: number;
  offset?: [number, number];
  brightness?: number;
  reflection?: number;
  /** Canvas pixels per CSS pixel. */
  resolution?: number;
  /** Holds the current frame, e.g. while a game is running. */
  paused?: boolean;
  fps?: number;
  className?: string;
};

type Uniforms = Omit<ShaderGradientProps, "resolution" | "fps" | "className" | "paused">;

const vertexSource = `
attribute vec2 aPosition;
void main() { gl_Position = vec4(aPosition, 0.0, 1.0); }
`;

const UNIFORMS = [
  "uResolution",
  "uTime",
  "uLoad",
  "uColor1",
  "uColor2",
  "uColor3",
  "uDensity",
  "uStrength",
  "uRotation",
  "uOffset",
  "uBrightness",
  "uReflection",
] as const;

// The surface starts flat and folds in over this long when the canvas mounts.
const LOAD_MS = 1200;

function hexToRgb(hex: string): Rgb {
  const value = parseInt(hex.slice(1), 16);
  return [((value >> 16) & 255) / 255, ((value >> 8) & 255) / 255, (value & 255) / 255];
}

function compile(gl: WebGLRenderingContext, type: number, source: string) {
  const shader = gl.createShader(type);
  if (!shader) return null;
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.error("ShaderGradient:", gl.getShaderInfoLog(shader));
    gl.deleteShader(shader);
    return null;
  }
  return shader;
}

function createProgram(gl: WebGLRenderingContext) {
  const vertex = compile(gl, gl.VERTEX_SHADER, vertexSource);
  const fragment = compile(gl, gl.FRAGMENT_SHADER, fragmentSource);
  const program = gl.createProgram();
  if (!vertex || !fragment || !program) return null;
  gl.attachShader(program, vertex);
  gl.attachShader(program, fragment);
  gl.linkProgram(program);
  gl.deleteShader(vertex);
  gl.deleteShader(fragment);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    console.error("ShaderGradient:", gl.getProgramInfoLog(program));
    gl.deleteProgram(program);
    return null;
  }
  return program;
}

export function ShaderGradient({
  colors,
  density = 1.3,
  strength = 2,
  speed = 0.06,
  rotation = 50,
  offset = [-1.4, 0],
  brightness = 1,
  reflection = 0.1,
  resolution = 0.5,
  fps = 30,
  paused = false,
  className,
}: ShaderGradientProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const uniformsRef = useRef<Uniforms>({ colors });
  const redrawRef = useRef<(() => void) | null>(null);
  const pausedRef = useRef(paused);
  const restartRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    uniformsRef.current = { colors, density, strength, speed, rotation, offset, brightness, reflection };
    redrawRef.current?.();
  });

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    let gl: WebGLRenderingContext | null = null;
    let program: WebGLProgram | null = null;
    let buffer: WebGLBuffer | null = null;
    const locations: Partial<Record<(typeof UNIFORMS)[number], WebGLUniformLocation | null>> = {};
    let frame = 0;
    let startedAt = 0;
    let lastFrameAt = 0;
    let time = 0;

    function init() {
      // Vizier runs next to the game, so stay off the discrete GPU where possible.
      gl = canvas!.getContext("webgl", {
        alpha: false,
        antialias: false,
        depth: false,
        stencil: false,
        powerPreference: "low-power",
      });
      if (!gl) return false;
      program = createProgram(gl);
      if (!program) return false;
      gl.useProgram(program);
      buffer = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
      const position = gl.getAttribLocation(program, "aPosition");
      gl.enableVertexAttribArray(position);
      gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
      for (const name of UNIFORMS) locations[name] = gl.getUniformLocation(program, name);
      return true;
    }

    function draw(load: number) {
      if (!gl || !program) return;
      const u = uniformsRef.current;
      const [c1, c2, c3] = u.colors.map(hexToRgb);
      gl.viewport(0, 0, canvas!.width, canvas!.height);
      gl.uniform2f(locations.uResolution!, canvas!.width, canvas!.height);
      gl.uniform1f(locations.uTime!, time);
      gl.uniform1f(locations.uLoad!, load);
      gl.uniform3fv(locations.uColor1!, c1);
      gl.uniform3fv(locations.uColor2!, c2);
      gl.uniform3fv(locations.uColor3!, c3);
      gl.uniform1f(locations.uDensity!, u.density!);
      gl.uniform1f(locations.uStrength!, u.strength!);
      gl.uniform1f(locations.uRotation!, (u.rotation! * Math.PI) / 180);
      gl.uniform2fv(locations.uOffset!, u.offset!);
      gl.uniform1f(locations.uBrightness!, u.brightness!);
      gl.uniform1f(locations.uReflection!, u.reflection!);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    }

    function tick(now: number) {
      frame = requestAnimationFrame(tick);
      if (!startedAt) startedAt = lastFrameAt = now;
      const elapsed = now - lastFrameAt;
      if (elapsed < 1000 / fps - 2) return;
      lastFrameAt = now;
      // Clamp so a throttled or minimised window does not jump ahead on resume.
      time += (Math.min(elapsed, 100) / 1000) * uniformsRef.current.speed!;
      const t = Math.min((now - startedAt) / LOAD_MS, 1);
      draw(1 - Math.pow(1 - t, 3));
    }

    function start() {
      cancelAnimationFrame(frame);
      if (reducedMotion.matches || pausedRef.current) {
        redrawRef.current = () => draw(1);
        draw(1);
      } else {
        redrawRef.current = null;
        frame = requestAnimationFrame(tick);
      }
    }

    const observer = new ResizeObserver(() => {
      canvas.width = Math.max(1, Math.round(canvas.clientWidth * resolution));
      canvas.height = Math.max(1, Math.round(canvas.clientHeight * resolution));
      redrawRef.current?.();
    });

    function onContextLost(event: Event) {
      event.preventDefault();
      cancelAnimationFrame(frame);
      redrawRef.current = null;
      program = null;
    }

    function onContextRestored() {
      if (init()) start();
    }

    if (!init()) return;
    observer.observe(canvas);
    start();
    restartRef.current = start;
    reducedMotion.addEventListener("change", start);
    canvas.addEventListener("webglcontextlost", onContextLost);
    canvas.addEventListener("webglcontextrestored", onContextRestored);

    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      reducedMotion.removeEventListener("change", start);
      canvas.removeEventListener("webglcontextlost", onContextLost);
      canvas.removeEventListener("webglcontextrestored", onContextRestored);
      redrawRef.current = null;
      restartRef.current = null;
      if (gl && !gl.isContextLost()) {
        gl.deleteBuffer(buffer);
        gl.deleteProgram(program);
      }
    };
  }, [resolution, fps]);

  useEffect(() => {
    pausedRef.current = paused;
    restartRef.current?.();
  }, [paused]);

  return (
    <canvas
      ref={canvasRef}
      className={className}
      aria-hidden="true"
      // Shown until the first frame lands, and kept if WebGL is unavailable.
      style={{ background: `linear-gradient(120deg, ${colors[0]}, ${colors[1]})` }}
    />
  );
}
