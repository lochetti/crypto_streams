fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(false)
        .compile(&["proto/messages.proto"], &["proto/"])
        .map_err(Into::into)
}
