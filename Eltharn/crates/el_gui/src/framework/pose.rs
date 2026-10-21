use el_math::Matrix3x2f;
#[derive(Clone, Debug)]
pub struct PoseStack {
    stack: Vec<Matrix3x2f>,
}
impl PoseStack {
    pub fn new() -> Self {
        Self {
            stack: vec![Matrix3x2f::new()],
        }
    }
    pub fn push_pose(&mut self) {
        self.stack.push(self.pose());
    }
    pub fn pop_pose(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }
    pub fn pose(&self) -> Matrix3x2f {
        *self.stack.last().expect("pose stack always has root pose")
    }
    pub fn translate(&mut self, x: f32, y: f32) {
        let translation = Matrix3x2f {
            m00: 1.0,
            m01: 0.0,
            m10: 0.0,
            m11: 1.0,
            m20: x,
            m21: y,
        };
        self.mul(translation);
    }
    pub fn scale(&mut self, x: f32, y: f32) {
        let scale = Matrix3x2f {
            m00: x,
            m01: 0.0,
            m10: 0.0,
            m11: y,
            m20: 0.0,
            m21: 0.0,
        };
        self.mul(scale);
    }
    pub fn rotate(&mut self, angle_rad: f32) {
        let c = angle_rad.cos();
        let s = angle_rad.sin();
        let rot = Matrix3x2f {
            m00: c,
            m01: s,
            m10: -s,
            m11: c,
            m20: 0.0,
            m21: 0.0,
        };
        self.mul(rot);
    }
    pub fn mul(&mut self, rhs: Matrix3x2f) {
        let lhs = self.pose();
        *self
            .stack
            .last_mut()
            .expect("pose stack always has root pose") = multiply(lhs, rhs);
    }
}
impl Default for PoseStack {
    fn default() -> Self {
        Self::new()
    }
}
pub fn multiply(a: Matrix3x2f, b: Matrix3x2f) -> Matrix3x2f {
    Matrix3x2f {
        m00: a.m00 * b.m00 + a.m10 * b.m01,
        m01: a.m01 * b.m00 + a.m11 * b.m01,
        m10: a.m00 * b.m10 + a.m10 * b.m11,
        m11: a.m01 * b.m10 + a.m11 * b.m11,
        m20: a.m00 * b.m20 + a.m10 * b.m21 + a.m20,
        m21: a.m01 * b.m20 + a.m11 * b.m21 + a.m21,
    }
}
pub fn transform_point(matrix: Matrix3x2f, x: f32, y: f32) -> (f32, f32) {
    (
        x * matrix.m00 + y * matrix.m10 + matrix.m20,
        x * matrix.m01 + y * matrix.m11 + matrix.m21,
    )
}
