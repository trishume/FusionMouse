use std::f32::consts::PI;
use std::f32;

use cgmath::{Vector2, vec2};
use rand::{self, Rng};

pub struct LowPassFilter {
    first_time: bool,
    pub hat_x_prev: f32,
}

impl LowPassFilter {
    pub fn new() -> LowPassFilter {
        LowPassFilter { first_time: true, hat_x_prev: 0.0 }
    }

    pub fn filter(&mut self, x: f32, alpha: f32) -> f32 {
        if self.first_time {
            self.first_time = false;
            self.hat_x_prev = x;
        }
        let hatx = alpha*x + (1.0-alpha)*self.hat_x_prev;
        self.hat_x_prev = hatx;
        hatx
    }
}

pub struct OneEuroFilter {
    first_time: bool,
    mincutoff: f32,
    beta: f32,
    dcutoff: f32,
    xfilt: LowPassFilter,
    dxfilt: LowPassFilter,
}

impl OneEuroFilter {
    pub fn new(mincutoff: f32, beta: f32, dcutoff: f32) -> Self {
        OneEuroFilter {
            first_time: true,
            mincutoff, beta, dcutoff,
            xfilt: LowPassFilter::new(),
            dxfilt: LowPassFilter::new(),
        }
    }

    pub fn filter(&mut self, x: f32, dt: f32) -> f32 {
        let rate = 1.0/dt;
        let dx = if self.first_time {
            self.first_time = false;
            0.0
        } else {
            (x-self.xfilt.hat_x_prev)*rate
        };

        let edx = self.dxfilt.filter(dx, Self::alpha(rate, self.dcutoff));
        let cutoff = self.mincutoff + self.beta * edx.abs();
        self.xfilt.filter(x, Self::alpha(rate, cutoff))
    }

    fn alpha(rate: f32, cutoff: f32) -> f32 {
        let tau = 1.0 / (2.0*PI*cutoff);
        let te = 1.0/rate;
        1.0/(1.0+(tau/te))
    }
}

pub struct VecOneEuroFilter {
    xf: OneEuroFilter,
    yf: OneEuroFilter,
}

impl VecOneEuroFilter {
    pub fn new(mincutoff: f32, beta: f32, dcutoff: f32) -> Self {
        VecOneEuroFilter {
            xf: OneEuroFilter::new(mincutoff, beta, dcutoff),
            yf: OneEuroFilter::new(mincutoff, beta, dcutoff),
        }
    }

    pub fn filter(&mut self, x: Vector2<f32>, dt: f32) -> Vector2<f32> {
        vec2(self.xf.filter(x.x, dt), self.yf.filter(x.y, dt))
    }
}

/// Based on page 16 of Mathieu Nancel's "Mid-Air Pointing on Ultra-Walls" paper
/// See the paper for how to set the constants.
pub struct Acceleration {
    pub cd_min: f32,
    pub cd_max: f32,
    pub v_min: f32,
    pub v_max: f32,
    pub lambda: f32,
    pub ratio: f32,
}

impl Acceleration {
    pub fn transform(&self, diff: f32, dt: f32) -> f32 {
        let v_inf = self.ratio*(self.v_max - self.v_min) + self.v_min;
        let raw_vel = diff*dt;
        let exponent = -self.lambda*(raw_vel.abs() - v_inf);
        let cd = ((self.cd_max-self.cd_min)/(1.0+f32::exp(exponent)))+self.cd_min;
        diff * cd
    }
}

/// Round a number where the probability of rounding up is the fractional part
pub fn stochastic_round(x: f32) -> i32 {
    let mut rng = rand::thread_rng();
    let mut res = x.trunc();
    if rng.next_f32() < x.fract().abs() {
        res += res.signum();
    }
    res as i32
}
