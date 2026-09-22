fn main() -> Result<(), CappError> {
    vmix_plugin::force_link();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pi/src/generated");
    std::fs::create_dir_all(&root)?;
    let dest = root.join("contracts.ts");
    streamdeck_plugin::export_typescript(&dest)?;
    let source = std::fs::read_to_string(&dest)?;
    let exported = source.replace("\ntype ", "\nexport type ");
    std::fs::write(&dest, exported)?;
    std::fs::write(root.join("shortcuts.json"), vmix_rs::shortcuts::JSON)?;
    println!("wrote {}", dest.display());
    Ok(())
}

#[derive(Debug)]
struct CappError(String);

impl std::fmt::Display for CappError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for CappError {}

impl From<streamdeck_plugin::Error> for CappError {
    fn from(error: streamdeck_plugin::Error) -> Self {
        Self(error.to_string())
    }
}

impl From<std::io::Error> for CappError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}
