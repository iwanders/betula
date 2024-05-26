pub struct Spiral {
    pub x: f64,
    pub y: f64,
    pub speed: f64,
    pub max_radius: f64,
    pub a: f64,
    pub b: f64,
    pub parameter: f64,
}
impl Spiral {
    pub fn new(c: (f64, f64), a: f64, b: f64, speed: f64, max_radius: f64) -> Self {
        let (x, y) = c;
        Self {
            x,
            y,
            speed,
            a,
            b,
            max_radius,
            parameter: 0.0,
        }
    }

    pub fn advance_to_radius(&mut self, radius: f64, dt: f64) -> (f64, f64) {
        let (x, y) = self.advance(0.0);
        let (x, y) = (x - self.x, y - self.y);
        let mut r = (x * x + y * y).sqrt();
        // println!("current: {x}, {y} -> {r}   {radius}");
        while r < radius {
            let (x, y) = self.advance(dt);
            let (x, y) = (x - self.x, y - self.y);
            r = (x * x + y * y).sqrt();
            // println!("   current: {x}, {y} -> {r}");
        }
        self.advance(0.0)
    }

    pub fn advance(&mut self, dt: f64) -> (f64, f64) {
        // use std::f64::consts::PI;
        // https://gamedev.stackexchange.com/a/16756
        //   spiral fun: (cos(t) * f(t), sin(t) * f(t))
        //   w(t) = V / (2 * pi * f (t))
        // Archimedal:
        //   fun (cos(t) * t, sin(t) * t)
        //   w(t) = V / (2 * pi * t)
        // Parametrized:
        //    f(t) = r = a + b*t
        //    w(t) = v / (2 * pi * (a + b * t));

        self.parameter = self.parameter + dt * self.speed;
        let t = self.parameter;
        let fv = self.a + self.b * t;

        let x = t.cos() * fv;
        let y = t.sin() * fv;
        let r = (x * x + y * y).sqrt();

        if r >= self.max_radius {
            self.parameter = 0.0;
        }

        (x + self.x, y + self.y)
    }

    pub fn parameter(&self) -> f64 {
        self.parameter
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_spiral() {
        // https://www.desmos.com/calculator/hv3yyi8bln
        let mut points_f64 = vec![];
        let mut points_i32 = vec![];

        let a = 35.0;
        let b = 10.0;
        let speed = 10.0;
        let max_radius = 1000.0;
        let mut spiral = Spiral::new((0.0, 0.0), a, b, speed, max_radius);
        let dt = 0.1;

        spiral.advance_to_radius(400.0, 0.1);

        for i in 0..100 {
            let p = spiral.advance(dt);
            points_f64.push(p);
            points_i32.push((p.0 as i32, p.1 as i32));
        }
        println!("points_f64: {points_f64:?}");
        println!("points_i32: {points_i32:?}");
    }
}
