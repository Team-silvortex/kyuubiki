use crate::models::BenchmarkWorkload;

pub(crate) fn workload_shape(workload: &BenchmarkWorkload) -> (usize, usize, usize) {
    match workload {
        BenchmarkWorkload::AxialBar(request) => {
            (request.elements + 1, request.elements, request.elements)
        }
        BenchmarkWorkload::ThermalBar1d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::AcousticBar1d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::HeatBar1d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::ElectrostaticBar1d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::MagnetostaticBar1d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::Torsion1d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::Spring1d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::Spring2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::Spring3d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 3,
        ),
        BenchmarkWorkload::NonlinearSpring1d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::ContactGap1d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::Beam1d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::ThermalBeam1d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::Frame2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 3,
        ),
        BenchmarkWorkload::Frame3d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 6,
        ),
        BenchmarkWorkload::ThermalFrame2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 3,
        ),
        BenchmarkWorkload::ThermalFrame3d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 6,
        ),
        BenchmarkWorkload::ModalFrame2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 3,
        ),
        BenchmarkWorkload::ModalFrame3d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 6,
        ),
        BenchmarkWorkload::Truss2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::Truss3d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 3,
        ),
        BenchmarkWorkload::ThermalTruss2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::ThermalTruss3d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 3,
        ),
        BenchmarkWorkload::PlaneTriangle2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::PlaneQuad2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::ThermalPlaneTriangle2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::ThermalPlaneQuad2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::HeatPlaneQuad2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::HeatPlaneTriangle2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::ElectrostaticPlaneTriangle2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::ElectrostaticPlaneQuad2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::MagnetostaticPlaneTriangle2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::MagnetostaticPlaneQuad2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::StokesFlowPlaneQuad2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 3,
        ),
        BenchmarkWorkload::HeadlessActionManifest | BenchmarkWorkload::DirectFemManifest => {
            (0, 0, 0)
        }
    }
}
