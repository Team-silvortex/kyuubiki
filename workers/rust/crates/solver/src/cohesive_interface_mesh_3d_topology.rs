use crate::rigid_body_restraints_3d::rigid_body_restraint_rank;
use kyuubiki_protocol::SolveCohesiveInterfaceMesh3dRequest;
use std::collections::BTreeMap;

pub(crate) fn validate_topology_and_restraints(
    request: &SolveCohesiveInterfaceMesh3dRequest,
) -> Result<(), String> {
    let mut parent = (0..request.nodes.len()).collect::<Vec<_>>();
    let mut rank = vec![0_u8; request.nodes.len()];
    let mut referenced = vec![false; request.nodes.len()];
    for element in &request.elements {
        connect_element(
            &mut parent,
            &mut rank,
            &mut referenced,
            [
                element.lower_a,
                element.lower_b,
                element.lower_c,
                element.upper_a,
                element.upper_b,
                element.upper_c,
            ],
        );
    }
    for element in &request.host_tetrahedra {
        connect_element(
            &mut parent,
            &mut rank,
            &mut referenced,
            [
                element.node_a,
                element.node_b,
                element.node_c,
                element.node_d,
            ],
        );
    }
    if let Some(index) = referenced.iter().position(|is_referenced| !is_referenced) {
        return Err(format!(
            "cohesive interface mesh 3d node {index} ({}) is not referenced by an interface or host element",
            request.nodes[index].id,
        ));
    }

    let mut components = BTreeMap::<usize, Vec<usize>>::new();
    for node in 0..request.nodes.len() {
        let root = find(&mut parent, node);
        components.entry(root).or_default().push(node);
    }
    let mut components = components.into_values().collect::<Vec<_>>();
    components.sort_by_key(|nodes| nodes[0]);
    for (component_index, component) in components.iter().enumerate() {
        let restraint_rank = rigid_body_restraint_rank(
            component,
            |index| {
                let node = &request.nodes[index];
                [node.x, node.y, node.z]
            },
            |index| request.nodes[index].fixed,
        );
        if restraint_rank < 6 {
            let first_node = component[0];
            return Err(format!(
                "cohesive interface mesh 3d component {component_index} (first node {first_node}:{}) restrains rigid-body rank {restraint_rank}/6",
                request.nodes[first_node].id,
            ));
        }
    }
    Ok(())
}

fn connect_element<const N: usize>(
    parent: &mut [usize],
    rank: &mut [u8],
    referenced: &mut [bool],
    nodes: [usize; N],
) {
    for &node in &nodes {
        referenced[node] = true;
    }
    for &node in &nodes[1..] {
        union(parent, rank, nodes[0], node);
    }
}

fn find(parent: &mut [usize], node: usize) -> usize {
    let mut root = node;
    while parent[root] != root {
        root = parent[root];
    }
    let mut current = node;
    while parent[current] != current {
        let next = parent[current];
        parent[current] = root;
        current = next;
    }
    root
}

fn union(parent: &mut [usize], rank: &mut [u8], left: usize, right: usize) {
    let mut left_root = find(parent, left);
    let mut right_root = find(parent, right);
    if left_root == right_root {
        return;
    }
    if rank[left_root] < rank[right_root] {
        std::mem::swap(&mut left_root, &mut right_root);
    }
    parent[right_root] = left_root;
    if rank[left_root] == rank[right_root] {
        rank[left_root] += 1;
    }
}
