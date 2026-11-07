use heroinn_util::{
    ConnectionInfo, HeroinnProtocol, Socks5ProxyConfig,
    dna::{
        CLIENT_CAPABILITY_MARKER, connection_stamp_offsets, replace_connection_info_to_new_file,
    },
};
use std::{env, fs, path::PathBuf};

#[test]
#[ignore = "requires HEROINN_CLIENT_TEMPLATE to point at a built heroinn_client"]
fn patches_a_real_client_template() {
    let configured_template = PathBuf::from(
        env::var_os("HEROINN_CLIENT_TEMPLATE")
            .expect("HEROINN_CLIENT_TEMPLATE must point at a built heroinn_client"),
    );
    let template = if configured_template.is_absolute() {
        configured_template
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("heroinn_util must be inside the workspace")
            .join(configured_template)
    };
    let output_directory = env::temp_dir().join(format!(
        "heroinn-real-template-test-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&output_directory).unwrap();
    let output = output_directory.join(if cfg!(windows) {
        "heroinn_client_generated.exe"
    } else {
        "heroinn_client_generated"
    });

    let template_binary = fs::read(&template).unwrap();
    assert_eq!(
        template_binary
            .windows(CLIENT_CAPABILITY_MARKER.len())
            .filter(|window| *window == CLIENT_CAPABILITY_MARKER)
            .count(),
        1,
        "client capability marker must be present exactly once"
    );
    assert_eq!(
        connection_stamp_offsets(&template_binary).len(),
        1,
        "the structurally valid connection stamp must remain uniquely locatable"
    );

    let info = ConnectionInfo::new_with_socks5(
        HeroinnProtocol::TCP,
        "example.com",
        "9001",
        "integration",
        Some(Socks5ProxyConfig::new("proxy.example", 1080).unwrap()),
    )
    .unwrap();
    let input_size = fs::metadata(&template).unwrap().len();
    replace_connection_info_to_new_file(&template, &output, info.clone()).unwrap();

    let binary = fs::read(&output).unwrap();
    let payload = info.serialize().unwrap();
    assert_eq!(binary.len() as u64, input_size);
    assert!(
        binary
            .windows(payload.len())
            .any(|window| window == payload)
    );
    assert!(heroinn_util::dna::client_supports_feature(
        &binary,
        heroinn_util::dna::CLIENT_FEATURE_SOCKS5_PROXY
    ));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let input_mode = fs::metadata(&template).unwrap().permissions().mode() & 0o777;
        let output_mode = fs::metadata(&output).unwrap().permissions().mode() & 0o777;
        assert_eq!(output_mode, input_mode);
    }

    fs::remove_dir_all(output_directory).unwrap();
}
