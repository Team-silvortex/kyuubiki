use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct ExpandParameterSweepOperator {
    descriptor: OperatorDescriptor,
}

struct SummarizeParameterSweepOperator {
    descriptor: OperatorDescriptor,
}

struct JoinParameterSweepResultsOperator {
    descriptor: OperatorDescriptor,
}

struct ScoreParameterSweepOperator {
    descriptor: OperatorDescriptor,
}

struct MapParameterSweepScoresToQualityCandidatesOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for ExpandParameterSweepOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            crate::workflow_parameter_sweep::expand_parameter_sweep(input.payload, input.config),
        )
    }
}

impl JsonOperator for SummarizeParameterSweepOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            crate::workflow_parameter_sweep::summarize_parameter_sweep(input.payload, input.config),
        )
    }
}

impl JsonOperator for JoinParameterSweepResultsOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            crate::workflow_parameter_sweep::join_parameter_sweep_results(
                input.payload,
                input.config,
            ),
        )
    }
}

impl JsonOperator for ScoreParameterSweepOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            crate::workflow_parameter_sweep::score_parameter_sweep(input.payload, input.config),
        )
    }
}

impl JsonOperator for MapParameterSweepScoresToQualityCandidatesOperator {
    type Input = WorkflowOperatorEnvelope;

    fn descriptor(&self) -> &OperatorDescriptor {
        &self.descriptor
    }

    fn run_typed(
        &self,
        input: Self::Input,
        _context: &OperatorRunContext,
    ) -> Result<OperatorRunResult, OperatorSdkError> {
        run_summary_only(
            &self.descriptor.id,
            crate::workflow_parameter_sweep::map_parameter_sweep_scores_to_quality_candidates(
                input.payload,
                input.config,
            ),
        )
    }
}

pub(crate) fn register_parameter_sweep_operators(
    registry: &mut OperatorRegistry,
    descriptor: fn(&str) -> OperatorDescriptor,
) {
    registry
        .register_json(ExpandParameterSweepOperator {
            descriptor: descriptor("transform.expand_parameter_sweep"),
        })
        .expect("transform.expand_parameter_sweep should register");
    registry
        .register_json(SummarizeParameterSweepOperator {
            descriptor: descriptor("transform.summarize_parameter_sweep"),
        })
        .expect("transform.summarize_parameter_sweep should register");
    registry
        .register_json(JoinParameterSweepResultsOperator {
            descriptor: descriptor("transform.join_parameter_sweep_results"),
        })
        .expect("transform.join_parameter_sweep_results should register");
    registry
        .register_json(ScoreParameterSweepOperator {
            descriptor: descriptor("transform.score_parameter_sweep"),
        })
        .expect("transform.score_parameter_sweep should register");
    registry
        .register_json(MapParameterSweepScoresToQualityCandidatesOperator {
            descriptor: descriptor("transform.map_parameter_sweep_scores_to_quality_candidates"),
        })
        .expect("transform.map_parameter_sweep_scores_to_quality_candidates should register");
}
