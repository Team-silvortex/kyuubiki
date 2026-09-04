use kyuubiki_protocol::{
    HeatBar1dNodeInput, SolveHarmonicSpring1dRequest, SolveTransientHeatBar1dRequest,
    SolveTransientSpring1dRequest, TransientHeatBar1dElementInput, TransientSpring1dElementInput,
    TransientSpring1dNodeInput,
};

const TRANSIENT_STEPS: usize = 4;

pub(crate) fn generate_transient_heat_bar_case(elements: usize) -> SolveTransientHeatBar1dRequest {
    let nodes = (0..=elements)
        .map(|index| HeatBar1dNodeInput {
            id: format!("th{index}"),
            x: index as f64,
            fix_temperature: index == 0,
            temperature: 293.15,
            heat_load: if index == elements { 1_000.0 } else { 0.0 },
        })
        .collect();
    let elements = (0..elements)
        .map(|index| TransientHeatBar1dElementInput {
            id: format!("the{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.01,
            conductivity: 45.0,
            density: 7_850.0,
            specific_heat: 470.0,
        })
        .collect();

    SolveTransientHeatBar1dRequest {
        nodes,
        elements,
        time_step: 0.1,
        steps: TRANSIENT_STEPS,
        history_stride: Some(TRANSIENT_STEPS),
    }
}

pub(crate) fn generate_transient_spring_1d_case(elements: usize) -> SolveTransientSpring1dRequest {
    let (nodes, elements) = dynamic_spring_chain(elements);
    SolveTransientSpring1dRequest {
        nodes,
        elements,
        time_step: 0.001,
        steps: TRANSIENT_STEPS,
        history_stride: Some(TRANSIENT_STEPS),
    }
}

pub(crate) fn generate_harmonic_spring_1d_case(elements: usize) -> SolveHarmonicSpring1dRequest {
    let (nodes, elements) = dynamic_spring_chain(elements);
    SolveHarmonicSpring1dRequest {
        nodes,
        elements,
        frequencies_hz: vec![2.0],
    }
}

fn dynamic_spring_chain(
    elements: usize,
) -> (
    Vec<TransientSpring1dNodeInput>,
    Vec<TransientSpring1dElementInput>,
) {
    let nodes = (0..=elements)
        .map(|index| TransientSpring1dNodeInput {
            id: format!("ds{index}"),
            x: index as f64,
            fix_x: index == 0,
            load_x: if index == elements { 100.0 } else { 0.0 },
            mass: 1.0,
            initial_displacement: 0.0,
            initial_velocity: 0.0,
        })
        .collect();
    let elements = (0..elements)
        .map(|index| TransientSpring1dElementInput {
            id: format!("dse{index}"),
            node_i: index,
            node_j: index + 1,
            stiffness: 30_000.0,
            damping: 4.0,
        })
        .collect();
    (nodes, elements)
}

#[cfg(test)]
mod tests {
    use super::{
        TRANSIENT_STEPS, generate_harmonic_spring_1d_case, generate_transient_heat_bar_case,
        generate_transient_spring_1d_case,
    };

    #[test]
    fn transient_generators_bound_history_for_large_profiles() {
        let heat = generate_transient_heat_bar_case(16);
        let spring = generate_transient_spring_1d_case(16);

        assert_eq!(heat.history_stride, Some(TRANSIENT_STEPS));
        assert_eq!(spring.history_stride, Some(TRANSIENT_STEPS));
        assert_eq!(heat.nodes.len(), 17);
        assert_eq!(spring.elements.len(), 16);
    }

    #[test]
    fn harmonic_generator_uses_a_damped_path_topology() {
        let request = generate_harmonic_spring_1d_case(16);

        assert_eq!(request.frequencies_hz, vec![2.0]);
        assert!(request.elements.iter().all(|element| element.damping > 0.0));
        assert!(request.nodes[0].fix_x);
    }
}
