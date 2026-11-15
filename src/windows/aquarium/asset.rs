use glam::{Mat3, Mat4, Quat, Vec3};
use gltf::animation::{util::ReadOutputs, Interpolation};

pub struct Asset {
    pub nodes: Vec<Node>,
    pub order: Vec<usize>,
    pub skins: Vec<Skin>,
    pub tracks: Vec<Track>,
    turning_pose: Vec<(usize, Quat)>,
    swim_roots: Vec<usize>,
}

#[derive(Clone, Copy)]
pub struct Node {
    pub parent: Option<usize>,
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

pub struct Skin {
    pub joints: Vec<usize>,
    pub inverse_bind: Vec<Mat4>,
}

pub struct Track {
    node: usize,
    time: Vec<f32>,
    duration: f32,
    step: bool,
    values: Values,
}

enum Values {
    Position(Vec<Vec3>),
    Rotation(Vec<Quat>),
    Scale(Vec<Vec3>),
}

impl Asset {
    pub fn read(gltf: &gltf::Document, bin: &[u8]) -> Result<Self, String> {
        let scene = gltf
            .default_scene()
            .or_else(|| gltf.scenes().next())
            .ok_or("No scene in GLB.")?;
        let data = |buffer: gltf::Buffer| {
            matches!(buffer.source(), gltf::buffer::Source::Bin).then_some(bin)
        };
        let mut nodes: Vec<_> = gltf
            .nodes()
            .map(|node| {
                let (position, rotation, scale) = node.transform().decomposed();
                Node {
                    parent: None,
                    translation: Vec3::from(position),
                    rotation: Quat::from_array(rotation).normalize(),
                    scale: Vec3::from(scale),
                }
            })
            .collect();
        fn visit(
            node: gltf::Node,
            parent: Option<usize>,
            nodes: &mut [Node],
            order: &mut Vec<usize>,
            seen: &mut [bool],
            depth: u32,
        ) -> Result<(), String> {
            let index = node.index();
            if depth > 128 || seen[index] {
                return Err("Invalid GLB node hierarchy.".into());
            }
            seen[index] = true;
            nodes[index].parent = parent;
            order.push(index);
            for child in node.children() {
                visit(child, Some(index), nodes, order, seen, depth + 1)?;
            }
            Ok(())
        }
        let mut order = Vec::new();
        let mut seen = vec![false; nodes.len()];
        for root in scene.nodes() {
            visit(root, None, &mut nodes, &mut order, &mut seen, 0)?;
        }
        let mut skins = Vec::new();
        for skin in gltf.skins() {
            let joints: Vec<_> = skin.joints().map(|joint| joint.index()).collect();
            if joints.is_empty() || joints.len() > super::gpu::MAX_JOINTS {
                return Err("The viewer supports up to 32 joints per skin.".into());
            }
            let inverse_bind: Vec<_> = skin
                .reader(data)
                .read_inverse_bind_matrices()
                .map(|matrices| {
                    matrices
                        .map(|matrix| Mat4::from_cols_array_2d(&matrix))
                        .collect()
                })
                .unwrap_or_else(|| vec![Mat4::IDENTITY; joints.len()]);
            if inverse_bind.len() != joints.len() {
                return Err("Invalid GLB skin.".into());
            }
            skins.push(Skin {
                joints,
                inverse_bind,
            });
        }
        let mut tracks = Vec::new();
        for animation in gltf.animations() {
            let first_track = tracks.len();
            let mut duration = 0.0_f32;
            for channel in animation.channels() {
                let reader = channel.reader(data);
                let time: Vec<_> = reader
                    .read_inputs()
                    .ok_or("Missing GLB keyframe times.")?
                    .collect();
                if time.is_empty()
                    || time.iter().any(|t| !t.is_finite())
                    || time.windows(2).any(|pair| pair[0] >= pair[1])
                {
                    return Err("Invalid GLB keyframe times.".into());
                }
                let interpolation = channel.sampler().interpolation();
                if interpolation == Interpolation::CubicSpline {
                    return Err("Export sampled linear or step animation from Blender.".into());
                }
                let (values, count) =
                    match reader.read_outputs().ok_or("Missing GLB animation data.")? {
                        ReadOutputs::Translations(data) => {
                            let values: Vec<_> = data.map(Vec3::from).collect();
                            let count = values.len();
                            (Values::Position(values), count)
                        }
                        ReadOutputs::Rotations(data) => {
                            let values: Vec<_> = data
                                .into_f32()
                                .map(|q| Quat::from_array(q).normalize())
                                .collect();
                            let count = values.len();
                            (Values::Rotation(values), count)
                        }
                        ReadOutputs::Scales(data) => {
                            let values: Vec<_> = data.map(Vec3::from).collect();
                            let count = values.len();
                            (Values::Scale(values), count)
                        }
                        ReadOutputs::MorphTargetWeights(_) => {
                            return Err("Morph animations are unsupported.".into())
                        }
                    };
                if count != time.len() {
                    return Err("Mismatched GLB keyframe counts.".into());
                }
                duration = duration.max(*time.last().unwrap());
                tracks.push(Track {
                    node: channel.target().node().index(),
                    time,
                    duration: 0.0,
                    step: interpolation == Interpolation::Step,
                    values,
                });
            }
            for track in &mut tracks[first_track..] {
                track.duration = duration;
            }
        }
        // The baked paths circle to the right. Curve the body into that turn,
        // with the head leading and the tail trailing, beneath the swim cycle.
        let turning_pose = gltf
            .nodes()
            .filter_map(|node| {
                let angle: f32 = match node.name()? {
                    "Head" => -0.07,
                    "Spine 01" => 0.035,
                    "Spine 02" => 0.06,
                    "Spine 03" | "Spine 04" => 0.075,
                    "Spine 05" => 0.06,
                    "Spine 06" => 0.045,
                    _ => return None,
                };
                // Bone-local Z is the shark's vertical axis.
                Some((node.index(), Quat::from_rotation_z(angle)))
            })
            .collect();
        let swim_roots = gltf
            .nodes()
            .filter(|node| node.name().is_some_and(|name| name.ends_with(" | swim")))
            .map(|node| node.index())
            .collect();
        Ok(Self {
            nodes,
            order,
            skins,
            tracks,
            turning_pose,
            swim_roots,
        })
    }

    pub fn pose(&self, seconds: f32, posed: &mut [Node], world: &mut [Mat4]) {
        posed.copy_from_slice(&self.nodes);
        for track in &self.tracks {
            let time = if track.duration > 0.0 {
                seconds.rem_euclid(track.duration)
            } else {
                0.0
            };
            let i = track
                .time
                .partition_point(|sample| *sample <= time)
                .saturating_sub(1);
            let j = (i + 1).min(track.time.len() - 1);
            let weight = if track.step || i == j {
                0.0
            } else {
                ((time - track.time[i]) / (track.time[j] - track.time[i])).clamp(0.0, 1.0)
            };
            let node = &mut posed[track.node];
            match &track.values {
                Values::Position(values) => node.translation = values[i].lerp(values[j], weight),
                Values::Rotation(values) => {
                    node.rotation = values[i].slerp(values[j], weight).normalize()
                }
                Values::Scale(values) => node.scale = values[i].lerp(values[j], weight),
            }
        }
        for &(index, turn) in &self.turning_pose {
            posed[index].rotation = (posed[index].rotation * turn).normalize();
        }
        for &index in &self.swim_roots {
            let node = &mut posed[index];
            // Keep the sampled heading and pitch, but replace the baked roll
            // so it cannot overpower the inward bank during the swim cycle.
            let along = node.rotation * Vec3::X;
            let side = along.cross(Vec3::Y).normalize();
            let up = side.cross(along);
            node.rotation = (Quat::from_mat3(&Mat3::from_cols(along, up, side))
                * Quat::from_rotation_x(-0.06))
            .normalize();
        }
        for &index in &self.order {
            let node = posed[index];
            let transform =
                Mat4::from_scale_rotation_translation(node.scale, node.rotation, node.translation);
            world[index] = node
                .parent
                .map_or(transform, |parent| world[parent] * transform);
        }
    }
}
