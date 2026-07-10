use std::{env, fs};

use kyuubiki_headless_sdk::{MaterialResearchBundle, SdkResult};

fn main() -> SdkResult<()> {
    let text = match env::args().nth(1) {
        Some(path) => fs::read_to_string(path)?,
        None => include_str!("../../../schemas/examples.material-research-bundle.json").to_string(),
    };
    let bundle: MaterialResearchBundle = serde_json::from_str(&text)?;
    bundle.validate()?;

    println!("schema={}", bundle.schema_version);
    println!("study={}", bundle.study);
    println!("winner={}", bundle.summary.winner_candidate_id);
    println!("reliability={}", bundle.summary.reliability_decision);

    Ok(())
}
