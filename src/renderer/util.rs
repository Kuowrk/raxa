use glam::Vec3;

pub fn calculate_pitch(forward: Vec3) -> f32 {
    let forward = forward.normalize();
    forward.y.clamp(-1.0, 1.0).asin()
}

pub fn calculate_yaw(forward: Vec3) -> f32 {
    let forward = forward.normalize();
    forward.z.atan2(forward.x)
}

pub fn calculate_roll(forward: Vec3, up: Vec3) -> f32 {
    let right = forward.cross(up).normalize();
    right.y.atan2(right.x)
}

pub fn calculate_direction(pitch: f32, yaw: f32) -> Vec3 {
    Vec3::new(
        yaw.cos() * pitch.cos(),
        pitch.sin(),
        yaw.sin() * pitch.cos(),
    )
}