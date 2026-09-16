/** 首页装饰专用：背景完全自绘，不把 DOM、对象字段或屏幕截图上传为纹理。 */
export interface LiquidAppearance {
  dark: boolean;
  reduced: boolean;
  container: string;
  accent: string;
  secondary: string;
  tertiary: string;
}

export interface LiquidRenderer {
  update: (appearance: LiquidAppearance) => void;
  dispose: () => void;
}

const vertexSource = 'attribute vec2 position; void main(){gl_Position=vec4(position,0.,1.);}';
const fragmentSource = `precision highp float;
uniform vec2 resolution; uniform vec2 pointer; uniform float dark;
uniform vec3 container; uniform vec3 accent; uniform vec3 secondary; uniform vec3 tertiary;
float box(vec2 p,vec2 b,float r){vec2 q=abs(p)-b+r;return min(max(q.x,q.y),0.)+length(max(q,0.))-r;}
vec3 scene(vec2 uv){
  vec3 base=container;
  float cool=exp(-dot((uv-vec2(.82,.85))*2.,(uv-vec2(.82,.85))*2.));
  float warm=exp(-dot((uv-vec2(.98,.08))*2.3,(uv-vec2(.98,.08))*2.3));
  base=mix(base,accent,cool*mix(.24,.16,dark));
  base=mix(base,tertiary,warm*.68);
  base=mix(base,secondary,exp(-dot((uv-vec2(.5,.1))*3.,(uv-vec2(.5,.1))*3.))*.25);
  // 用柔和光带呈现折射，左侧平缓过渡到主题色底，保证前景文字的对比度。
  float wave=.28+sin(uv.x*4.4)*.085;
  float ribbonDistance=(uv.y-wave)/.045;
  float strandDistance=(uv.y-wave-.025)/.005;
  float ribbon=exp(-ribbonDistance*ribbonDistance);
  float strand=exp(-strandDistance*strandDistance);
  base=mix(base,mix(vec3(1.),accent,dark*.6),(ribbon*.16+strand*.2)*smoothstep(.35,.72,uv.x));
  return mix(container,base,smoothstep(.25,.72,uv.x));
}
void main(){
  vec2 uv=gl_FragCoord.xy/resolution;
  float aspect=resolution.x/resolution.y;
  // 装饰与盾牌固定在同一中心，触摸只改变光线和折射，不挤占文字区域。
  vec2 center=vec2(.78,.5);
  vec2 p=(uv-center)*vec2(aspect,1.);
  float halfWidth=min(.29,aspect*.155);
  vec2 size=vec2(halfWidth,halfWidth*1.14);float radius=halfWidth*.6;
  float d=box(p,size,radius);float aa=1.7/resolution.y;
  float mask=1.-smoothstep(-aa,aa,d);float eps=.002;
  vec2 n=normalize(vec2(box(p+vec2(eps,0),size,radius)-box(p-vec2(eps,0),size,radius),box(p+vec2(0,eps),size,radius)-box(p-vec2(0,eps),size,radius))+vec2(.00001));
  float edge=exp(-abs(d)*38.);float lens=max(0.,1.-length(p/size));
  vec2 offset=(n*edge*.055+p*.08+lens*p*.1+(pointer-vec2(.5))*.006)/vec2(aspect,1.);
  vec2 sampleUV=uv-offset;float dispersion=.002*edge;
  vec3 refract=vec3(scene(sampleUV+vec2(dispersion,0)).r,scene(sampleUV).g,scene(sampleUV-vec2(dispersion,0)).b);
  refract=mix(refract,mix(vec3(1.),container,dark),mix(.12,.08,dark));
  vec2 light=normalize(vec2(-.55,.85)+(pointer-vec2(.5))*.35);
  float highlight=pow(max(0.,dot(n,light)),3.);
  float rim=exp(-abs(d+aa)*180.);
  refract+=vec3(.92,.97,1.)*edge*highlight*mix(.16,.1,dark);
  refract+=vec3(.94,.98,1.)*rim*(.09+highlight*.48);
  refract-=vec3(.055,.065,.08)*edge*(1.-highlight)*.55;
  vec3 bg=scene(uv);
  float shadow=exp(-max(0.,box(p+vec2(0.,.035),size,radius))*32.)*(1.-mask);
  bg*=1.-shadow*.12;gl_FragColor=vec4(mix(bg,refract,mask),1.);
}`;

function rgb(hex: string): [number, number, number] {
  return [1, 3, 5].map((index) => parseInt(hex.slice(index, index + 2), 16) / 255) as [
    number,
    number,
    number,
  ];
}

export function createAndroidLiquidRenderer(
  canvas: HTMLCanvasElement,
  initial: LiquidAppearance,
  onAvailability: (available: boolean) => void,
): LiquidRenderer | null {
  const gl = canvas.getContext('webgl', {
    alpha: false,
    antialias: false,
    preserveDrawingBuffer: false,
    powerPreference: 'low-power',
  });
  if (!gl) {
    onAvailability(false);
    return null;
  }
  let appearance = initial;
  let program: WebGLProgram | null = null;
  let buffer: WebGLBuffer | null = null;
  let shaders: WebGLShader[] = [];
  let uniforms: Record<string, WebGLUniformLocation | null> = {};
  let disposed = false;
  let lost = false;
  let visible = false;
  let frame = 0;
  let point: [number, number] = [0.5, 0.5];

  function clearResources() {
    if (program) gl!.deleteProgram(program);
    if (buffer) gl!.deleteBuffer(buffer);
    shaders.forEach((shader) => gl!.deleteShader(shader));
    program = null;
    buffer = null;
    shaders = [];
  }
  function initialize() {
    clearResources();
    try {
      for (const [type, source] of [
        [gl!.VERTEX_SHADER, vertexSource],
        [gl!.FRAGMENT_SHADER, fragmentSource],
      ] as const) {
        const shader = gl!.createShader(type);
        if (!shader) throw new Error('No shader');
        shaders.push(shader);
        gl!.shaderSource(shader, source);
        gl!.compileShader(shader);
        if (!gl!.getShaderParameter(shader, gl!.COMPILE_STATUS))
          throw new Error('Shader compile failed');
      }
      program = gl!.createProgram();
      buffer = gl!.createBuffer();
      if (!program || !buffer) throw new Error('No graphics resources');
      shaders.forEach((shader) => gl!.attachShader(program!, shader));
      gl!.linkProgram(program);
      if (!gl!.getProgramParameter(program, gl!.LINK_STATUS)) throw new Error('Shader link failed');
      gl!.useProgram(program);
      gl!.bindBuffer(gl!.ARRAY_BUFFER, buffer);
      gl!.bufferData(
        gl!.ARRAY_BUFFER,
        new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
        gl!.STATIC_DRAW,
      );
      const position = gl!.getAttribLocation(program, 'position');
      gl!.enableVertexAttribArray(position);
      gl!.vertexAttribPointer(position, 2, gl!.FLOAT, false, 0, 0);
      uniforms = Object.fromEntries(
        ['resolution', 'pointer', 'dark', 'container', 'accent', 'secondary', 'tertiary'].map(
          (key) => [key, gl!.getUniformLocation(program!, key)],
        ),
      );
      return true;
    } catch {
      clearResources();
      onAvailability(false);
      return false;
    }
  }
  function stop() {
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
  }
  function draw() {
    frame = 0;
    if (disposed || lost || !visible || document.hidden || !program) return;
    const rect = canvas.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    // 限制绘制像素数；不按高密度屏的全部物理分辨率渲染装饰。
    const scale = Math.min(devicePixelRatio || 1, 1.5, 900 / rect.width);
    const width = Math.max(1, Math.round(rect.width * scale)),
      height = Math.max(1, Math.round(rect.height * scale));
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }
    gl!.viewport(0, 0, width, height);
    gl!.uniform2f(uniforms.resolution, width, height);
    gl!.uniform2f(uniforms.pointer, ...point);
    gl!.uniform1f(uniforms.dark, appearance.dark ? 1 : 0);
    for (const key of ['container', 'accent', 'secondary', 'tertiary'] as const)
      gl!.uniform3f(uniforms[key], ...rgb(appearance[key]));
    gl!.drawArrays(gl!.TRIANGLES, 0, 6);
    onAvailability(true);
    // 不循环申请下一帧：只有输入、尺寸、主题或可见性变化才重绘。
  }
  function invalidate() {
    if (!frame && !disposed && !lost && visible && !document.hidden && program)
      frame = requestAnimationFrame(draw);
  }
  function move(event: PointerEvent) {
    if (appearance.reduced) return;
    const rect = canvas.getBoundingClientRect();
    point = [
      Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width)),
      Math.max(0, Math.min(1, 1 - (event.clientY - rect.top) / rect.height)),
    ];
    invalidate();
  }
  function visibility() {
    if (document.hidden) stop();
    else invalidate();
  }
  function contextLost(event: Event) {
    event.preventDefault();
    lost = true;
    stop();
    onAvailability(false);
  }
  function contextRestored() {
    lost = false;
    if (initialize()) invalidate();
  }
  if (!initialize()) return null;
  const intersection = new IntersectionObserver((entries) => {
    visible = entries[0].isIntersecting;
    if (visible) invalidate();
    else stop();
  });
  const resize = new ResizeObserver(invalidate);
  intersection.observe(canvas);
  resize.observe(canvas);
  canvas.addEventListener('pointermove', move);
  canvas.addEventListener('pointerdown', move);
  canvas.addEventListener('webglcontextlost', contextLost);
  canvas.addEventListener('webglcontextrestored', contextRestored);
  document.addEventListener('visibilitychange', visibility);
  return {
    update(next) {
      appearance = next;
      if (next.reduced) point = [0.5, 0.5];
      invalidate();
    },
    dispose() {
      disposed = true;
      stop();
      intersection.disconnect();
      resize.disconnect();
      canvas.removeEventListener('pointermove', move);
      canvas.removeEventListener('pointerdown', move);
      canvas.removeEventListener('webglcontextlost', contextLost);
      canvas.removeEventListener('webglcontextrestored', contextRestored);
      document.removeEventListener('visibilitychange', visibility);
      clearResources();
      // StrictMode 和路由重挂载会重新使用同一 canvas；释放 GPU 资源，不主动永久丢弃上下文。
    },
  };
}
