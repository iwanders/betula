/// An object to create coordinates on an Archimedean spiral
pub struct Spiral {
    /// The x coordinate of the center of the spiral.
    pub x: f64,
    /// The y coordinate of the center of the spiral.
    pub y: f64,
    /// The speed at which the spiral is traversed (in arc speed).
    pub speed: f64,
    /// The maximum radius of the spiral, at this point the parameter is reset.
    pub max_radius: f64,
    /// The a component in the (a + b * t) spiral radius.
    pub a: f64,
    /// The b component in the (a + b * t) spiral radius.
    pub b: f64,
    /// The current parameter, advanced by dt * speed each advance.
    pub parameter: f64,
    /// The minimum radius, below this the path parameter is advanced.
    pub min_radius: f64,
    /// The timestep to achieve the minimum radius.
    pub min_radius_dt: f64,
}

impl Spiral {
    /// Create a new spiral with a max radius.
    pub fn new(c: (f64, f64), a: f64, b: f64, speed: f64, max_radius: f64) -> Self {
        let (x, y) = c;
        let mut v = Self {
            x,
            y,
            speed,
            a,
            b,
            max_radius,
            parameter: 0.0,
            min_radius: 0.0,
            min_radius_dt: 0.01, // prevent footgun for infinite loops.
        };
        v.reset();
        v
    }

    /// Consume the value and return an initialised value.
    ///
    /// Commonly used after constructing one.
    pub fn initialised(mut self) -> Self {
        self.reset();
        self
    }

    /// Reset the spiral to its minimum radius.
    pub fn reset(&mut self) {
        self.parameter = 0.0;
        self.advance_to_radius(self.min_radius, self.min_radius_dt);
    }

    /// Advance the spiral to a certain radius.
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

    /// Advance the spiral with a dt and return the new position.
    pub fn advance(&mut self, dt: f64) -> (f64, f64) {
        // use std::f64::consts::PI;
        // https://gamedev.stackexchange.com/a/16756
        //   spiral fun: (cos(t) * f(t), sin(t) * f(t))
        //   w(t) = V / (2 * pi * f (t))
        // Archimedean:
        //   fun (cos(t) * t, sin(t) * t)
        //   w(t) = V / (2 * pi * t)
        // Parametrized:
        //    f(t) = r = a + b*t
        //    w(t) = v / (2 * pi * (a + b * t));
        // The above is ignored for now, I didn't really need it just yet.
        self.parameter = self.parameter + dt * self.speed;
        let t = self.parameter;
        let fv = self.a + self.b * t;

        let x = t.cos() * fv;
        let y = t.sin() * fv;
        let r = (x * x + y * y).sqrt();

        if r >= self.max_radius {
            self.reset();
        }

        (x + self.x, y + self.y)
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

        for _i in 0..100 {
            let p = spiral.advance(dt);
            points_f64.push(p);
            points_i32.push((p.0 as i32, p.1 as i32));
        }
        println!("points_f64: {points_f64:?}");
        println!("points_i32: {points_i32:?}");
    }
}
