fn main() {
    // Only the release workflow is generated here. The Dockerfile is hand-written and committed:
    // the image has to carry KumoMTA next to the service binary, and the Dockerfile
    // `CiGenerator::as_basic_service()` produces is a bare `FROM ubuntu` + `COPY` with no hook
    // to add it. Leaving `as_basic_service()` out makes `build()` skip the Dockerfile generation
    // and keep the committed one intact.
    ci_utils::ci_generator::CiGenerator::new(env!("CARGO_PKG_NAME"))
        .generate_github_ci_file()
        .with_ci_test()
        .build();

    // The proto file lives in this repository - there is nothing to sync from elsewhere.
    ci_utils::compile_protos("proto/MySmtpSender.proto");
}
