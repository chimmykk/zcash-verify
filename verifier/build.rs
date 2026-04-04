fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Proto compilation is disabled; we use pre-generated gRPC stubs.
    // tonic_prost_build::compile_protos("protos/service.proto")?;
    Ok(())
}
