export function webglFixture(fail: { shader?: number; program?: boolean; link?: boolean; buffer?: number } = {}) {
  const calls: Record<string, number> = {};
  const liveBuffers = new Set<object>(), livePrograms = new Set<object>(), liveShaders = new Set<object>();
  const scissors: number[][] = [];
  let lost = false;
  const count = (key: string) => { calls[key] = (calls[key] ?? 0) + 1; };
  const gl = {
    VERTEX_SHADER: 1, FRAGMENT_SHADER: 2, COMPILE_STATUS: 3, LINK_STATUS: 4,
    ARRAY_BUFFER: 5, STATIC_DRAW: 6, FLOAT: 7, COLOR_BUFFER_BIT: 8, BLEND: 9,
    SRC_ALPHA: 10, ONE_MINUS_SRC_ALPHA: 11, POINTS: 12, LINES: 13, SCISSOR_TEST: 14,
    createShader(type: number) { count("createShader"); const shader = { type }; liveShaders.add(shader); return shader; },
    shaderSource() {}, compileShader() { count("compileShader"); },
    getShaderParameter(shader: { type: number }) { return shader.type !== fail.shader; },
    deleteShader(shader: object) { count("deleteShader"); liveShaders.delete(shader); },
    createProgram() { count("createProgram"); if (fail.program) return null; const program = {}; livePrograms.add(program); return program; },
    attachShader() {}, linkProgram() { count("linkProgram"); }, getProgramParameter() { return !fail.link; },
    deleteProgram(program: object) { count("deleteProgram"); livePrograms.delete(program); },
    createBuffer() { count("createBuffer"); if (calls.createBuffer === fail.buffer) return null; const buffer = {}; liveBuffers.add(buffer); return buffer; },
    deleteBuffer(buffer: object) { count("deleteBuffer"); liveBuffers.delete(buffer); },
    getAttribLocation() { return 0; }, getUniformLocation() { return {}; },
    bindBuffer() {}, bufferData() { count("bufferData"); },
    enableVertexAttribArray() {}, disableVertexAttribArray() {}, vertexAttribPointer() {}, vertexAttrib1f() {},
    viewport() { count("viewport"); }, clearColor() {}, clear() {}, enable() {}, disable() {},
    scissor(...rect: number[]) { scissors.push(rect); }, blendFunc() {}, useProgram() {},
    uniform4f() {}, uniform3f() {}, uniform2f() {}, uniform1f() {},
    drawArrays() { count("drawArrays"); }, isContextLost() { return lost; },
  };
  return { gl: gl as unknown as WebGLRenderingContext, calls, scissors, liveBuffers, livePrograms, liveShaders, lose: () => { lost = true; } };
}
