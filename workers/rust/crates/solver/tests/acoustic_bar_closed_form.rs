use kyuubiki_protocol::{
    AcousticBar1dElementInput, AcousticBar1dNodeInput, SolveAcousticBar1dRequest,
};
use kyuubiki_solver::solve_acoustic_bar_1d;

const TOL: f64 = 1.0e-9;

#[test]
fn acoustic_bar_1d_matches_closed_form_response_across_frequencies() {
    for frequency_hz in [50.0, 200.0] {
        let expected = expected_closed_form(frequency_hz);
        let result =
            solve_acoustic_bar_1d(&request(frequency_hz)).expect("closed-form acoustic case");

        assert_close(result.frequency_hz, frequency_hz);
        assert_close(result.angular_frequency, expected.omega);
        assert_close(result.nodes[0].pressure, expected.fixed_pressure);
        assert_close(result.nodes[1].pressure, expected.source_pressure);
        assert_close(result.max_pressure, expected.max_pressure);
        assert_close(result.max_particle_velocity, expected.particle_velocity);
        assert_close(result.max_acoustic_intensity, expected.acoustic_intensity);
        assert_close(result.total_damping_loss, expected.damping_loss);

        let element = &result.elements[0];
        assert_close(element.speed_of_sound, expected.speed_of_sound);
        assert_close(element.wave_number, expected.wave_number);
        assert_close(element.pressure_gradient, expected.pressure_gradient);
        assert_close(element.particle_velocity, expected.particle_velocity);
        assert_close(element.acoustic_intensity, expected.acoustic_intensity);
        assert_close(element.damping_loss, expected.damping_loss);
    }
}

#[test]
fn acoustic_bar_1d_reports_zero_loss_for_undamped_closed_form_case() {
    let mut request = request(125.0);
    request.elements[0].damping_ratio = 0.0;
    let result = solve_acoustic_bar_1d(&request).expect("undamped acoustic case");

    assert_close(result.elements[0].damping_loss, 0.0);
    assert_close(result.total_damping_loss, 0.0);
}

#[test]
fn acoustic_bar_1d_tracks_material_wave_speed_scaling() {
    let baseline_request = request(180.0);
    let baseline_expected = expected_from_request(&baseline_request);
    let baseline =
        solve_acoustic_bar_1d(&baseline_request).expect("baseline acoustic material case");

    let bulk_scale: f64 = 1.21;
    let mut stiffer_request = baseline_request.clone();
    stiffer_request.elements[0].bulk_modulus *= bulk_scale;
    let stiffer_expected = expected_from_request(&stiffer_request);
    let stiffer = solve_acoustic_bar_1d(&stiffer_request).expect("stiffer acoustic material case");

    assert_close(
        stiffer.elements[0].speed_of_sound / baseline.elements[0].speed_of_sound,
        bulk_scale.sqrt(),
    );
    assert_close(
        stiffer.elements[0].wave_number / baseline.elements[0].wave_number,
        1.0 / bulk_scale.sqrt(),
    );
    assert_close(stiffer.nodes[1].pressure, stiffer_expected.source_pressure);
    assert_close(
        stiffer.elements[0].particle_velocity,
        stiffer_expected.particle_velocity,
    );
    assert_close(stiffer.total_damping_loss, stiffer_expected.damping_loss);

    let density_scale: f64 = 1.44;
    let mut denser_request = baseline_request.clone();
    denser_request.elements[0].density *= density_scale;
    let denser_expected = expected_from_request(&denser_request);
    let denser = solve_acoustic_bar_1d(&denser_request).expect("denser acoustic material case");

    assert_close(
        denser.elements[0].speed_of_sound / baseline.elements[0].speed_of_sound,
        1.0 / density_scale.sqrt(),
    );
    assert_close(
        denser.elements[0].wave_number / baseline.elements[0].wave_number,
        density_scale.sqrt(),
    );
    assert_close(denser.nodes[1].pressure, denser_expected.source_pressure);
    assert_close(
        denser.elements[0].particle_velocity,
        denser_expected.particle_velocity,
    );
    assert_close(denser.total_damping_loss, denser_expected.damping_loss);
    assert_close(
        baseline.nodes[1].pressure,
        baseline_expected.source_pressure,
    );

    let length_scale = 1.35;
    let mut longer_request = baseline_request.clone();
    longer_request.nodes[1].x *= length_scale;
    let longer_expected = expected_from_request(&longer_request);
    let longer = solve_acoustic_bar_1d(&longer_request).expect("length-scaled acoustic duct");

    assert_close(
        longer.elements[0].speed_of_sound,
        baseline.elements[0].speed_of_sound,
    );
    assert_close(
        longer.elements[0].wave_number,
        baseline.elements[0].wave_number,
    );
    assert_close(longer.nodes[1].pressure, longer_expected.source_pressure);
    assert_close(
        longer.elements[0].pressure_gradient,
        longer_expected.pressure_gradient,
    );
    assert_close(
        longer.elements[0].particle_velocity,
        longer_expected.particle_velocity,
    );
    assert_close(longer.total_damping_loss, longer_expected.damping_loss);
}

#[test]
fn acoustic_bar_1d_tracks_source_amplitude_scaling() {
    let baseline_request = source_scaling_request(160.0);
    let baseline = solve_acoustic_bar_1d(&baseline_request).expect("baseline acoustic source case");

    let source_scale = 2.5;
    let mut louder_request = baseline_request.clone();
    louder_request.nodes[1].volume_velocity_source *= source_scale;
    let louder_expected = expected_from_request(&louder_request);
    let louder = solve_acoustic_bar_1d(&louder_request).expect("source-scaled acoustic case");

    assert_close(louder.nodes[1].pressure, louder_expected.source_pressure);
    assert_close(
        louder.nodes[1].pressure / baseline.nodes[1].pressure,
        source_scale,
    );
    assert_close(
        louder.elements[0].pressure_gradient / baseline.elements[0].pressure_gradient,
        source_scale,
    );
    assert_close(
        louder.max_particle_velocity / baseline.max_particle_velocity,
        source_scale,
    );
    assert_close(
        louder.max_acoustic_intensity / baseline.max_acoustic_intensity,
        source_scale * source_scale,
    );
    assert_close(
        louder.total_damping_loss / baseline.total_damping_loss,
        source_scale * source_scale,
    );
    assert_close(
        louder.nodes[1].sound_pressure_level_db,
        20.0 * (louder.nodes[1].pressure.abs() / 20.0e-6).log10(),
    );
}

fn source_scaling_request(frequency_hz: f64) -> SolveAcousticBar1dRequest {
    let mut request = request(frequency_hz);
    request.nodes[0].pressure = 0.0;
    request
}

fn request(frequency_hz: f64) -> SolveAcousticBar1dRequest {
    SolveAcousticBar1dRequest {
        frequency_hz,
        nodes: vec![
            node("fixed", 0.0, true, 1.5, 0.0),
            node("source", 1.25, false, 0.0, 0.008),
        ],
        elements: vec![AcousticBar1dElementInput {
            id: "duct".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.08,
            density: 1.18,
            bulk_modulus: 141_000.0,
            damping_ratio: 0.015,
        }],
    }
}

fn node(
    id: &str,
    x: f64,
    fix_pressure: bool,
    pressure: f64,
    volume_velocity_source: f64,
) -> AcousticBar1dNodeInput {
    AcousticBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_pressure,
        pressure,
        volume_velocity_source,
    }
}

fn expected_closed_form(frequency_hz: f64) -> ExpectedAcousticResponse {
    let request = request(frequency_hz);
    expected_from_request(&request)
}

fn expected_from_request(request: &SolveAcousticBar1dRequest) -> ExpectedAcousticResponse {
    let element = &request.elements[0];
    let fixed_pressure = request.nodes[0].pressure;
    let source = request.nodes[1].volume_velocity_source;
    let length = request.nodes[element.node_j].x - request.nodes[element.node_i].x;
    let omega = 2.0 * std::f64::consts::PI * request.frequency_hz;
    let stiffness = element.area / (element.density * length);
    let mass = element.area * length / (6.0 * element.bulk_modulus);
    let dynamic = omega * omega * mass * (1.0 + element.damping_ratio);
    let k10 = -stiffness + dynamic;
    let k11 = stiffness + 2.0 * dynamic;
    let source_pressure = (source * omega - k10 * fixed_pressure) / k11;
    let speed_of_sound = f64::sqrt(element.bulk_modulus / element.density);
    let wave_number = omega / speed_of_sound;
    let pressure_gradient = (source_pressure - fixed_pressure) / length;
    let particle_velocity = pressure_gradient.abs() / (element.density * omega);
    let average_pressure = (fixed_pressure.abs() + source_pressure.abs()) / 2.0;
    let acoustic_intensity = average_pressure * particle_velocity / 2.0;
    let damping_loss = acoustic_intensity * element.damping_ratio * element.area * length;
    ExpectedAcousticResponse {
        omega,
        fixed_pressure,
        source_pressure,
        max_pressure: fixed_pressure.abs().max(source_pressure.abs()),
        speed_of_sound,
        wave_number,
        pressure_gradient,
        particle_velocity,
        acoustic_intensity,
        damping_loss,
    }
}

struct ExpectedAcousticResponse {
    omega: f64,
    fixed_pressure: f64,
    source_pressure: f64,
    max_pressure: f64,
    speed_of_sound: f64,
    wave_number: f64,
    pressure_gradient: f64,
    particle_velocity: f64,
    acoustic_intensity: f64,
    damping_loss: f64,
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
