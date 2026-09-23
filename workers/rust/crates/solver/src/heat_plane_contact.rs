use crate::linear_algebra::{SparseMatrix, add_at};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use kyuubiki_protocol::{HeatPlaneContactInput, HeatPlaneContactResult, HeatPlaneNodeInput};
use std::collections::{HashMap, HashSet};

pub(super) struct HeatContact {
    map: [usize; 4],
    area: f64,
    weight: f64,
}

struct BulkEdge {
    count: usize,
    thickness: f64,
    interior_node: usize,
}

fn edge_key([left, right]: [usize; 2]) -> (usize, usize) {
    (left.min(right), left.max(right))
}

pub(super) fn prepare_heat_contacts<const N: usize>(
    nodes: &[HeatPlaneNodeInput],
    contacts: &[HeatPlaneContactInput],
    elements: impl ExactSizeIterator<Item = ([usize; N], f64)>,
) -> Result<Vec<HeatContact>, String> {
    if contacts.is_empty() {
        return Ok(Vec::new());
    }
    checkpoint(SolverStage::ElementPrecompute, 0)?;
    if contacts.len() > elements.len().saturating_mul(N) / 2 {
        return Err("thermal contact count exceeds the available bulk edges".into());
    }
    let mut components = Components::new(nodes.len());
    // Only index requested interface edges, not every edge in a large bulk mesh.
    let mut edges: HashMap<(usize, usize), BulkEdge> = contacts
        .iter()
        .flat_map(|contact| [contact.side_a, contact.side_b])
        .map(|side| {
            (
                edge_key(side),
                BulkEdge {
                    count: 0,
                    thickness: 0.0,
                    interior_node: 0,
                },
            )
        })
        .collect();
    let element_count = elements.len();
    for (index, (map, thickness)) in elements.enumerate() {
        for edge in 0..N {
            components.union(map[0], map[edge]);
            if let Some(boundary) = edges.get_mut(&edge_key([map[edge], map[(edge + 1) % N]])) {
                boundary.count += 1;
                boundary.thickness = thickness;
                // Scalar quads use (i,j,k) and (i,k,l), including concave
                // shapes. Select the triangle actually adjacent to this edge.
                boundary.interior_node = match (N, edge) {
                    (4, 1) => map[0],
                    (4, 3) => map[2],
                    _ => map[(edge + 2) % N],
                };
            }
        }
        checkpoint_chunk(SolverStage::ElementPrecompute, index + 1, element_count)?;
    }
    let mut ids = HashSet::new();
    let mut used_edges = HashSet::new();
    let mut prepared = Vec::with_capacity(contacts.len());
    for (index, contact) in contacts.iter().enumerate() {
        let error = |reason: &str| format!("thermal contact {}: {reason}", contact.id);
        if contact.id.trim().is_empty() || !ids.insert(contact.id.as_str()) {
            return Err(error("IDs must be nonblank and unique"));
        }
        let indices = [
            contact.side_a[0],
            contact.side_a[1],
            contact.side_b[0],
            contact.side_b[1],
        ];
        for (position, node) in indices.iter().enumerate() {
            if *node >= nodes.len() || indices[..position].contains(node) {
                return Err(error(
                    "requires four distinct known nodes on independent sides",
                ));
            }
        }
        let resistance = contact.thermal_resistance_m2_k_w;
        if !resistance.is_finite() || resistance <= 0.0 {
            return Err(error(
                "area-specific resistance must be finite and positive",
            ));
        }
        let key_a = edge_key(contact.side_a);
        let key_b = edge_key(contact.side_b);
        let Some((edge_a, edge_b)) = edges.get(&key_a).zip(edges.get(&key_b)) else {
            return Err(error("each side must be a bulk element boundary edge"));
        };
        if edge_a.count != 1 || edge_b.count != 1 {
            return Err(error(
                "a missing, interior or nonmanifold edge cannot be a contact side",
            ));
        }
        if !used_edges.insert(key_a) || !used_edges.insert(key_b) {
            return Err(error(
                "a boundary edge cannot participate in multiple contacts",
            ));
        }
        if (edge_a.thickness - edge_b.thickness).abs()
            > 1.0e-12 * edge_a.thickness.max(edge_b.thickness)
        {
            return Err(error("adjacent element thicknesses must match"));
        }
        let [a0, a1] = contact.side_a;
        let distance =
            |i: usize, j: usize| (nodes[i].x - nodes[j].x).hypot(nodes[i].y - nodes[j].y);
        let length = distance(a0, a1);
        if !length.is_finite() || length <= 0.0 {
            return Err(error("edge length must be finite and positive"));
        }
        // Use local edge scale, never the absolute world-coordinate magnitude.
        let aligned = |[b0, b1]: [usize; 2]| {
            distance(a0, b0) <= 1.0e-10 * length && distance(a1, b1) <= 1.0e-10 * length
        };
        let side_b = if aligned(contact.side_b) {
            contact.side_b
        } else {
            let reversed = [contact.side_b[1], contact.side_b[0]];
            if !aligned(reversed) {
                return Err(error("sides must have coincident matching endpoints"));
            }
            reversed
        };
        let side = |interior: usize| {
            (nodes[a1].x - nodes[a0].x) * (nodes[interior].y - nodes[a0].y)
                - (nodes[a1].y - nodes[a0].y) * (nodes[interior].x - nodes[a0].x)
        };
        let left = side(edge_a.interior_node);
        let right = side(edge_b.interior_node);
        if !left.is_finite()
            || !right.is_finite()
            || left == 0.0
            || right == 0.0
            || left.is_sign_positive() == right.is_sign_positive()
        {
            return Err(error(
                "adjacent bulk elements must lie on opposite sides of the interface",
            ));
        }
        let area = length * edge_a.thickness;
        let conductance = area / resistance;
        let weight = conductance / 6.0;
        let reconstructed_conductance = 6.0 * weight;
        if !area.is_finite()
            || area <= 0.0
            || !conductance.is_finite()
            || !weight.is_finite()
            || weight <= 0.0
            || !reconstructed_conductance.is_finite()
            || (reconstructed_conductance - conductance).abs() / conductance > 1.0e-12
        {
            return Err(error(
                "contact area or integrated conductance is not representable",
            ));
        }
        components.union(a0, side_b[0]);
        prepared.push(HeatContact {
            map: [a0, a1, side_b[0], side_b[1]],
            area,
            weight,
        });
        checkpoint_chunk(SolverStage::ElementPrecompute, index + 1, contacts.len())?;
    }
    components.validate_anchors(nodes)?;
    Ok(prepared)
}

pub(super) fn assemble_heat_contacts(
    contacts: &[HeatContact],
    stiffness: &mut SparseMatrix,
) -> Result<(), String> {
    for (index, contact) in contacts.iter().enumerate() {
        // Exact linear-edge integration: G/6 * [[2,1],[1,2]], with equal
        // diagonal blocks and negative cross blocks. No heat is generated.
        for row in 0..2 {
            for column in 0..2 {
                let value = contact.weight * if row == column { 2.0 } else { 1.0 };
                add_at(stiffness, contact.map[row], contact.map[column], value);
                add_at(
                    stiffness,
                    contact.map[row + 2],
                    contact.map[column + 2],
                    value,
                );
                add_at(stiffness, contact.map[row], contact.map[column + 2], -value);
                add_at(stiffness, contact.map[row + 2], contact.map[column], -value);
            }
        }
        checkpoint_chunk(SolverStage::ElementAssembly, index + 1, contacts.len())?;
    }
    Ok(())
}

pub(super) fn recover_heat_contacts(
    inputs: &[HeatPlaneContactInput],
    contacts: &[HeatContact],
    relative_temperatures: &[f64],
) -> Result<Vec<HeatPlaneContactResult>, String> {
    let mut results = Vec::with_capacity(contacts.len());
    for (index, (input, contact)) in inputs.iter().zip(contacts).enumerate() {
        let [a0, a1, b0, b1] = contact.map;
        let jump = [
            relative_temperatures[a0] - relative_temperatures[b0],
            relative_temperatures[a1] - relative_temperatures[b1],
        ];
        let diagonal = 2.0 * contact.weight;
        let nodal_flow = [
            diagonal * jump[0] + contact.weight * jump[1],
            contact.weight * jump[0] + diagonal * jump[1],
        ];
        let flow = nodal_flow[0] + nodal_flow[1];
        let flux = flow / contact.area;
        if jump
            .iter()
            .chain(&nodal_flow)
            .any(|value| !value.is_finite())
            || !flow.is_finite()
            || !flux.is_finite()
            || (flow != 0.0 && flux == 0.0)
            || (jump.iter().any(|value| *value != 0.0) && nodal_flow == [0.0, 0.0])
        {
            return Err(format!(
                "thermal contact {}: temperature jump or heat flow is not representable",
                input.id
            ));
        }
        results.push(HeatPlaneContactResult {
            index,
            id: input.id.clone(),
            side_a: [a0, a1],
            side_b: [b0, b1],
            thermal_resistance_m2_k_w: input.thermal_resistance_m2_k_w,
            area_m2: contact.area,
            average_temperature_jump_k: jump[0].midpoint(jump[1]),
            max_abs_temperature_jump_k: jump[0].abs().max(jump[1].abs()),
            heat_flow_a_to_b_w: flow,
            heat_flux_a_to_b_w_m2: flux,
            nodal_heat_flow_a_to_b_w: nodal_flow,
        });
        checkpoint_chunk(SolverStage::ResultElements, index + 1, contacts.len())?;
    }
    Ok(results)
}

struct Components {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl Components {
    fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
            rank: vec![0; count],
        }
    }

    fn root(&mut self, node: usize) -> usize {
        let mut root = node;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut current = node;
        while self.parent[current] != current {
            let next = self.parent[current];
            self.parent[current] = root;
            current = next;
        }
        root
    }

    fn union(&mut self, a: usize, b: usize) {
        let mut a = self.root(a);
        let mut b = self.root(b);
        if a == b {
            return;
        }
        if self.rank[a] < self.rank[b] {
            std::mem::swap(&mut a, &mut b);
        }
        self.parent[b] = a;
        if self.rank[a] == self.rank[b] {
            self.rank[a] += 1;
        }
    }

    fn validate_anchors(&mut self, nodes: &[HeatPlaneNodeInput]) -> Result<(), String> {
        let mut anchored = vec![false; nodes.len()];
        for (index, node) in nodes.iter().enumerate() {
            if node.fix_temperature {
                anchored[self.root(index)] = true;
            }
            checkpoint_chunk(SolverStage::ElementPrecompute, index + 1, nodes.len())?;
        }
        for (index, node) in nodes.iter().enumerate() {
            if !anchored[self.root(index)] {
                return Err(format!(
                    "thermal contact topology: component containing node {index} ({}) has no temperature support",
                    node.id
                ));
            }
            checkpoint_chunk(SolverStage::ElementPrecompute, index + 1, nodes.len())?;
        }
        Ok(())
    }
}
