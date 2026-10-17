use crate::host::PluginKey;
pub fn default_instrument() -> Result<PluginKey, String> {
    let common = std::env::var_os("CommonProgramFiles").ok_or("CommonProgramFiles unavailable")?;
    let path = std::path::PathBuf::from(common).join("CLAP/Surge Synth Team/Surge XT.clap");
    if !path.is_file() {
        return Err(format!("Default instrument missing: {}", path.display()));
    }
    Ok(PluginKey {
        format: "CLAP".into(),
        id: "org.surge-synth-team.surge-xt".into(),
        name: "Surge XT".into(),
        vendor: "Surge Synth Team".into(),
        bundle_path: path.to_string_lossy().into_owned(),
    })
}
