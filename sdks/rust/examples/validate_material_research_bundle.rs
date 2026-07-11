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
    println!("next_round={}", bundle.summary.next_round_decision);
    println!(
        "next_iteration={}",
        bundle.summary.next_iteration.unwrap_or_default()
    );
    println!(
        "runnable_next_steps={}",
        bundle.summary.runnable_next_step_count.unwrap_or_default()
    );
    println!("chain_stop={}", bundle.summary.chain_stop_reason);

    Ok(())
}
