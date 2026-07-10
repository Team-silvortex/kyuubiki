use kyuubiki_headless_sdk::{MaterialResearchBundle, SdkResult};

fn main() -> SdkResult<()> {
    let bundle: MaterialResearchBundle = serde_json::from_str(include_str!(
        "../../../schemas/examples.material-research-bundle.json"
    ))?;
    bundle.validate()?;

    println!("schema={}", bundle.schema_version);
    println!("study={}", bundle.study);
    println!("winner={}", bundle.summary.winner_candidate_id);
    println!("reliability={}", bundle.summary.reliability_decision);

    Ok(())
}
