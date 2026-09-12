import { TRUSS3D_PROJECTION, VIEWPORT_CLIP, type CameraState, type ProjectionMode } from "./workbench-viewport-core";
import type { SceneBufferSet } from "./workbench-truss3d-webgl-scene";

export type Truss3dDrawView = {
  projected3d: { minX: number; minZ: number; width: number; height: number };
  camera: CameraState;
  projectionMode: ProjectionMode;
};

const VERTEX_SHADER = `
attribute vec3 aPosition;
attribute vec4 aColor;
attribute float aPointSize;
uniform vec4 uBounds;
uniform vec3 uCamera;
uniform vec2 uPan;
uniform float uPerspective;
varying vec4 vColor;

void main() {
  float cy = cos(uCamera.x);
  float sy = sin(uCamera.x);
  float cp = cos(uCamera.y);
  float sp = sin(uCamera.y);
  float yawX = aPosition.x * cy - aPosition.y * sy;
  float yawY = aPosition.x * sy + aPosition.y * cy;
  float pitchY = yawY * cp - aPosition.z * sp;
  float pitchZ = yawY * sp + aPosition.z * cp;
  float depth = uPerspective > 0.5 ? 8.0 / (8.0 + pitchY) : 1.0;
  float scale = min(${TRUSS3D_PROJECTION.width}.0 / uBounds.z, ${TRUSS3D_PROJECTION.height}.0 / uBounds.w) * uCamera.z * depth;
  float screenX = ${TRUSS3D_PROJECTION.centerX}.0 + (yawX - uBounds.x - uBounds.z * 0.5) * scale + uPan.x;
  float screenY = ${TRUSS3D_PROJECTION.centerY}.0 - (pitchZ - uBounds.y - uBounds.w * 0.5) * scale + uPan.y;
  gl_Position = vec4((screenX / 980.0) * 2.0 - 1.0, 1.0 - (screenY / 460.0) * 2.0, 0.0, 1.0);
  gl_PointSize = aPointSize;
  vColor = aColor;
}`;
const FRAGMENT_SHADER = `precision mediump float; varying vec4 vColor; void main() { gl_FragColor = vColor; }`;

function createShader(gl: WebGLRenderingContext, type: number, source: string) {
  const shader = gl.createShader(type);
  if (!shader) return null;
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    gl.deleteShader(shader);
    return null;
  }
  return shader;
}

export function createTruss3dWebglRenderer(gl: WebGLRenderingContext) {
  const vertex = createShader(gl, gl.VERTEX_SHADER, VERTEX_SHADER);
  if (!vertex) return null;
  const fragment = createShader(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER);
  if (!fragment) { gl.deleteShader(vertex); return null; }
  const program = gl.createProgram();
  if (program) {
    gl.attachShader(program, vertex);
    gl.attachShader(program, fragment);
    gl.linkProgram(program);
  }
  gl.deleteShader(vertex);
  gl.deleteShader(fragment);
  if (!program) return null;
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) { gl.deleteProgram(program); return null; }

  const keys = [
    "linePositions", "lineColors", "deformedLinePositions", "deformedLineColors",
    "nodePositions", "nodeColors", "nodeSizes", "deformedNodePositions", "deformedNodeColors",
  ] as const;
  const buffers = new Map<(typeof keys)[number], WebGLBuffer>();
  for (const key of keys) {
    const buffer = gl.createBuffer();
    if (!buffer) {
      for (const previous of buffers.values()) gl.deleteBuffer(previous);
      gl.deleteProgram(program);
      return null;
    }
    buffers.set(key, buffer);
  }
  const aPosition = gl.getAttribLocation(program, "aPosition");
  const aColor = gl.getAttribLocation(program, "aColor");
  const aPointSize = gl.getAttribLocation(program, "aPointSize");
  const uBounds = gl.getUniformLocation(program, "uBounds");
  const uCamera = gl.getUniformLocation(program, "uCamera");
  const uPan = gl.getUniformLocation(program, "uPan");
  const uPerspective = gl.getUniformLocation(program, "uPerspective");
  let uploadedScene: SceneBufferSet | null = null;
  let disposed = false;

  const bindAttribute = (key: (typeof keys)[number], attribute: number, size: number) => {
    gl.bindBuffer(gl.ARRAY_BUFFER, buffers.get(key)!);
    gl.enableVertexAttribArray(attribute);
    gl.vertexAttribPointer(attribute, size, gl.FLOAT, false, 0, 0);
  };

  return {
    draw(scene: SceneBufferSet, view: Truss3dDrawView, width: number, height: number) {
      if (disposed || gl.isContextLost()) return;
      // Camera/resize updates only change uniforms. Geometry uploads follow immutable scene identity.
      if (uploadedScene !== scene) {
        for (const key of keys) {
          gl.bindBuffer(gl.ARRAY_BUFFER, buffers.get(key)!);
          gl.bufferData(gl.ARRAY_BUFFER, scene[key], gl.STATIC_DRAW);
        }
        uploadedScene = scene;
      }
      gl.viewport(0, 0, width, height);
      gl.disable(gl.SCISSOR_TEST);
      gl.clearColor(0.05, 0.06, 0.08, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      // Match the SVG picking region so GPU geometry cannot cover viewport chrome.
      gl.enable(gl.SCISSOR_TEST);
      gl.scissor(
        Math.round(VIEWPORT_CLIP.x / 980 * width),
        Math.round((460 - VIEWPORT_CLIP.y - VIEWPORT_CLIP.height) / 460 * height),
        Math.round(VIEWPORT_CLIP.width / 980 * width),
        Math.round(VIEWPORT_CLIP.height / 460 * height),
      );
      gl.enable(gl.BLEND);
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
      gl.useProgram(program);
      gl.uniform4f(uBounds, view.projected3d.minX, view.projected3d.minZ, view.projected3d.width, view.projected3d.height);
      gl.uniform3f(uCamera, view.camera.yaw, view.camera.pitch, view.camera.zoom);
      gl.uniform2f(uPan, view.camera.panX, view.camera.panY);
      gl.uniform1f(uPerspective, view.projectionMode === "persp" ? 1 : 0);

      for (const [position, color, points] of [
        ["linePositions", "lineColors", false],
        ["deformedLinePositions", "deformedLineColors", false],
        ["nodePositions", "nodeColors", true],
        ["deformedNodePositions", "deformedNodeColors", true],
      ] as const) {
        const count = scene[position].length / 3;
        if (count === 0) continue;
        bindAttribute(position, aPosition, 3);
        bindAttribute(color, aColor, 4);
        if (points) bindAttribute("nodeSizes", aPointSize, 1);
        else { gl.disableVertexAttribArray(aPointSize); gl.vertexAttrib1f(aPointSize, 1); }
        gl.drawArrays(points ? gl.POINTS : gl.LINES, 0, count);
      }
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      uploadedScene = null;
      for (const buffer of buffers.values()) gl.deleteBuffer(buffer);
      buffers.clear();
      gl.deleteProgram(program);
    },
  };
}
