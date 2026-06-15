use kyuubiki_operator_template::run_template_operator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = run_template_operator(vec![12.0, 18.0, 27.5])?;
    println!("{}", serde_json::to_string_pretty(&result.summary)?);
    Ok(())
}
