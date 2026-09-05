use kyuubiki_protocol::SolveElectricConductionPlaneQuad2dRequest;

pub(crate) fn validate_anchored_components(
    request: &SolveElectricConductionPlaneQuad2dRequest,
) -> Result<(), String> {
    let mut parent = (0..request.nodes.len()).collect::<Vec<_>>();
    let mut rank = vec![0_u8; request.nodes.len()];
    for element in &request.elements {
        for node in [element.node_j, element.node_k, element.node_l] {
            union(&mut parent, &mut rank, element.node_i, node);
        }
    }
    for contact in &request.contact_interfaces {
        union(&mut parent, &mut rank, contact.node_i, contact.node_j);
    }

    let mut anchored = vec![false; request.nodes.len()];
    for (index, node) in request.nodes.iter().enumerate() {
        if node.fix_electric_potential {
            let root = find(&mut parent, index);
            anchored[root] = true;
        }
    }
    for terminal in &request.terminals {
        let root = find(&mut parent, terminal.node);
        anchored[root] = true;
    }

    for (index, node) in request.nodes.iter().enumerate() {
        let root = find(&mut parent, index);
        if !anchored[root] {
            return Err(format!(
                "electric conduction topology component containing node {index} ({}) is not anchored by a fixed potential or impedance terminal",
                node.id,
            ));
        }
    }
    Ok(())
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
