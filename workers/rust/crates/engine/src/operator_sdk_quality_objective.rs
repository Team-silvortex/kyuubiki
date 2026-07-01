use crate::operator_sdk_runtime::{WorkflowOperatorEnvelope, run_summary_only};
use crate::workflow_quality_objective::{
    build_quality_parameter_sweep_plan, compose_quality_objective,
    prepare_quality_next_round_request, rank_quality_candidates,
};
use crate::workflow_quality_sweep_plan::materialize_quality_sweep_expansion;
use kyuubiki_operator_sdk::{JsonOperator, OperatorRegistry, OperatorSdkError};
use kyuubiki_protocol::{OperatorDescriptor, OperatorRunContext, OperatorRunResult};

struct ComposeQualityObjectiveOperator {
    descriptor: OperatorDescriptor,
}

struct RankQualityCandidatesOperator {
    descriptor: OperatorDescriptor,
}

struct PrepareQualityNextRoundRequestOperator {
    descriptor: OperatorDescriptor,
}

struct BuildQualityParameterSweepPlanOperator {
    descriptor: OperatorDescriptor,
}

struct MaterializeQualitySweepExpansionOperator {
    descriptor: OperatorDescriptor,
}

impl JsonOperator for ComposeQualityObjectiveOperator {
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
            compose_quality_objective(input.payload, input.config),
        )
    }
}

impl JsonOperator for RankQualityCandidatesOperator {
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
            rank_quality_candidates(input.payload, input.config),
        )
    }
}

impl JsonOperator for PrepareQualityNextRoundRequestOperator {
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
            prepare_quality_next_round_request(input.payload, input.config),
        )
    }
}

impl JsonOperator for BuildQualityParameterSweepPlanOperator {
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
            build_quality_parameter_sweep_plan(input.payload, input.config),
        )
    }
}

impl JsonOperator for MaterializeQualitySweepExpansionOperator {
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
            materialize_quality_sweep_expansion(input.payload, input.config),
        )
    }
}

pub(crate) fn register_quality_objective_operators(
    registry: &mut OperatorRegistry,
    descriptor: fn(&str) -> OperatorDescriptor,
) {
    registry
        .register_json(ComposeQualityObjectiveOperator {
            descriptor: descriptor("transform.compose_quality_objective"),
        })
        .expect("transform.compose_quality_objective should register");
    registry
        .register_json(RankQualityCandidatesOperator {
            descriptor: descriptor("transform.rank_quality_candidates"),
        })
        .expect("transform.rank_quality_candidates should register");
    registry
        .register_json(PrepareQualityNextRoundRequestOperator {
            descriptor: descriptor("transform.prepare_quality_next_round_request"),
        })
        .expect("transform.prepare_quality_next_round_request should register");
    registry
        .register_json(BuildQualityParameterSweepPlanOperator {
            descriptor: descriptor("transform.build_quality_parameter_sweep_plan"),
        })
        .expect("transform.build_quality_parameter_sweep_plan should register");
    registry
        .register_json(MaterializeQualitySweepExpansionOperator {
            descriptor: descriptor("transform.materialize_quality_sweep_expansion"),
        })
        .expect("transform.materialize_quality_sweep_expansion should register");
}
