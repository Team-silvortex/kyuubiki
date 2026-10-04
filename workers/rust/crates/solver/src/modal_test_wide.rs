// Test-only bounded double-double arithmetic. TwoSum / FMA transforms follow
// https://www.tuhh.de/ti3/paper/rump/OgRuOi05.pdf; this is not interval arithmetic.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Wide {
    pub(crate) high: f64,
    pub(crate) low: f64,
}

impl Wide {
    pub(crate) fn from(value: f64) -> Self {
        assert!(value.is_finite());
        Self {
            high: value,
            low: 0.0,
        }
    }

    fn parts(left: f64, right: f64) -> Self {
        let high = left + right;
        let virtual_right = high - left;
        let low = (left - (high - virtual_right)) + (right - virtual_right);
        assert!(high.is_finite() && low.is_finite());
        Self { high, low }
    }

    pub(crate) fn add(self, other: Self) -> Self {
        let head = Self::parts(self.high, other.high);
        let tail = Self::parts(self.low, other.low);
        let middle = Self::parts(head.high, head.low + tail.high);
        Self::parts(middle.high, middle.low + tail.low)
    }

    pub(crate) fn sub(self, other: Self) -> Self {
        self.add(Self {
            high: -other.high,
            low: -other.low,
        })
    }

    pub(crate) fn mul(self, other: Self) -> Self {
        let high = self.high * other.high;
        let error = self.high.mul_add(other.high, -high);
        let cross = self.high * other.low + self.low * other.high;
        Self::parts(high, error + cross + self.low * other.low)
    }

    pub(crate) fn div(self, other: Self) -> Self {
        assert!(other.high != 0.0);
        let first = Self::from(self.high / other.high);
        let second = Self::from(self.sub(other.mul(first)).high / other.high);
        let combined = first.add(second);
        let third = Self::from(self.sub(other.mul(combined)).high / other.high);
        combined.add(third)
    }

    pub(crate) fn rounded(self) -> f64 {
        self.high + self.low
    }
}
