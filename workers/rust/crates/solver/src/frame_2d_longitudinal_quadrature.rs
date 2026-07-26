pub(super) const ADAPTIVE_POINT_COUNT: usize = 29;

const GAUSS_STATIONS_2: [(f64, f64); 2] = [
    (0.211_324_865_405_187_13, 0.5),
    (0.788_675_134_594_812_9, 0.5),
];
const GAUSS_STATIONS_3: [(f64, f64); 3] = [
    (0.112_701_665_379_258_3, 0.277_777_777_777_777_8),
    (0.5, 0.444_444_444_444_444_4),
    (0.887_298_334_620_741_7, 0.277_777_777_777_777_8),
];
const GAUSS_STATIONS_4: [(f64, f64); 4] = [
    (0.069_431_844_202_973_71, 0.173_927_422_568_726_92),
    (0.330_009_478_207_571_87, 0.326_072_577_431_273_05),
    (0.669_990_521_792_428_1, 0.326_072_577_431_273_05),
    (0.930_568_155_797_026_2, 0.173_927_422_568_726_92),
];
const GAUSS_STATIONS_8: [(f64, f64); 8] = [
    (0.019_855_071_751_231_9, 0.050_614_268_145_188_13),
    (0.101_666_761_293_186_64, 0.111_190_517_226_687_24),
    (0.237_233_795_041_835_5, 0.156_853_322_938_943_65),
    (0.408_282_678_752_175_1, 0.181_341_891_689_180_88),
    (0.591_717_321_247_824_8, 0.181_341_891_689_180_88),
    (0.762_766_204_958_164_5, 0.156_853_322_938_943_65),
    (0.898_333_238_706_813_4, 0.111_190_517_226_687_24),
    (0.980_144_928_248_768_1, 0.050_614_268_145_188_13),
];
const GAUSS_STATIONS_12: [(f64, f64); 12] = [
    (0.009_219_682_876_640_4, 0.023_587_668_193_255_9),
    (0.047_941_371_814_762_55, 0.053_469_662_997_659_2),
    (0.115_048_662_902_847_65, 0.080_039_164_271_673_1),
    (0.206_341_022_856_691_26, 0.101_583_713_361_532_96),
    (0.316_084_250_500_909_9, 0.116_746_268_269_177_4),
    (0.437_383_295_744_265_54, 0.124_573_522_906_701_39),
    (0.562_616_704_255_734_5, 0.124_573_522_906_701_39),
    (0.683_915_749_499_090_1, 0.116_746_268_269_177_4),
    (0.793_658_977_143_308_7, 0.101_583_713_361_532_96),
    (0.884_951_337_097_152_3, 0.080_039_164_271_673_1),
    (0.952_058_628_185_237_5, 0.053_469_662_997_659_2),
    (0.990_780_317_123_359_6, 0.023_587_668_193_255_9),
];

pub(super) fn gauss_stations(point_count: usize) -> &'static [(f64, f64)] {
    match point_count {
        3 => &GAUSS_STATIONS_3,
        4 => &GAUSS_STATIONS_4,
        8 => &GAUSS_STATIONS_8,
        12 => &GAUSS_STATIONS_12,
        _ => &GAUSS_STATIONS_2,
    }
}

pub(super) fn adaptive_history_offset(point_count: usize, fiber_count: usize) -> usize {
    match point_count {
        3 => 2 * fiber_count,
        4 => 5 * fiber_count,
        8 => 9 * fiber_count,
        12 => 17 * fiber_count,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{ADAPTIVE_POINT_COUNT, adaptive_history_offset, gauss_stations};

    #[test]
    fn twelve_point_rule_integrates_degree_twenty_two_monomial() {
        let integral = gauss_stations(12)
            .iter()
            .map(|(station, weight)| weight * station.powi(22))
            .sum::<f64>();

        assert!((integral - 1.0 / 23.0).abs() < 1.0e-14);
    }

    #[test]
    fn adaptive_history_offsets_form_a_contiguous_fixed_identity_layout() {
        let fiber_count = 7;
        let spans = [2, 3, 4, 8, 12].map(|order| {
            let start = adaptive_history_offset(order, fiber_count);
            (start, start + order * fiber_count)
        });

        assert_eq!(spans[0].0, 0);
        assert!(spans.windows(2).all(|pair| pair[0].1 == pair[1].0));
        assert_eq!(spans[4].1, ADAPTIVE_POINT_COUNT * fiber_count);
    }
}
