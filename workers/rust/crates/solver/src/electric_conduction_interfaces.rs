use crate::linear_algebra::{SparseMatrix, add_at};
use kyuubiki_protocol::{
    ElectricConductionContactResult, ElectricConductionTerminalResult,
    SolveElectricConductionPlaneQuad2dRequest,
};
use std::collections::HashSet;

pub(crate) fn assemble_interfaces(
    request: &SolveElectricConductionPlaneQuad2dRequest,
    conductance: &mut SparseMatrix,
    applied_currents: &mut [f64],
) {
    for contact in &request.contact_interfaces {
        let value = contact.contact_resistance_ohm.recip();
        add_at(conductance, contact.node_i, contact.node_i, value);
        add_at(conductance, contact.node_i, contact.node_j, -value);
        add_at(conductance, contact.node_j, contact.node_i, -value);
        add_at(conductance, contact.node_j, contact.node_j, value);
    }
    for terminal in &request.terminals {
        let value = terminal.impedance_ohm.recip();
        add_at(conductance, terminal.node, terminal.node, value);
        applied_currents[terminal.node] += value * terminal.external_potential_v;
    }
}

pub(crate) fn recover_contacts(
    request: &SolveElectricConductionPlaneQuad2dRequest,
    potentials: &[f64],
) -> Vec<ElectricConductionContactResult> {
    request
        .contact_interfaces
        .iter()
        .enumerate()
        .map(|(index, contact)| {
            let voltage_drop_v = potentials[contact.node_i] - potentials[contact.node_j];
            let current_i_to_j_a = voltage_drop_v / contact.contact_resistance_ohm;
            ElectricConductionContactResult {
                index,
                id: contact.id.clone(),
                node_i: contact.node_i,
                node_j: contact.node_j,
                contact_resistance_ohm: contact.contact_resistance_ohm,
                voltage_drop_v,
                current_i_to_j_a,
                joule_power_w: current_i_to_j_a.powi(2) * contact.contact_resistance_ohm,
            }
        })
        .collect()
}

pub(crate) fn recover_terminals(
    request: &SolveElectricConductionPlaneQuad2dRequest,
    potentials: &[f64],
) -> Vec<ElectricConductionTerminalResult> {
    request
        .terminals
        .iter()
        .enumerate()
        .map(|(index, terminal)| {
            let node_potential_v = potentials[terminal.node];
            let voltage_drop_v = terminal.external_potential_v - node_potential_v;
            let current_into_domain_a = voltage_drop_v / terminal.impedance_ohm;
            ElectricConductionTerminalResult {
                index,
                id: terminal.id.clone(),
                node: terminal.node,
                external_potential_v: terminal.external_potential_v,
                node_potential_v,
                impedance_ohm: terminal.impedance_ohm,
                current_into_domain_a,
                impedance_joule_power_w: current_into_domain_a.powi(2) * terminal.impedance_ohm,
                power_delivered_to_domain_w: node_potential_v * current_into_domain_a,
                source_power_w: terminal.external_potential_v * current_into_domain_a,
            }
        })
        .collect()
}

pub(crate) fn terminal_currents_by_node(
    node_count: usize,
    terminals: &[ElectricConductionTerminalResult],
) -> Vec<f64> {
    let mut currents = vec![0.0; node_count];
    for terminal in terminals {
        currents[terminal.node] += terminal.current_into_domain_a;
    }
    currents
}

pub(crate) fn validate_interfaces(
    request: &SolveElectricConductionPlaneQuad2dRequest,
) -> Result<(), String> {
    let node_count = request.nodes.len();
    let mut contact_ids = HashSet::new();
    if request.contact_interfaces.iter().any(|contact| {
        contact.id.trim().is_empty()
            || !contact_ids.insert(contact.id.as_str())
            || contact.node_i >= node_count
            || contact.node_j >= node_count
            || contact.node_i == contact.node_j
            || !contact.contact_resistance_ohm.is_finite()
            || contact.contact_resistance_ohm <= 0.0
    }) {
        return Err("electric conduction contact interface parameters are invalid".to_string());
    }
    let mut terminal_ids = HashSet::new();
    if request.terminals.iter().any(|terminal| {
        terminal.id.trim().is_empty()
            || !terminal_ids.insert(terminal.id.as_str())
            || terminal.node >= node_count
            || !terminal.external_potential_v.is_finite()
            || !terminal.impedance_ohm.is_finite()
            || terminal.impedance_ohm <= 0.0
    }) {
        return Err("electric conduction terminal parameters are invalid".to_string());
    }
    Ok(())
}
