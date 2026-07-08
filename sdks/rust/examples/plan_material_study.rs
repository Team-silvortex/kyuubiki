use kyuubiki_headless_sdk::{SdkResult, material_study_execution_plan_example};

fn main() -> SdkResult<()> {
    let plan = material_study_execution_plan_example();

    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}
