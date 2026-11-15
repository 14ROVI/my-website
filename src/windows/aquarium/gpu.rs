use super::asset::{Asset, Node};
use glam::{Mat4, Vec3};
use glow::{Context, HasContext};
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};

pub const MAX_JOINTS: usize = 32;
const WATER_PARTICLES: usize = 748;

const VERTEX: &str = "#version 300 es
layout(location=0) in vec3 position;
layout(location=1) in vec3 normal;
layout(location=2) in vec2 uv;
layout(location=3) in vec4 joints;
layout(location=4) in vec4 weights;
uniform mat4 camera, model, bones[32];
uniform bool skinned;
out vec2 texcoord;
out vec3 surface_normal, surface_position;
void main() {
    mat4 transform = model;
    if (skinned) transform = bones[int(joints.x)] * weights.x + bones[int(joints.y)] * weights.y
        + bones[int(joints.z)] * weights.z + bones[int(joints.w)] * weights.w;
    vec4 p = transform * vec4(position, 1.0);
    surface_position = p.xyz;
    surface_normal = mat3(transform) * normal;
    texcoord = uv;
    gl_Position = camera * p;
}";

const FRAGMENT: &str = "#version 300 es
precision highp float;
in vec2 texcoord;
in vec3 surface_normal, surface_position;
uniform sampler2D skin;
uniform vec4 colour;
uniform vec3 eye;
uniform float seconds;
uniform bool glowing_spots;
out vec4 pixel;
void main() {
    vec3 smooth_normal = normalize(surface_normal);
    vec3 face_normal = normalize(cross(dFdx(surface_position), dFdy(surface_position)));
    if (dot(face_normal, smooth_normal) < 0.0) face_normal = -face_normal;
    // Bring out the individual faces at aquarium scale while keeping fine triangles from glittering.
    vec3 n = normalize(mix(smooth_normal, face_normal, 0.65));
    vec3 linear = texture(skin, texcoord, -0.55).rgb * colour.rgb;
    vec3 base = mix(linear * 12.92, 1.055 * pow(linear, vec3(1.0 / 2.4)) - 0.055,
        step(vec3(0.0031308), linear));
    float depth = dot(normalize(eye), surface_position);
    vec3 view = normalize(eye - surface_position);
    float rim = 1.0 - max(dot(n, view), 0.0);
    float band = max(sin(seconds * 1.15 + surface_position.x * 1.6
        + sin(surface_position.z - seconds * 0.4) * 1.7), 0.0);
    // Fade the overhead wave light out on the sides and exclude downward-facing surfaces.
    float wave_light = pow(band, 4.0) * 0.21 * smoothstep(0.0, 0.35, smooth_normal.y);
    float diffuse = dot(n, normalize(vec3(-0.37, 0.87, 0.32)));
    diffuse = mix(diffuse, floor(diffuse * 7.0 + 0.5) / 7.0, 0.72);
    float exposure = clamp(0.77 + diffuse * 0.49 + depth * 0.011 + wave_light, 0.43, 1.34);
    vec3 lit = mix(base * exposure, vec3(120.0, 205.0, 231.0) / 255.0, rim * rim * 0.14);
    if (glowing_spots) {
        float back = smoothstep(0.1, 0.45, cos(texcoord.y * 6.28318));
        float spot = smoothstep(0.37, 0.68, base.r) * back;
        lit += vec3(0.10, 0.12, 0.14) * spot;
    }
    pixel = vec4(lit, 1.0);
}";

const WATER_VERTEX: &str = "#version 300 es
layout(location=0) in vec4 position_seed;
layout(location=1) in vec3 style_seed;
uniform mat4 camera;
uniform vec3 direction;
uniform float seconds;
out vec3 colour;
out float opacity, bubble;
void main() {
    float rise = fract(style_seed.x + seconds * (0.045 + position_seed.z * 0.069) * (22.8 / 72.0));
    // Independent positions and initial rise phases fill the water from the first frame.
    vec3 p = vec3((position_seed.x - 0.5) * 80.0
        + sin(rise * 8.0 + position_seed.w * 6.283185) * 0.5,
        rise * 72.0 - 36.0, (position_seed.y - 0.5) * 80.0);
    gl_Position = camera * vec4(p, 1.0);
    float depth = dot(direction, p);
    float size = (3.0 + style_seed.z * 2.0) * 24.5 / max(gl_Position.w, 1.0);
    bubble = step(0.72, style_seed.z);
    gl_PointSize = mix(1.0, clamp(floor(size + 0.5), 3.0, 7.0), bubble);
    float fade = min(1.0, min(rise * 9.0, (1.0 - rise) * 8.0))
        * smoothstep(0.5, 2.0, gl_Position.w);
    float gleam = fade * (0.18 + style_seed.y * 0.59) * (0.8 + depth * 0.02);
    colour = mix(vec3(22.0, 49.0, 66.0), vec3(172.0, 226.0, 234.0), clamp(gleam, 0.0, 1.0)) / 255.0;
    opacity = smoothstep(0.04, 0.12, gleam);
}";

const WATER_FRAGMENT: &str = "#version 300 es
precision mediump float;
in vec3 colour;
in float opacity, bubble;
out vec4 pixel;
void main() {
    vec2 p = gl_PointCoord - 0.5;
    p.y *= 1.14;
    float distance_to_centre = length(p);
    float ring = step(0.22, distance_to_centre) * step(distance_to_centre, 0.57);
    float highlight = step(p.x + p.y, -0.12);
    float rim = ring * mix(0.44, 1.0, highlight);
    vec3 lit = mix(colour, colour * 1.23, ring * highlight * bubble);
    pixel = vec4(lit, opacity * mix(1.0, rim, bubble));
}";

struct Primitive {
    vao: glow::VertexArray,
    count: i32,
    material: usize,
    node: usize,
    skin: Option<usize>,
}

struct Material {
    colour: [f32; 4],
    glowing_spots: bool,
    texture: glow::Texture,
}

struct Resources {
    gl: Context,
    programs: Vec<glow::Program>,
    arrays: Vec<glow::VertexArray>,
    buffers: Vec<glow::Buffer>,
    textures: Vec<glow::Texture>,
}

impl Resources {
    fn new(canvas: &HtmlCanvasElement) -> Result<Self, String> {
        let context = canvas
            .get_context("webgl2")
            .ok()
            .flatten()
            .and_then(|gl| gl.dyn_into::<WebGl2RenderingContext>().ok())
            .ok_or("This browser could not start WebGL2.")?;
        Ok(Self {
            gl: Context::from_webgl2_context(context),
            programs: vec![],
            arrays: vec![],
            buffers: vec![],
            textures: vec![],
        })
    }

    fn program(&mut self, vertex: &str, fragment: &str) -> Result<glow::Program, String> {
        let gl = &self.gl;
        // All handles below belong to this canvas's context.
        unsafe {
            let program = gl.create_program()?;
            self.programs.push(program);
            for (kind, source) in [
                (glow::VERTEX_SHADER, vertex),
                (glow::FRAGMENT_SHADER, fragment),
            ] {
                let shader = gl.create_shader(kind)?;
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    let error = gl.get_shader_info_log(shader);
                    gl.delete_shader(shader);
                    return Err(error);
                }
                gl.attach_shader(program, shader);
                gl.delete_shader(shader);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(gl.get_program_info_log(program));
            }
            Ok(program)
        }
    }

    fn array(&mut self) -> Result<glow::VertexArray, String> {
        let array = unsafe { self.gl.create_vertex_array()? };
        self.arrays.push(array);
        unsafe { self.gl.bind_vertex_array(Some(array)) };
        Ok(array)
    }

    fn buffer<T: bytemuck::Pod>(&mut self, target: u32, data: &[T]) -> Result<(), String> {
        unsafe {
            let buffer = self.gl.create_buffer()?;
            self.buffers.push(buffer);
            self.gl.bind_buffer(target, Some(buffer));
            self.gl
                .buffer_data_u8_slice(target, bytemuck::cast_slice(data), glow::STATIC_DRAW);
        }
        Ok(())
    }

    fn texture(&mut self, width: i32, height: i32, rgba: &[u8]) -> Result<glow::Texture, String> {
        unsafe {
            let gl = &self.gl;
            let texture = gl.create_texture()?;
            self.textures.push(texture);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::SRGB8_ALPHA8 as i32,
                width,
                height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(rgba)),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR_MIPMAP_LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.generate_mipmap(glow::TEXTURE_2D);
            Ok(texture)
        }
    }
}

impl Drop for Resources {
    fn drop(&mut self) {
        unsafe {
            for &vao in &self.arrays {
                self.gl.delete_vertex_array(vao);
            }
            for &buffer in &self.buffers {
                self.gl.delete_buffer(buffer);
            }
            for &texture in &self.textures {
                self.gl.delete_texture(texture);
            }
            for &program in &self.programs {
                self.gl.delete_program(program);
            }
        }
    }
}

fn decode_png(bytes: &[u8]) -> Result<(i32, i32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|err| err.to_string())?;
    let info = reader.info();
    if info.width > 4096 || info.height > 4096 {
        return Err("GLB texture exceeds 4096 pixels.".into());
    }
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut pixels)
        .map_err(|err| err.to_string())?;
    let bytes = &pixels[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => bytes
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => bytes.iter().flat_map(|&p| [p, p, p, 255]).collect(),
        png::ColorType::GrayscaleAlpha => bytes
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Indexed => return Err("Unsupported GLB texture.".into()),
    };
    Ok((info.width as i32, info.height as i32, rgba))
}

struct MeshUniforms {
    camera: Option<glow::UniformLocation>,
    eye: Option<glow::UniformLocation>,
    colour: Option<glow::UniformLocation>,
    glowing_spots: Option<glow::UniformLocation>,
    seconds: Option<glow::UniformLocation>,
    skinned: Option<glow::UniformLocation>,
    model: Option<glow::UniformLocation>,
    bones: Option<glow::UniformLocation>,
}

pub struct Renderer {
    gpu: Resources,
    program: glow::Program,
    uniforms: MeshUniforms,
    primitives: Vec<Primitive>,
    materials: Vec<Material>,
    water_program: glow::Program,
    water_array: glow::VertexArray,
    water_camera: Option<glow::UniformLocation>,
    water_time: Option<glow::UniformLocation>,
    water_direction: Option<glow::UniformLocation>,
    asset: Asset,
    posed: Vec<Node>,
    world: Vec<Mat4>,
    palettes: Vec<Vec<Mat4>>,
    pose_time: Option<f32>,
}

impl Renderer {
    pub fn new(canvas: &HtmlCanvasElement, bytes: &[u8]) -> Result<Self, String> {
        let model = gltf::Gltf::from_slice(bytes).map_err(|err| err.to_string())?;
        let bin = model
            .blob
            .as_deref()
            .ok_or("Expected a self-contained GLB.")?;
        let asset = Asset::read(&model, bin)?;
        let mut gpu = Resources::new(canvas)?;
        let program = gpu.program(VERTEX, FRAGMENT)?;
        let water_program = gpu.program(WATER_VERTEX, WATER_FRAGMENT)?;
        let uniform = |name| unsafe { gpu.gl.get_uniform_location(program, name) };
        let uniforms = MeshUniforms {
            camera: uniform("camera"),
            eye: uniform("eye"),
            colour: uniform("colour"),
            glowing_spots: uniform("glowing_spots"),
            seconds: uniform("seconds"),
            skinned: uniform("skinned"),
            model: uniform("model"),
            bones: uniform("bones[0]"),
        };
        let water_camera = unsafe { gpu.gl.get_uniform_location(water_program, "camera") };
        let water_time = unsafe { gpu.gl.get_uniform_location(water_program, "seconds") };
        let water_direction = unsafe { gpu.gl.get_uniform_location(water_program, "direction") };
        let white = gpu.texture(1, 1, &[255; 4])?;
        let mut textures = Vec::new();
        for image in model.images() {
            let gltf::image::Source::View {
                view,
                mime_type: "image/png",
            } = image.source()
            else {
                return Err("Export embedded PNG textures from Blender.".into());
            };
            let start = view.offset();
            let data = bin
                .get(start..start.saturating_add(view.length()))
                .ok_or("Invalid image buffer.")?;
            let (width, height, pixels) = decode_png(data)?;
            textures.push(gpu.texture(width, height, &pixels)?);
        }
        let mut materials: Vec<_> = model
            .materials()
            .map(|material| {
                let pbr = material.pbr_metallic_roughness();
                Material {
                    colour: pbr.base_color_factor(),
                    glowing_spots: material.name() == Some("Whale shark • spotted skin"),
                    texture: pbr
                        .base_color_texture()
                        .map_or(white, |info| textures[info.texture().source().index()]),
                }
            })
            .collect();
        let default_material = materials.len();
        materials.push(Material {
            colour: [1.0; 4],
            glowing_spots: false,
            texture: white,
        });
        let data = |buffer: gltf::Buffer| {
            matches!(buffer.source(), gltf::buffer::Source::Bin).then_some(bin)
        };
        let mut primitives = Vec::new();
        for node in model
            .nodes()
            .filter(|node| asset.order.contains(&node.index()))
        {
            let Some(mesh) = node.mesh() else { continue };
            let skin = node.skin().map(|skin| skin.index());
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    return Err("Only triangle meshes are supported.".into());
                }
                let reader = primitive.reader(data);
                let positions: Vec<_> = reader
                    .read_positions()
                    .ok_or("GLB mesh needs positions.")?
                    .collect();
                let count = positions.len();
                let normals: Vec<_> = reader
                    .read_normals()
                    .ok_or("Export GLB mesh normals.")?
                    .collect();
                let uv: Vec<_> = reader
                    .read_tex_coords(0)
                    .map(|coords| coords.into_f32().collect())
                    .unwrap_or_else(|| vec![[0.0; 2]; count]);
                let joints: Vec<_> = reader
                    .read_joints(0)
                    .map(|indices| indices.into_u16().collect())
                    .unwrap_or_else(|| vec![[0; 4]; count]);
                let weights: Vec<_> = reader
                    .read_weights(0)
                    .map(|weights| weights.into_f32().collect())
                    .unwrap_or_else(|| vec![[1.0, 0.0, 0.0, 0.0]; count]);
                if [normals.len(), uv.len(), joints.len(), weights.len()]
                    .into_iter()
                    .any(|length| length != count)
                {
                    return Err("Mismatched GLB vertex attributes.".into());
                }
                if skin.is_some_and(|index| {
                    joints
                        .iter()
                        .flatten()
                        .any(|&joint| joint as usize >= asset.skins[index].joints.len())
                }) {
                    return Err("Invalid GLB vertex joint.".into());
                }
                let mut vertices = Vec::with_capacity(count);
                for i in 0..count {
                    let [x, y, z] = positions[i];
                    let [nx, ny, nz] = normals[i];
                    let [u, v] = uv[i];
                    let [j0, j1, j2, j3] = joints[i].map(f32::from);
                    let [w0, w1, w2, w3] = weights[i];
                    vertices.push([x, y, z, nx, ny, nz, u, v, j0, j1, j2, j3, w0, w1, w2, w3]);
                }
                let indices: Vec<_> = reader
                    .read_indices()
                    .map(|indices| indices.into_u32().collect())
                    .unwrap_or_else(|| (0..count as u32).collect());
                if indices.iter().any(|&index| index as usize >= count) {
                    return Err("Invalid GLB mesh index.".into());
                }
                let vao = gpu.array()?;
                gpu.buffer(glow::ARRAY_BUFFER, &vertices)?;
                gpu.buffer(glow::ELEMENT_ARRAY_BUFFER, &indices)?;
                unsafe {
                    for (attribute, size, offset) in
                        [(0, 3, 0), (1, 3, 12), (2, 2, 24), (3, 4, 32), (4, 4, 48)]
                    {
                        gpu.gl.enable_vertex_attrib_array(attribute);
                        gpu.gl.vertex_attrib_pointer_f32(
                            attribute,
                            size,
                            glow::FLOAT,
                            false,
                            64,
                            offset,
                        );
                    }
                }
                primitives.push(Primitive {
                    vao,
                    count: indices.len() as i32,
                    material: primitive.material().index().unwrap_or(default_material),
                    node: node.index(),
                    skin,
                });
            }
        }
        if primitives.is_empty() {
            return Err("The GLB scene contains no meshes.".into());
        }
        let water_array = Self::water_array(&mut gpu, WATER_PARTICLES, 19, [0, 1, 2, 3, 4, 5, 6])?;
        unsafe {
            gpu.gl.bind_vertex_array(None);
            gpu.gl.enable(glow::DEPTH_TEST);
            gpu.gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gpu.gl.blend_func_separate(
                glow::SRC_ALPHA,
                glow::ONE_MINUS_SRC_ALPHA,
                glow::ONE,
                glow::ONE_MINUS_SRC_ALPHA,
            );
        }
        let posed = asset.nodes.clone();
        let world = vec![Mat4::IDENTITY; posed.len()];
        let palettes = asset
            .skins
            .iter()
            .map(|skin| vec![Mat4::IDENTITY; skin.joints.len()])
            .collect();
        Ok(Self {
            gpu,
            program,
            uniforms,
            primitives,
            materials,
            water_program,
            water_array,
            water_camera,
            water_time,
            water_direction,
            asset,
            posed,
            world,
            palettes,
            pose_time: None,
        })
    }

    fn water_array(
        gpu: &mut Resources,
        count: usize,
        stride: u32,
        offsets: [u32; 7],
    ) -> Result<glow::VertexArray, String> {
        let array = gpu.array()?;
        let seeds: Vec<_> = (0..count as u32)
            .map(|i| {
                offsets.map(|offset| {
                    let seed = i * stride + offset;
                    let mut mixed = seed ^ (seed >> 16);
                    mixed = mixed.wrapping_mul(0x7feb_352d);
                    mixed ^= mixed >> 15;
                    mixed = mixed.wrapping_mul(0x846c_a68b);
                    mixed ^= mixed >> 16;
                    (mixed >> 8) as f32 / 16_777_216.0
                })
            })
            .collect();
        gpu.buffer(glow::ARRAY_BUFFER, &seeds)?;
        unsafe {
            gpu.gl.enable_vertex_attrib_array(0);
            gpu.gl.enable_vertex_attrib_array(1);
            gpu.gl
                .vertex_attrib_pointer_f32(0, 4, glow::FLOAT, false, 28, 0);
            gpu.gl
                .vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 28, 16);
        }
        Ok(array)
    }

    pub fn draw(&mut self, width: i32, height: i32, camera: Mat4, eye: Vec3, seconds: f32) {
        if self.pose_time != Some(seconds) {
            self.asset.pose(seconds, &mut self.posed, &mut self.world);
            for (skin, palette) in self.asset.skins.iter().zip(&mut self.palettes) {
                for ((&joint, inverse), matrix) in
                    skin.joints.iter().zip(&skin.inverse_bind).zip(palette)
                {
                    *matrix = self.world[joint] * *inverse;
                }
            }
            self.pose_time = Some(seconds);
        }
        let gl = &self.gpu.gl;
        let u = &self.uniforms;
        unsafe {
            gl.viewport(0, 0, width, height);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.disable(glow::BLEND);
            gl.enable(glow::DEPTH_TEST);
            gl.use_program(Some(self.program));
            gl.uniform_matrix_4_f32_slice(u.camera.as_ref(), false, &camera.to_cols_array());
            gl.uniform_3_f32(u.eye.as_ref(), eye.x, eye.y, eye.z);
            gl.uniform_1_f32(u.seconds.as_ref(), seconds);
            let mut last_skin = None;
            for primitive in &self.primitives {
                let material = &self.materials[primitive.material];
                gl.bind_texture(glow::TEXTURE_2D, Some(material.texture));
                gl.uniform_4_f32_slice(u.colour.as_ref(), &material.colour);
                gl.uniform_1_i32(u.glowing_spots.as_ref(), i32::from(material.glowing_spots));
                gl.uniform_1_i32(u.skinned.as_ref(), i32::from(primitive.skin.is_some()));
                if let Some(skin) = primitive.skin {
                    if last_skin != Some(skin) {
                        gl.uniform_matrix_4_f32_slice(
                            u.bones.as_ref(),
                            false,
                            bytemuck::cast_slice(&self.palettes[skin]),
                        );
                        last_skin = Some(skin);
                    }
                } else {
                    gl.uniform_matrix_4_f32_slice(
                        u.model.as_ref(),
                        false,
                        &self.world[primitive.node].to_cols_array(),
                    );
                }
                gl.bind_vertex_array(Some(primitive.vao));
                gl.draw_elements(glow::TRIANGLES, primitive.count, glow::UNSIGNED_INT, 0);
            }
            gl.enable(glow::BLEND);
            gl.depth_mask(false);
            gl.use_program(Some(self.water_program));
            let direction = eye.normalize();
            gl.uniform_3_f32(
                self.water_direction.as_ref(),
                direction.x,
                direction.y,
                direction.z,
            );
            gl.uniform_1_f32(self.water_time.as_ref(), seconds);
            gl.uniform_matrix_4_f32_slice(
                self.water_camera.as_ref(),
                false,
                &camera.to_cols_array(),
            );
            gl.bind_vertex_array(Some(self.water_array));
            gl.draw_arrays(glow::POINTS, 0, WATER_PARTICLES as i32);
            gl.depth_mask(true);
            gl.disable(glow::BLEND);
            gl.bind_vertex_array(None);
        }
    }
}
