// bench-blur.c - what SceneFX's window blur costs on this GPU.
//
// The numbers in docs/optimization.md section 7 come from this program. It
// needs no compositor and no display: it creates an EGL/GLES2 context on the
// Mali render node and times the exact pass chain SceneFX 0.5 runs, with
// glFinish() on both sides.
//
//   gcc -O2 -o bench-blur docs/bench-blur.c -lEGL -lGLESv2 -lm
//   ./bench-blur 1366 912
//
// The two fragment shaders that matter (blur1/blur2) are copied verbatim from
// scenefx 0.5 (render/fx_renderer/shaders/), including their mediump precision.
//
// Scenefx renders the down/up chain with the scaled region inside a full-size
// framebuffer; the shaded pixel count is what costs time, so each pass here
// renders into a target of the scaled size instead. The final full-screen draw
// of the blurred texture (what fx_render_pass_add_blur does after the chain) is
// included, because the optimized path is exactly that draw.
#define _GNU_SOURCE
#include <EGL/egl.h>
#include <EGL/eglext.h>
#include <GLES2/gl2.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static double now_ms(void) {
	struct timespec t;
	clock_gettime(CLOCK_MONOTONIC, &t);
	return t.tv_sec * 1e3 + t.tv_nsec / 1e6;
}

static const char *vs_src =
	"attribute vec2 pos;\n"
	"varying vec2 v_texcoord;\n"
	"void main() {\n"
	"  v_texcoord = pos * 0.5 + 0.5;\n"
	"  gl_Position = vec4(pos, 0.0, 1.0);\n"
	"}\n";

static const char *fs_plain =
	"precision mediump float;\n"
	"varying vec2 v_texcoord;\n"
	"uniform sampler2D tex;\n"
	"void main() { gl_FragColor = texture2D(tex, v_texcoord); }\n";

/* scenefx blur1.frag */
static const char *fs_blur1 =
	"precision mediump float;\n"
	"varying mediump vec2 v_texcoord;\n"
	"uniform sampler2D tex;\n"
	"uniform float radius;\n"
	"uniform vec2 halfpixel;\n"
	"void main() {\n"
	"  vec2 uv = v_texcoord * 2.0;\n"
	"  vec4 sum = texture2D(tex, uv) * 4.0;\n"
	"  sum += texture2D(tex, uv - halfpixel.xy * radius);\n"
	"  sum += texture2D(tex, uv + halfpixel.xy * radius);\n"
	"  sum += texture2D(tex, uv + vec2(halfpixel.x, -halfpixel.y) * radius);\n"
	"  sum += texture2D(tex, uv - vec2(halfpixel.x, -halfpixel.y) * radius);\n"
	"  gl_FragColor = sum / 8.0;\n"
	"}\n";

/* scenefx blur2.frag */
static const char *fs_blur2 =
	"precision mediump float;\n"
	"varying mediump vec2 v_texcoord;\n"
	"uniform sampler2D tex;\n"
	"uniform float radius;\n"
	"uniform vec2 halfpixel;\n"
	"void main() {\n"
	"  vec2 uv = v_texcoord / 2.0;\n"
	"  vec4 sum = texture2D(tex, uv + vec2(-halfpixel.x * 2.0, 0.0) * radius);\n"
	"  sum += texture2D(tex, uv + vec2(-halfpixel.x, halfpixel.y) * radius) * 2.0;\n"
	"  sum += texture2D(tex, uv + vec2(0.0, halfpixel.y * 2.0) * radius);\n"
	"  sum += texture2D(tex, uv + vec2(halfpixel.x, halfpixel.y) * radius) * 2.0;\n"
	"  sum += texture2D(tex, uv + vec2(halfpixel.x * 2.0, 0.0) * radius);\n"
	"  sum += texture2D(tex, uv + vec2(halfpixel.x, -halfpixel.y) * radius) * 2.0;\n"
	"  sum += texture2D(tex, uv + vec2(0.0, -halfpixel.y * 2.0) * radius);\n"
	"  sum += texture2D(tex, uv + vec2(-halfpixel.x, -halfpixel.y) * radius) * 2.0;\n"
	"  gl_FragColor = sum / 12.0;\n"
	"}\n";

/* scenefx blur_effects.frag, the noise/tint pass */
static const char *fs_effects =
	"precision mediump float;\n"
	"varying vec2 v_texcoord;\n"
	"uniform sampler2D tex;\n"
	"uniform float brightness, contrast, saturation, noise;\n"
	"void main() {\n"
	"  vec4 c = texture2D(tex, v_texcoord);\n"
	"  c.rgb = (c.rgb - 0.5) * contrast + 0.5;\n"
	"  float l = dot(c.rgb, vec3(0.3086, 0.6094, 0.0820));\n"
	"  c.rgb = mix(vec3(l), c.rgb, saturation);\n"
	"  c.rgb *= brightness;\n"
	"  vec3 p3 = fract(vec3(v_texcoord.xyx) * 1689.1984);\n"
	"  p3 += dot(p3, p3.yzx + 33.33);\n"
	"  float h = fract((p3.x + p3.y) * p3.z);\n"
	"  c.rgb += (mod(h, 1.0) - 0.5) * noise;\n"
	"  gl_FragColor = c;\n"
	"}\n";

typedef struct { GLuint prog, tex, fbo; int w, h; } target;

static GLuint compile(GLenum type, const char *src) {
	GLuint s = glCreateShader(type);
	glShaderSource(s, 1, &src, NULL);
	glCompileShader(s);
	GLint ok;
	glGetShaderiv(s, GL_COMPILE_STATUS, &ok);
	if (!ok) {
		char log[4096];
		glGetShaderInfoLog(s, sizeof log, NULL, log);
		fprintf(stderr, "shader: %s\n", log);
		exit(1);
	}
	return s;
}

static GLuint program(const char *fs) {
	GLuint p = glCreateProgram();
	glAttachShader(p, compile(GL_VERTEX_SHADER, vs_src));
	glAttachShader(p, compile(GL_FRAGMENT_SHADER, fs));
	glBindAttribLocation(p, 0, "pos");
	glLinkProgram(p);
	GLint ok;
	glGetProgramiv(p, GL_LINK_STATUS, &ok);
	if (!ok) {
		char log[4096];
		glGetProgramInfoLog(p, sizeof log, NULL, log);
		fprintf(stderr, "link: %s\n", log);
		exit(1);
	}
	return p;
}

static target make_target(int w, int h) {
	target t = { .w = w, .h = h };
	glGenFramebuffers(1, &t.fbo);
	glBindFramebuffer(GL_FRAMEBUFFER, t.fbo);
	glGenTextures(1, &t.tex);
	glBindTexture(GL_TEXTURE_2D, t.tex);
	glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, w, h, 0, GL_RGBA, GL_UNSIGNED_BYTE, NULL);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
	glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
	glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, t.tex, 0);
	if (glCheckFramebufferStatus(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE) {
		fprintf(stderr, "incomplete fbo %dx%d\n", w, h);
		exit(1);
	}
	return t;
}

static void draw(target *dst, GLuint prog, GLuint tex, int w, int h) {
	static const float quad[] = { -1, -1, 1, -1, -1, 1, 1, 1 };
	glBindFramebuffer(GL_FRAMEBUFFER, dst->fbo);
	glViewport(0, 0, w, h);
	glUseProgram(prog);
	glActiveTexture(GL_TEXTURE0);
	glBindTexture(GL_TEXTURE_2D, tex);
	glUniform1i(glGetUniformLocation(prog, "tex"), 0);
	glEnableVertexAttribArray(0);
	glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, 0, quad);
	glDrawArrays(GL_TRIANGLE_STRIP, 0, 4);
}

static GLuint p_plain, p_blur1, p_blur2, p_effects;
static target wall, half, quarter, eighth, blurred, fx, screen;
static int W, H;

// The downsample + upsample chain, then the draw of the blurred texture onto
// the output (a *different* buffer - sampling the target you write is a
// feedback loop, and the driver is allowed to throw that work away).
// `passes` levels: 2 uses half and quarter, 3 adds eighth.
static void chain(int passes, float radius, int with_effects) {
	for (int i = 0; i < passes; i++) {
		target *dst = i == 0 ? &half : i == 1 ? &quarter : &eighth;
		GLuint src = i == 0 ? wall.tex : (i == 1 ? half.tex : quarter.tex);
		glUniform1f(glGetUniformLocation(p_blur1, "radius"), radius);
		glUniform2f(glGetUniformLocation(p_blur1, "halfpixel"),
				0.5f / (dst->w / 2.0f), 0.5f / (dst->h / 2.0f));
		draw(dst, p_blur1, src, dst->w, dst->h);
	}
	for (int i = passes - 1; i >= 0; i--) {
		target *dst = i == 0 ? &blurred : i == 1 ? &half : &quarter;
		target *src_t = i == 0 ? &half : i == 1 ? &quarter : &eighth;
		glUniform1f(glGetUniformLocation(p_blur2, "radius"), radius);
		glUniform2f(glGetUniformLocation(p_blur2, "halfpixel"),
				0.5f / (src_t->w * 2.0f), 0.5f / (src_t->h * 2.0f));
		draw(dst, p_blur2, src_t->tex, dst->w, dst->h);
	}
	if (with_effects) {
		glUseProgram(p_effects);
		glActiveTexture(GL_TEXTURE0);
		glBindTexture(GL_TEXTURE_2D, blurred.tex);
		glUniform1i(glGetUniformLocation(p_effects, "tex"), 0);
		glUniform1f(glGetUniformLocation(p_effects, "brightness"), 0.9f);
		glUniform1f(glGetUniformLocation(p_effects, "contrast"), 0.9f);
		glUniform1f(glGetUniformLocation(p_effects, "saturation"), 1.1f);
		glUniform1f(glGetUniformLocation(p_effects, "noise"), 0.02f);
		draw(&fx, p_effects, blurred.tex, W, H);
	}
	draw(&screen, p_plain, with_effects ? fx.tex : blurred.tex, W, H);
}

#define RUN(label, body) do { \
	double best = 1e9; \
	for (int r = 0; r < 5; r++) { \
		body; glFinish(); \
		double t0 = now_ms(); \
		for (int i = 0; i < iters; i++) { body; } \
		glFinish(); \
		double t1 = now_ms(); \
		if ((t1 - t0) / iters < best) best = (t1 - t0) / iters; \
	} \
	printf("%-42s %7.3f ms\n", label, best); \
} while (0)

int main(int argc, char **argv) {
	W = argc > 1 ? atoi(argv[1]) : 1366;
	H = argc > 2 ? atoi(argv[2]) : 912;
	int iters = argc > 3 ? atoi(argv[3]) : 200;

	PFNEGLQUERYDEVICESEXTPROC query_devices =
		(PFNEGLQUERYDEVICESEXTPROC)eglGetProcAddress("eglQueryDevicesEXT");
	PFNEGLGETPLATFORMDISPLAYEXTPROC get_platform_display =
		(PFNEGLGETPLATFORMDISPLAYEXTPROC)eglGetProcAddress("eglGetPlatformDisplayEXT");
	PFNEGLQUERYDEVICESTRINGEXTPROC device_string =
		(PFNEGLQUERYDEVICESTRINGEXTPROC)eglGetProcAddress("eglQueryDeviceStringEXT");
	if (!query_devices || !get_platform_display) { fprintf(stderr, "no EGL device API\n"); return 1; }

	// The device with a DRM render node is the Mali; GL_RENDERER confirms it.
	EGLDisplay dpy = EGL_NO_DISPLAY;
	EGLDeviceEXT devs[16];
	EGLint n = 0;
	query_devices(16, devs, &n);
	for (int i = 0; i < n && dpy == EGL_NO_DISPLAY; i++) {
		if (device_string && !device_string(devs[i], EGL_DRM_RENDER_NODE_FILE_EXT)) continue;
		EGLDisplay d = get_platform_display(EGL_PLATFORM_DEVICE_EXT, devs[i], NULL);
		if (d != EGL_NO_DISPLAY && eglInitialize(d, NULL, NULL)) dpy = d;
	}
	if (dpy == EGL_NO_DISPLAY) { fprintf(stderr, "no EGL device\n"); return 1; }
	if (!eglBindAPI(EGL_OPENGL_ES_API)) { fprintf(stderr, "eglBindAPI\n"); return 1; }

	EGLint cfg_attribs[] = {
		EGL_SURFACE_TYPE, EGL_PBUFFER_BIT,
		EGL_RENDERABLE_TYPE, EGL_OPENGL_ES2_BIT,
		EGL_RED_SIZE, 8, EGL_GREEN_SIZE, 8, EGL_BLUE_SIZE, 8,
		EGL_NONE
	};
	EGLConfig cfg;
	EGLint ncfg;
	if (!eglChooseConfig(dpy, cfg_attribs, &cfg, 1, &ncfg) || !ncfg) {
		fprintf(stderr, "no EGL config\n"); return 1;
	}
	EGLint ctx_attribs[] = { EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE };
	EGLContext ctx = eglCreateContext(dpy, cfg, EGL_NO_CONTEXT, ctx_attribs);
	if (ctx == EGL_NO_CONTEXT) { fprintf(stderr, "no EGL context\n"); return 1; }
	if (!eglMakeCurrent(dpy, EGL_NO_SURFACE, EGL_NO_SURFACE, ctx)) {
		fprintf(stderr, "eglMakeCurrent: 0x%x\n", eglGetError()); return 1;
	}

	printf("%s | %s | %s\n", glGetString(GL_RENDERER), glGetString(GL_VENDOR),
			glGetString(GL_VERSION));
	printf("%dx%d, best of 5 runs of %d iterations\n", W, H, iters);

	p_plain = program(fs_plain);
	p_blur1 = program(fs_blur1);
	p_blur2 = program(fs_blur2);
	p_effects = program(fs_effects);
	wall = make_target(W, H);
	half = make_target(W / 2, H / 2);
	quarter = make_target(W / 4, H / 4);
	eighth = make_target(W / 8, H / 8);
	blurred = make_target(W, H);
	fx = make_target(W, H);
	screen = make_target(W, H);

	glBindFramebuffer(GL_FRAMEBUFFER, wall.fbo);
	glViewport(0, 0, W, H);
	glClearColor(0.05f, 0.15f, 0.1f, 1.0f);
	glClear(GL_COLOR_BUFFER_BIT);

	// The cached path only samples the buffer that was blurred once.
	chain(2, 4.0f, 0);

	RUN("cached path: 1 bilinear tap, full screen",
			draw(&screen, p_plain, blurred.tex, W, H));
	RUN("live: r4 p2 + final draw",
			chain(2, 4.0f, 0));
	RUN("  ... + effects pass (noise 0.02)",
			chain(2, 4.0f, 1));
	RUN("live: r5 p3 + effects (shipped defaults)",
			chain(3, 5.0f, 1));
	printf("\npanel is 1366x912 @ 60.004 Hz: a frame is 16.7 ms\n");
	return 0;
}
