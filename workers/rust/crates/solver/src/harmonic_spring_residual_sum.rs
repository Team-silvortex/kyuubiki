use super::Interval;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct IntervalSum {
    lower: Sum,
    upper: Sum,
}

impl IntervalSum {
    pub(super) fn add(&mut self, value: Interval) -> Result<(), String> {
        value.checked()?;
        let mut next = *self;
        next.lower.add(value.lower)?;
        next.upper.add(value.upper)?;
        *self = next;
        Ok(())
    }

    pub(super) fn interval(self) -> Result<Interval, String> {
        Interval {
            lower: self.lower.interval()?.lower,
            upper: self.upper.interval()?.upper,
        }
        .checked()
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Sum {
    high: f64,
    low: f64,
    radius: f64,
}

impl Sum {
    fn add(&mut self, value: f64) -> Result<(), String> {
        let (high, first_tail) = two_sum(self.high, value)?;
        let (low, lost_tail) = two_sum(self.low, first_tail)?;
        // high + low retains the first addition's exact tail. Only the
        // second addition's remaining tail enters the outward error radius.
        let radius = add_radius(self.radius, lost_tail.abs())?;
        *self = Self { high, low, radius };
        Ok(())
    }

    fn interval(self) -> Result<Interval, String> {
        let (center, tail) = two_sum(self.high, self.low)?;
        let radius = add_radius(self.radius, tail.abs())?;
        if radius == 0.0 {
            return Ok(Interval {
                lower: center,
                upper: center,
            });
        }
        Interval {
            lower: (center - radius).next_down(),
            upper: (center + radius).next_up(),
        }
        .checked()
    }
}

fn two_sum(first: f64, second: f64) -> Result<(f64, f64), String> {
    if !first.is_finite() || !second.is_finite() {
        return Err("harmonic residual sum contains a non-finite operand".into());
    }
    let high = first + second;
    let low = if first.abs() >= second.abs() {
        (first - high) + second
    } else {
        (second - high) + first
    };
    if !high.is_finite() || !low.is_finite() {
        return Err("harmonic residual sum exceeds the finite numeric range".into());
    }
    Ok((high, low))
}

fn add_radius(current: f64, extra: f64) -> Result<f64, String> {
    if !current.is_finite() || !extra.is_finite() || current < 0.0 || extra < 0.0 {
        return Err("harmonic residual sum contains an invalid error radius".into());
    }
    let next = if extra == 0.0 {
        current
    } else if current == 0.0 {
        extra
    } else {
        (current + extra).next_up()
    };
    if !next.is_finite() {
        return Err("harmonic residual sum error radius exceeds range".into());
    }
    Ok(next)
}

#[cfg(test)]
#[path = "harmonic_spring_residual_sum_tests.rs"]
mod tests;
