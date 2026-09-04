const MAX_TRANSIENT_HISTORY_SCALARS: usize = 32 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct TransientHistoryPlan {
    stride: usize,
    frame_count: usize,
}

impl TransientHistoryPlan {
    pub(crate) fn new(
        label: &str,
        node_count: usize,
        steps: usize,
        configured_stride: Option<usize>,
        scalar_fields_per_node: usize,
    ) -> Result<Self, String> {
        let stride = configured_stride.unwrap_or(1);
        if stride == 0 {
            return Err(format!("{label} history_stride must be positive"));
        }

        let periodic_frames = steps / stride;
        let final_frame = usize::from(steps % stride != 0);
        let frame_count = 1_usize
            .checked_add(periodic_frames)
            .and_then(|count| count.checked_add(final_frame))
            .ok_or_else(|| format!("{label} history frame count overflows usize"))?;
        let scalar_count = node_count
            .checked_mul(scalar_fields_per_node)
            .and_then(|count| count.checked_mul(frame_count))
            .ok_or_else(|| format!("{label} history sample count overflows usize"))?;
        if scalar_count > MAX_TRANSIENT_HISTORY_SCALARS {
            return Err(format!(
                "{label} history would store {scalar_count} scalar samples; increase history_stride or reduce nodes/steps (limit {MAX_TRANSIENT_HISTORY_SCALARS})"
            ));
        }

        Ok(Self {
            stride,
            frame_count,
        })
    }

    pub(crate) fn frame_count(&self) -> usize {
        self.frame_count
    }

    pub(crate) fn captures(&self, step: usize, final_step: usize) -> bool {
        step == final_step || step.is_multiple_of(self.stride)
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_TRANSIENT_HISTORY_SCALARS, TransientHistoryPlan};

    #[test]
    fn samples_periodic_steps_and_always_keeps_the_final_state() {
        let plan = TransientHistoryPlan::new("test", 10, 8, Some(3), 2).expect("valid plan");
        assert_eq!(plan.frame_count(), 4);
        assert!(!plan.captures(1, 8));
        assert!(plan.captures(3, 8));
        assert!(plan.captures(8, 8));
    }

    #[test]
    fn rejects_zero_stride_and_oversized_payloads_before_allocation() {
        assert!(TransientHistoryPlan::new("test", 2, 2, Some(0), 1).is_err());
        let error =
            TransientHistoryPlan::new("test", MAX_TRANSIENT_HISTORY_SCALARS / 2 + 1, 1, None, 1)
                .expect_err("oversized history should fail closed");
        assert!(error.contains("increase history_stride"));
    }
}
