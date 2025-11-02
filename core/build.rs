fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &[
                "proto/event.proto",
                "proto/agent.proto",
                "proto/plugin.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}
