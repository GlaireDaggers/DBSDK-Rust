use std::{mem::size_of, convert::TryInto, ops};

use field_offset::FieldOffset;

use crate::db_internal::{mat4_loadSIMD, mat4_storeSIMD, mat4_mulSIMD, mat4_transformSIMD};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    pub fn new(x: f32, y: f32) -> Vector2 {
        return Vector2 { x: x, y: y };
    }

    pub fn zero() -> Vector2 {
        return Vector2 { x: 0.0, y: 0.0 };
    }

    pub fn unit_x() -> Vector2 {
        return Vector2 { x: 1.0, y: 0.0 };
    }
    
    pub fn unit_y() -> Vector2 {
        return Vector2 { x: 0.0, y: 1.0 };
    }

    /// Compute the squared distance between two vectors
    pub fn distance_sq(lhs: &Vector2, rhs: &Vector2) -> f32 {
        let dx = lhs.x - rhs.x;
        let dy = lhs.y - rhs.y;
        return (dx * dx) + (dy * dy);
    }

    /// Compute the distance between two vectors
    pub fn distance(lhs: &Vector2, rhs: &Vector2) -> f32 {
        let dx = lhs.x - rhs.x;
        let dy = lhs.y - rhs.y;
        return ((dx * dx) + (dy * dy)).sqrt();
    }

    /// Compute the squared length of the vector
    pub fn length_sq(self) -> f32 {
        return (self.x * self.x) + (self.y * self.y);
    }

    /// Compute the length of the vector
    pub fn length(self) -> f32 {
        return ((self.x * self.x) + (self.y * self.y)).sqrt();
    }

    /// Compute the dot product of two vectors
    pub fn dot(lhs: &Vector2, rhs: &Vector2) -> f32 {
        return (lhs.x * rhs.x) + (lhs.y * rhs.y);
    }
}

impl ops::Add<Vector2> for Vector2 {
    type Output = Vector2;

    fn add(self, rhs: Vector2) -> Vector2 {
        return Vector2 { x: self.x + rhs.x, y: self.y + rhs.y };
    }
}

impl ops::Sub<Vector2> for Vector2 {
    type Output = Vector2;

    fn sub(self, rhs: Vector2) -> Vector2 {
        return Vector2 { x: self.x - rhs.x, y: self.y - rhs.y };
    }
}

impl ops::Mul<Vector2> for Vector2 {
    type Output = Vector2;

    fn mul(self, rhs: Vector2) -> Vector2 {
        return Vector2 { x: self.x * rhs.x, y: self.y * rhs.y };
    }
}

impl ops::Mul<f32> for Vector2 {
    type Output = Vector2;

    fn mul(self, rhs: f32) -> Vector2 {
        return Vector2 { x: self.x * rhs, y: self.y * rhs };
    }
}

impl ops::Mul<Vector2> for f32 {
    type Output = Vector2;

    fn mul(self, rhs: Vector2) -> Vector2 {
        return Vector2 { x: self * rhs.x, y: self * rhs.y };
    }
}

impl ops::Div<Vector2> for Vector2 {
    type Output = Vector2;

    fn div(self, rhs: Vector2) -> Vector2 {
        return Vector2 { x: self.x / rhs.x, y: self.y / rhs.y };
    }
}

impl ops::Div<f32> for Vector2 {
    type Output = Vector2;

    fn div(self, rhs: f32) -> Vector2 {
        return Vector2 { x: self.x / rhs, y: self.y / rhs };
    }
}

impl ops::Div<Vector2> for f32 {
    type Output = Vector2;

    fn div(self, rhs: Vector2) -> Vector2 {
        return Vector2 { x: self / rhs.x, y: self / rhs.y };
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vector3 {
        return Vector3 { x: x, y: y, z: z };
    }

    pub fn zero() -> Vector3 {
        return Vector3 { x: 0.0, y: 0.0, z: 0.0 };
    }

    pub fn unit_x() -> Vector3 {
        return Vector3 { x: 1.0, y: 0.0, z: 0.0 };
    }

    pub fn unit_y() -> Vector3 {
        return Vector3 { x: 0.0, y: 1.0, z: 0.0 };
    }

    pub fn unit_z() -> Vector3 {
        return Vector3 { x: 0.0, y: 0.0, z: 1.0 };
    }

    /// Compute the squared distance between two vectors
    pub fn distance_sq(lhs: &Vector3, rhs: &Vector3) -> f32 {
        let dx = lhs.x - rhs.x;
        let dy = lhs.y - rhs.y;
        let dz = lhs.z - rhs.z;
        return (dx * dx) + (dy * dy) + (dz * dz);
    }

    /// Compute the distance between two vectors
    pub fn distance(lhs: &Vector3, rhs: &Vector3) -> f32 {
        let dx = lhs.x - rhs.x;
        let dy = lhs.y - rhs.y;
        let dz = lhs.z - rhs.z;
        return ((dx * dx) + (dy * dy) + (dz * dz)).sqrt();
    }

    /// Compute the squared length of the vector
    pub fn length_sq(self) -> f32 {
        return (self.x * self.x) + (self.y * self.y) + (self.z * self.z);
    }

    /// Compute the length of the vector
    pub fn length(self) -> f32 {
        return ((self.x * self.x) + (self.y * self.y) + (self.z * self.z)).sqrt();
    }

    /// Compute the dot product of two vectors
    pub fn dot(lhs: &Vector3, rhs: &Vector3) -> f32 {
        return (lhs.x * rhs.x) + (lhs.y * rhs.y) + (lhs.z * rhs.z);
    }

    /// Compute the cross product of two vectors
    pub fn cross(lhs: &Vector3, rhs: &Vector3) -> Vector3 {
        return Vector3 {
            x: lhs.y * rhs.z - lhs.z * rhs.y,
            y: lhs.x * rhs.z - lhs.z * rhs.x,
            z: lhs.x * rhs.y - lhs.y * rhs.x
        };
    }
}

impl ops::Add<Vector3> for Vector3 {
    type Output = Vector3;

    fn add(self, rhs: Vector3) -> Vector3 {
        return Vector3 { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z };
    }
}

impl ops::Sub<Vector3> for Vector3 {
    type Output = Vector3;

    fn sub(self, rhs: Vector3) -> Vector3 {
        return Vector3 { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z };
    }
}

impl ops::Mul<Vector3> for Vector3 {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Vector3 {
        return Vector3 { x: self.x * rhs.x, y: self.y * rhs.y, z: self.z * rhs.z };
    }
}

impl ops::Mul<f32> for Vector3 {
    type Output = Vector3;

    fn mul(self, rhs: f32) -> Vector3 {
        return Vector3 { x: self.x * rhs, y: self.y * rhs, z: self.z * rhs };
    }
}

impl ops::Mul<Vector3> for f32 {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Vector3 {
        return Vector3 { x: self * rhs.x, y: self * rhs.y, z: self * rhs.z };
    }
}

impl ops::Div<Vector3> for Vector3 {
    type Output = Vector3;

    fn div(self, rhs: Vector3) -> Vector3 {
        return Vector3 { x: self.x / rhs.x, y: self.y / rhs.y, z: self.z / rhs.z };
    }
}

impl ops::Div<f32> for Vector3 {
    type Output = Vector3;

    fn div(self, rhs: f32) -> Vector3 {
        return Vector3 { x: self.x / rhs, y: self.y / rhs, z: self.z / rhs };
    }
}

impl ops::Div<Vector3> for f32 {
    type Output = Vector3;

    fn div(self, rhs: Vector3) -> Vector3 {
        return Vector3 { x: self / rhs.x, y: self / rhs.y, z: self / rhs.z };
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vector4 {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Vector4 {
        return Vector4 { x: x, y: y, z: z, w: w };
    }

    pub fn zero() -> Vector4 {
        return Vector4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
    }

    pub fn unit_x() -> Vector4 {
        return Vector4 { x: 1.0, y: 0.0, z: 0.0, w: 0.0 };
    }

    pub fn unit_y() -> Vector4 {
        return Vector4 { x: 0.0, y: 1.0, z: 0.0, w: 0.0 };
    }

    pub fn unit_z() -> Vector4 {
        return Vector4 { x: 0.0, y: 0.0, z: 1.0, w: 0.0 };
    }

    pub fn unit_w() -> Vector4 {
        return Vector4 { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };
    }

    /// Compute the squared distance between two vectors
    pub fn distance_sq(lhs: &Vector4, rhs: &Vector4) -> f32 {
        let dx = lhs.x - rhs.x;
        let dy = lhs.y - rhs.y;
        let dz = lhs.z - rhs.z;
        let dw = lhs.w - rhs.w;
        return (dx * dx) + (dy * dy) + (dz * dz) + (dw * dw);
    }

    /// Compute the distance between two vectors
    pub fn distance(lhs: &Vector4, rhs: &Vector4) -> f32 {
        let dx = lhs.x - rhs.x;
        let dy = lhs.y - rhs.y;
        let dz = lhs.z - rhs.z;
        let dw = lhs.w - rhs.w;
        return ((dx * dx) + (dy * dy) + (dz * dz) + (dw * dw)).sqrt();
    }

    /// Compute the squared length of the vector
    pub fn length_sq(self) -> f32 {
        return (self.x * self.x) + (self.y * self.y) + (self.z * self.z) + (self.w * self.w);
    }

    /// Compute the length of the vector
    pub fn length(self) -> f32 {
        return ((self.x * self.x) + (self.y * self.y) + (self.z * self.z) + (self.w * self.w)).sqrt();
    }

    /// Compute the dot product of two vectors
    pub fn dot(lhs: &Vector4, rhs: &Vector4) -> f32 {
        return (lhs.x * rhs.x) + (lhs.y * rhs.y) + (lhs.z * rhs.z) + (lhs.w * rhs.w);
    }
}

impl ops::Add<Vector4> for Vector4 {
    type Output = Vector4;

    fn add(self, rhs: Vector4) -> Vector4 {
        return Vector4 { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z, w: self.w + rhs.w };
    }
}

impl ops::Sub<Vector4> for Vector4 {
    type Output = Vector4;

    fn sub(self, rhs: Vector4) -> Vector4 {
        return Vector4 { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z, w: self.w - rhs.w };
    }
}

impl ops::Mul<Vector4> for Vector4 {
    type Output = Vector4;

    fn mul(self, rhs: Vector4) -> Vector4 {
        return Vector4 { x: self.x * rhs.x, y: self.y * rhs.y, z: self.z * rhs.z, w: self.w * rhs.w };
    }
}

impl ops::Mul<f32> for Vector4 {
    type Output = Vector4;

    fn mul(self, rhs: f32) -> Vector4 {
        return Vector4 { x: self.x * rhs, y: self.y * rhs, z: self.z * rhs, w: self.w * rhs };
    }
}

impl ops::Mul<Vector4> for f32 {
    type Output = Vector4;

    fn mul(self, rhs: Vector4) -> Vector4 {
        return Vector4 { x: self * rhs.x, y: self * rhs.y, z: self * rhs.z, w: self * rhs.w };
    }
}

impl ops::Div<Vector4> for Vector4 {
    type Output = Vector4;

    fn div(self, rhs: Vector4) -> Vector4 {
        return Vector4 { x: self.x / rhs.x, y: self.y / rhs.y, z: self.z / rhs.z, w: self.w / rhs.w };
    }
}

impl ops::Div<f32> for Vector4 {
    type Output = Vector4;

    fn div(self, rhs: f32) -> Vector4 {
        return Vector4 { x: self.x / rhs, y: self.y / rhs, z: self.z / rhs, w: self.w / rhs };
    }
}

impl ops::Div<Vector4> for f32 {
    type Output = Vector4;

    fn div(self, rhs: Vector4) -> Vector4 {
        return Vector4 { x: self / rhs.x, y: self / rhs.y, z: self / rhs.z, w: self / rhs.w };
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quaternion {
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Quaternion {
        return Quaternion { x: x, y: y, z: z, w: w };
    }

    pub fn identity() -> Quaternion {
        return Quaternion { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };
    }

    /// Construct a new quaternion from the given rotations about each axis
    pub fn from_euler(euler_angles: Vector3) -> Quaternion {
        let cx = (euler_angles.x * 0.5).cos();
        let sx = (euler_angles.x * 0.5).sin();
        let cy = (euler_angles.y * 0.5).cos();
        let sy = (euler_angles.y * 0.5).sin();
        let cz = (euler_angles.z * 0.5).cos();
        let sz = (euler_angles.z * 0.5).sin();

        return Quaternion {
            x: sx * cy * cz - cx * sy * sz,
            y: cx * sy * cz + sx * cy * sz,
            z: cx * cy * sz - sx * sy * cz,
            w: cx * cy * cz + sx * sy * sz,
        };
    }

    /// Normalize the quaternion
    pub fn normalize(&mut self) {
        let mag = 1.0 / (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        self.x *= mag;
        self.y *= mag;
        self.z *= mag;
        self.w *= mag;
    }

    /// Invert the quaternion
    pub fn invert(&mut self) {
        let n = 1.0 / (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w);
        self.x *= -n;
        self.y *= -n;
        self.z *= -n;
        self.w *= n;
    }
}

impl ops::Mul<Quaternion> for Quaternion {
    type Output = Quaternion;

    fn mul(self, rhs: Quaternion) -> Quaternion {
        let x = self.x * rhs.w + self.y * rhs.z - self.z * rhs.y + self.w * rhs.x;
        let y = -self.x * rhs.z + self.y * rhs.w + self.z * rhs.x + self.w * rhs.y;
        let z = self.x * rhs.y - self.y * rhs.x + self.z * rhs.w + self.w * rhs.z;
        let w = -self.x * rhs.x - self.y * rhs.y - self.z * rhs.z + self.w * rhs.w;
        return Quaternion { x: x, y: y, z: z, w: w };
    }
}

impl ops::Mul<Vector3> for Quaternion {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Vector3 {
        let x = 2.0 * (self.y * rhs.z - self.z * rhs.y);
        let y = 2.0 * (self.z * rhs.x - self.x * rhs.z);
        let z = 2.0 * (self.x * rhs.y - self.y * rhs.x);

        let rx = rhs.x + x * self.w + (self.y * z - self.z * y);
        let ry = rhs.y + y * self.w + (self.z * x - self.x * z);
        let rz = rhs.z + z * self.w + (self.x * y - self.y * x);

        return Vector3 { x: rx, y: ry, z: rz };
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Matrix4x4 {
    pub m: [[f32;4];4],
}

impl Matrix4x4 {
    pub fn identity() -> Matrix4x4 {
        return Matrix4x4 { m: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ] };
    }

    /// Construct a translation matrix
    pub fn translation(translation: Vector3) -> Matrix4x4 {
        return Matrix4x4 { m: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [translation.x, translation.y, translation.z, 1.0],
        ] };
    }

    /// Construct a scale matrix
    pub fn scale(scale: Vector3) -> Matrix4x4 {
        return Matrix4x4 { m: [
            [scale.x, 0.0, 0.0, 0.0],
            [0.0, scale.y, 0.0, 0.0],
            [0.0, 0.0, scale.z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ] };
    }

    /// Construct a rotation matrix
    pub fn rotation(rotation: Quaternion) -> Matrix4x4 {
        let mut mat = Matrix4x4 { m: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ] };

        mat.m[0][0] = 1.0 - 2.0 * rotation.y * rotation.y - 2.0 * rotation.z * rotation.z;
        mat.m[0][1] = 2.0 * rotation.x * rotation.y - 2.0 * rotation.z * rotation.w;
        mat.m[0][2] = 2.0 * rotation.x * rotation.z + 2.0 * rotation.y * rotation.w;

        mat.m[1][0] = 2.0 * rotation.x * rotation.y + 2.0 * rotation.z * rotation.w;
        mat.m[1][1] = 1.0 - 2.0 * rotation.x * rotation.x - 2.0 * rotation.z * rotation.z;
        mat.m[1][2] = 2.0 * rotation.y * rotation.z - 2.0 * rotation.x * rotation.w;

        mat.m[2][0] = 2.0 * rotation.x * rotation.z - 2.0 * rotation.y * rotation.w;
        mat.m[2][1] = 2.0 * rotation.y * rotation.z + 2.0 * rotation.x * rotation.w;
        mat.m[2][2] = 1.0 - 2.0 * rotation.x * rotation.x - 2.0 * rotation.y * rotation.y;

        return mat;
    }

    /// Construct a new off-center orthographic projection matrix
    pub fn projection_ortho(left: f32, right: f32, top: f32, bottom: f32, near: f32, far: f32) -> Matrix4x4 {
        let scale_x = 2.0 / (right - left);
        let scale_y = 2.0 / (top - bottom);
        let scale_z = 1.0 / (near - far);

        let mut mat = Matrix4x4::identity();

        mat.m[0][0] = scale_x;
        mat.m[1][1] = scale_y;
        mat.m[2][2] = scale_z;

        mat.m[3][0] = (left + right) / (left - right);
        mat.m[3][1] = (top + bottom) / (bottom - top);
        mat.m[3][2] = near / (near - far);

        return mat;
    }

    /// Construct a new orthographic projection matrix using the given aspect ratio, scale, and near/far plane clip distances
    pub fn projection_ortho_aspect(aspect_ratio: f32, scale: f32, near: f32, far: f32) -> Matrix4x4 {
        let extent_x = scale * aspect_ratio * 0.5;
        let extent_y = scale * 0.5;

        return Matrix4x4::projection_ortho(-extent_x, extent_x, extent_y, -extent_y, near, far);
    }

    /// Construct a new perspective projection matrix using the given aspect ratio, field of view, and near/far plane clip distances
    pub fn projection_perspective(aspect_ratio: f32, field_of_view: f32, near: f32, far: f32) -> Matrix4x4 {
        let top = (field_of_view * 0.5).tan() * near;
        let bottom = -top;
        let right = top * aspect_ratio;
        let left = -right;

        let height = top - bottom;
        let width = right - left;

        let two_n = 2.0 * near;

        let mut mat = Matrix4x4 {m: [
            [ 0.0, 0.0, 0.0, 0.0 ],
            [ 0.0, 0.0, 0.0, 0.0 ],
            [ 0.0, 0.0, 0.0, 0.0 ],
            [ 0.0, 0.0, 0.0, 0.0 ],
        ]};

        mat.m[0][0] = two_n / width;
        mat.m[1][1] = two_n / height;
        mat.m[2][2] = far / (near - far);
        mat.m[2][3] = -1.0;
        mat.m[3][2] = (near * far) /
                    (near - far);

        return mat;
    }

    /// Load an identity matrix into the SIMD register
    pub fn load_identity_simd() {
        let m = Matrix4x4::identity();
        Matrix4x4::load_simd(&m);
    }

    /// Load a matrix into the SIMD register
    pub fn load_simd(matrix: &Matrix4x4) {
        unsafe { mat4_loadSIMD(matrix) };
    }

    /// Store the current value of the SIMD register to the given matrix
    pub fn store_simd(matrix: &mut Matrix4x4) {
        unsafe { mat4_storeSIMD(matrix) };
    }

    /// Multiply the matrix in the SIMD register by the given matrix
    pub fn mul_simd(matrix: &Matrix4x4) {
        unsafe { mat4_mulSIMD(matrix) };
    }

    /// Transform an array of vectors using the SIMD matrix register
    pub fn transform_vector_simd(data: &mut [Vector4]) {
        unsafe {
            let ptr = data.as_mut_ptr();
            let stride = size_of::<Vector4>();
            mat4_transformSIMD(ptr, ptr, data.len().try_into().unwrap(), stride.try_into().unwrap());
        }
    }

    /// Transform a field of an array of input vertex structs using the SIMD matrix register
    pub fn transform_vertex_simd<T>(data: &mut [T], field: FieldOffset<T,Vector4>) {
        unsafe {
            let fieldref = field.apply_ptr_mut(data.as_mut_ptr());
            let stride = size_of::<T>();
            mat4_transformSIMD(fieldref, fieldref, data.len().try_into().unwrap(), stride.try_into().unwrap());
        }
    }
}

impl ops::Mul<Vector4> for Matrix4x4 {
    type Output = Vector4;

    fn mul(self, rhs: Vector4) -> Vector4 {
        let x = (rhs.x * self.m[0][0]) + (rhs.y * self.m[1][0]) + (rhs.z * self.m[2][0]) + (rhs.w * self.m[3][0]);
        let y = (rhs.x * self.m[0][1]) + (rhs.y * self.m[1][1]) + (rhs.z * self.m[2][1]) + (rhs.w * self.m[3][1]);
        let z = (rhs.x * self.m[0][2]) + (rhs.y * self.m[1][2]) + (rhs.z * self.m[2][2]) + (rhs.w * self.m[3][2]);
        let w = (rhs.x * self.m[0][3]) + (rhs.y * self.m[1][3]) + (rhs.z * self.m[2][3]) + (rhs.w * self.m[3][3]);

        return Vector4 { x: x, y: y, z: z, w: w };
    }
}
