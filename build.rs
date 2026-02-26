fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::compile_protos("proto/helloworld.proto")?;
    tonic_prost_build::compile_protos("proto/robots.proto")?;
    Ok(())
}
