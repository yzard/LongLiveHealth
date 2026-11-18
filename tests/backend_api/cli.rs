use clap::Parser;
use longlivehealth::cli::{Cli, Command, write_template};

#[test]
fn config_generation_never_overwrites_existing_files() {
    let directory = tempfile::tempdir().unwrap();
    write_template(directory.path()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::metadata(directory.path().join("config.toml"))
            .unwrap()
            .permissions();
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }
    assert!(write_template(directory.path()).is_err());
    assert!(longlivehealth::config::Config::load(directory.path()).is_err());
    std::fs::write(
        directory.path().join("config.toml"),
        crate::configured_template(),
    )
    .unwrap();
    assert!(longlivehealth::config::Config::load(directory.path()).is_ok());
    let parsed = Cli::try_parse_from([
        "longlivehealth",
        "--data-dir",
        directory.path().to_str().unwrap(),
        "reset-password",
        "--username",
        "alice",
        "--password-stdin",
    ])
    .unwrap();
    assert!(matches!(parsed.command, Command::ResetPassword { .. }));
    assert!(
        Cli::try_parse_from([
            "longlivehealth",
            "create-user",
            "--username",
            "alice",
            "--password",
            "secret"
        ])
        .is_err()
    );
}

#[test]
fn loinc_archive_is_read_without_opening_a_database() {
    use std::{fs::File, io::Write, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("release.zip");
    let mut archive = zip::ZipWriter::new(File::create(&source).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    archive.start_file("LoincTable/Loinc.csv", options).unwrap();
    write!(
        archive,
        "LOINC_NUM,LONG_COMMON_NAME,VersionLastChanged\r\n33914-3,Previous term,1.0ma\r\n77147-7,Replacement term,9.17\r\n"
    ).unwrap();
    archive.start_file("LoincTable/MapTo.csv", options).unwrap();
    write!(archive, "LOINC,MAP_TO,COMMENT\r\n33914-3,77147-7,\"Replacement, with context\r\non another line\"\r\n").unwrap();
    archive.finish().unwrap();

    let check = |path: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_longlivehealth"))
            .args([
                "--data-dir",
                directory.path().to_str().unwrap(),
                "check-loinc",
            ])
            .env("LONGLIVEHEALTH_LOINC_ARCHIVE", path)
            .output()
            .unwrap()
    };
    let output = check(&source);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "LOINC 9.17: 2 terms, 1 MapTo rows"
    );
    assert!(!directory.path().join("database.sqlite").exists());
    let missing = check(&directory.path().join("missing.zip"));
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr)
            .contains("Cannot open the official LOINC release ZIP")
    );
}
