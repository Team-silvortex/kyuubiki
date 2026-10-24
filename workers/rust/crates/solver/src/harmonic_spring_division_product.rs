const FRACTION: u64 = (1 << 52) - 1;

pub(super) struct Product {
    pub(super) units: u128,
    pub(super) power: i64,
    pub(super) negative: bool,
}

impl Product {
    pub(super) fn new((left, right, extra): (f64, f64, i32)) -> Result<Self, String> {
        let decode = |value: f64| {
            let bits = value.to_bits();
            let field = ((bits >> 52) & 0x7ff) as i64;
            (
                (bits & FRACTION) | if field == 0 { 0 } else { 1 << 52 },
                if field == 0 { -1074 } else { field - 1075 },
            )
        };
        if !left.is_finite() || !right.is_finite() {
            return Err("complex exact division requires finite operands".into());
        }
        let (a, ap) = decode(left);
        let (b, bp) = decode(right);
        Ok(Self {
            units: u128::from(a) * u128::from(b),
            power: ap + bp + i64::from(extra),
            negative: left.is_sign_negative() ^ right.is_sign_negative(),
        })
    }
}
