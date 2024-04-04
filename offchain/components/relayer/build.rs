fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile(&["proto/amplifier.proto"], &["proto", "proto/googleapis"])?;
    Ok(())
}
