/// An omitted profile preserves the complete desktop runtime contract.
pub fn required_runtime_services(profile: Option<&str>) -> Result<&'static [&'static str], String> {
    match profile {
        None | Some("desktop") => Ok(&["agent", "orchestrator", "frontend"]),
        Some("headless") => Ok(&["agent", "orchestrator"]),
        Some(value) => Err(format!("unsupported runtime service profile `{value}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::required_runtime_services;

    #[test]
    fn missing_profile_never_weakens_the_desktop_inventory() {
        assert_eq!(required_runtime_services(None).unwrap().len(), 3);
        assert_eq!(required_runtime_services(Some("desktop")).unwrap().len(), 3);
        assert_eq!(
            required_runtime_services(Some("headless")).unwrap(),
            &["agent", "orchestrator"]
        );
        for invalid in ["", "Headless", "agent", "none", "headles"] {
            assert!(required_runtime_services(Some(invalid)).is_err());
        }
    }
}
