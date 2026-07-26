use kyuubiki_protocol::CohesiveTractionRegime;

const HISTORY_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CohesiveLaw {
    stiffness: f64,
    peak_traction: f64,
    failure_separation: f64,
    onset_separation: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CohesiveHistory {
    pub max_separation: f64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CohesiveResponse {
    pub traction: f64,
    pub tangent: f64,
    pub damage: f64,
    pub max_separation: f64,
    pub regime: CohesiveTractionRegime,
}

impl CohesiveLaw {
    pub fn new(
        stiffness: f64,
        peak_traction: f64,
        failure_separation: f64,
        label: &str,
    ) -> Result<Self, String> {
        for (name, value) in [
            ("initial_stiffness", stiffness),
            ("peak_traction", peak_traction),
            ("failure_separation", failure_separation),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("{label} {name} must be finite and positive"));
            }
        }
        let onset_separation = peak_traction / stiffness;
        if !onset_separation.is_finite() || onset_separation <= 0.0 {
            return Err(format!(
                "{label} peak_traction / initial_stiffness must be finite and positive"
            ));
        }
        if failure_separation <= onset_separation {
            return Err(format!(
                "{label} failure_separation must exceed peak_traction / initial_stiffness"
            ));
        }
        Ok(Self {
            stiffness,
            peak_traction,
            failure_separation,
            onset_separation,
        })
    }

    pub fn onset_separation(self) -> f64 {
        self.onset_separation
    }

    pub fn fracture_energy(self) -> f64 {
        0.5 * self.peak_traction * self.failure_separation
    }

    pub fn evaluate(
        self,
        history: &mut CohesiveHistory,
        separation: f64,
        compression_stiffness: Option<f64>,
    ) -> CohesiveResponse {
        if let Some(stiffness) = compression_stiffness
            && separation < 0.0
        {
            return CohesiveResponse {
                traction: stiffness * separation,
                tangent: stiffness,
                damage: self.damage(history.max_separation),
                max_separation: history.max_separation,
                regime: CohesiveTractionRegime::Compression,
            };
        }

        let equivalent_separation = if compression_stiffness.is_some() {
            separation.max(0.0)
        } else {
            separation.abs()
        };
        let previous_max = history.max_separation;
        history.max_separation = previous_max.max(equivalent_separation);
        let damage = self.damage(history.max_separation);
        let historical_unloading = equivalent_separation + HISTORY_TOLERANCE < previous_max;

        let (traction, tangent, regime) = if damage >= 1.0 {
            (0.0, 0.0, CohesiveTractionRegime::Failed)
        } else if historical_unloading {
            (
                (1.0 - damage) * self.stiffness * separation,
                (1.0 - damage) * self.stiffness,
                CohesiveTractionRegime::UnloadingReloading,
            )
        } else if history.max_separation <= self.onset_separation {
            (
                self.stiffness * separation,
                self.stiffness,
                if compression_stiffness.is_some() {
                    CohesiveTractionRegime::ElasticOpening
                } else {
                    CohesiveTractionRegime::Elastic
                },
            )
        } else {
            (
                (1.0 - damage) * self.stiffness * separation,
                -self.peak_traction / (self.failure_separation - self.onset_separation),
                CohesiveTractionRegime::Softening,
            )
        };

        CohesiveResponse {
            traction,
            tangent,
            damage,
            max_separation: history.max_separation,
            regime,
        }
    }

    fn damage(self, max_separation: f64) -> f64 {
        if max_separation <= self.onset_separation {
            0.0
        } else if max_separation >= self.failure_separation {
            1.0
        } else {
            self.failure_separation * (max_separation - self.onset_separation)
                / (max_separation * (self.failure_separation - self.onset_separation))
        }
    }
}
