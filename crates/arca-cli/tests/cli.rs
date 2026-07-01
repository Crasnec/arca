use std::fs;
use std::io::Write;
use std::process::{Command, Output, Stdio};

use tempfile::tempdir;

fn arca() -> Command {
    Command::new(env!("CARGO_BIN_EXE_arca"))
}

fn assert_success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "expected command to succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn output_with_stdin(mut command: Command, stdin: &[u8]) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Err(error) = child.stdin.as_mut().unwrap().write_all(stdin)
        && error.kind() != std::io::ErrorKind::BrokenPipe
    {
        panic!("failed to write command stdin: {error}");
    }
    child.wait_with_output().unwrap()
}

#[test]
fn explicit_commands_roundtrip_and_list_json() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("data.txt"), "hello cli\n").unwrap();
    let archive = dir.path().join("archive.zip");

    assert_success(
        arca()
            .args(["compress"])
            .arg(&input)
            .args(["-o"])
            .arg(&archive)
            .args(["--quiet"])
            .output()
            .unwrap(),
    );
    assert_success(
        arca()
            .args(["test"])
            .arg(&archive)
            .args(["--quiet"])
            .output()
            .unwrap(),
    );

    let list = assert_success(
        arca()
            .args(["list"])
            .arg(&archive)
            .args(["--json"])
            .output()
            .unwrap(),
    );
    let entries: serde_json::Value = serde_json::from_slice(&list.stdout).unwrap();
    let entries = entries.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["path"], "data.txt");
    assert!(entries[0].get("password").is_none());

    let output_dir = dir.path().join("out");
    assert_success(
        arca()
            .args(["extract"])
            .arg(&archive)
            .args(["-o"])
            .arg(&output_dir)
            .args(["--quiet"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        fs::read_to_string(output_dir.join("data.txt")).unwrap(),
        "hello cli\n"
    );
}

#[test]
fn password_stdin_uses_first_line_for_zip_roundtrip() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("data.txt"), "secret cli\n").unwrap();
    let archive = dir.path().join("secret.zip");
    let password_input = b"correct horse\nignored second line\n";

    assert_success(output_with_stdin(
        {
            let mut command = arca();
            command
                .args(["compress"])
                .arg(&input)
                .args(["-o"])
                .arg(&archive)
                .args(["--password-stdin", "--quiet"]);
            command
        },
        password_input,
    ));
    assert_success(output_with_stdin(
        {
            let mut command = arca();
            command
                .args(["test"])
                .arg(&archive)
                .args(["--password-stdin", "--quiet"]);
            command
        },
        password_input,
    ));

    let output_dir = dir.path().join("out");
    assert_success(output_with_stdin(
        {
            let mut command = arca();
            command
                .args(["extract"])
                .arg(&archive)
                .args(["-o"])
                .arg(&output_dir)
                .args(["--password-stdin", "--quiet"]);
            command
        },
        password_input,
    ));
    assert_eq!(
        fs::read_to_string(output_dir.join("data.txt")).unwrap(),
        "secret cli\n"
    );
}

#[test]
fn non_zip_encryption_flags_are_rejected_before_password_input() {
    for flag in ["--password", "--password-stdin", "--zipcrypto"] {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        fs::create_dir_all(&input).unwrap();
        fs::write(input.join("data.txt"), "hello cli\n").unwrap();
        let archive = dir.path().join("archive.tar");

        let output = arca()
            .args(["compress"])
            .arg(&input)
            .args(["-o"])
            .arg(&archive)
            .arg(flag)
            .args(["--quiet"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3), "flag: {flag}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("password options are only supported for .zip"),
            "stderr did not explain unsupported encryption flag {flag}:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !archive.exists(),
            "archive should not be created for {flag}"
        );
    }
}

#[test]
fn non_zip_extract_and_test_reject_password_flags_before_reading_password() {
    for command_name in ["extract", "test"] {
        for flag in ["--password", "--password-stdin"] {
            let archive = tempdir().unwrap().path().join("archive.tar");
            let output = if flag == "--password-stdin" {
                output_with_stdin(
                    {
                        let mut command = arca();
                        command
                            .arg(command_name)
                            .arg(&archive)
                            .arg(flag)
                            .args(["--quiet"]);
                        command
                    },
                    b"ignored\n",
                )
            } else {
                arca()
                    .arg(command_name)
                    .arg(&archive)
                    .arg(flag)
                    .args(["--quiet"])
                    .output()
                    .unwrap()
            };
            assert_eq!(
                output.status.code(),
                Some(3),
                "command: {command_name}, flag: {flag}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains("password options are only supported for .zip"),
                "stderr did not explain unsupported password flag {flag} for {command_name}:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

#[test]
fn zip_resource_limits_reject_list_test_and_extract_without_publishing() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("large.txt"), vec![b'x'; 128]).unwrap();
    let archive = dir.path().join("archive.zip");

    assert_success(
        arca()
            .args(["compress"])
            .arg(&input)
            .args(["-o"])
            .arg(&archive)
            .args(["--quiet"])
            .output()
            .unwrap(),
    );

    for command_name in ["list", "test"] {
        let output = arca()
            .arg(command_name)
            .arg(&archive)
            .args(["--quiet"])
            .env("ARCA_MAX_UNPACKED_BYTES", "32")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(4),
            "command: {command_name}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("limit exceeded"),
            "stderr did not explain resource limit:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output_dir = dir.path().join("out");
    let output = arca()
        .args(["extract"])
        .arg(&archive)
        .args(["-o"])
        .arg(&output_dir)
        .args(["--quiet"])
        .env("ARCA_MAX_UNPACKED_BYTES", "32")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("limit exceeded"),
        "stderr did not explain resource limit:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output_dir.exists(), "failed extraction should not publish");
}

#[test]
fn single_stream_resource_limits_reject_test_and_extract_without_publishing() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("large.txt");
    fs::write(&input, vec![b'x'; 128]).unwrap();
    let archive = dir.path().join("large.txt.gz");

    assert_success(
        arca()
            .args(["compress"])
            .arg(&input)
            .args(["-o"])
            .arg(&archive)
            .args(["--quiet"])
            .output()
            .unwrap(),
    );

    let test = arca()
        .args(["test"])
        .arg(&archive)
        .args(["--quiet"])
        .env("ARCA_MAX_UNPACKED_BYTES", "32")
        .output()
        .unwrap();
    assert_eq!(test.status.code(), Some(4));
    assert!(
        String::from_utf8_lossy(&test.stderr).contains("limit exceeded"),
        "stderr did not explain resource limit:\n{}",
        String::from_utf8_lossy(&test.stderr)
    );

    let output_file = dir.path().join("out.txt");
    let extract = arca()
        .args(["extract"])
        .arg(&archive)
        .args(["-o"])
        .arg(&output_file)
        .args(["--quiet"])
        .env("ARCA_MAX_UNPACKED_BYTES", "32")
        .output()
        .unwrap();
    assert_eq!(extract.status.code(), Some(4));
    assert!(
        String::from_utf8_lossy(&extract.stderr).contains("limit exceeded"),
        "stderr did not explain resource limit:\n{}",
        String::from_utf8_lossy(&extract.stderr)
    );
    assert!(
        !output_file.exists(),
        "failed extraction should not publish"
    );
}

#[test]
fn nested_archives_are_not_recursively_extracted() {
    let dir = tempdir().unwrap();
    let inner_input = dir.path().join("inner-input");
    fs::create_dir_all(&inner_input).unwrap();
    fs::write(inner_input.join("payload.txt"), "nested payload\n").unwrap();
    let inner_zip = dir.path().join("inner.zip");

    assert_success(
        arca()
            .args(["compress"])
            .arg(&inner_input)
            .args(["-o"])
            .arg(&inner_zip)
            .args(["--quiet"])
            .output()
            .unwrap(),
    );

    let outer_input = dir.path().join("outer-input");
    fs::create_dir_all(&outer_input).unwrap();
    fs::copy(&inner_zip, outer_input.join("inner.zip")).unwrap();
    let outer_zip = dir.path().join("outer.zip");

    assert_success(
        arca()
            .args(["compress"])
            .arg(&outer_input)
            .args(["-o"])
            .arg(&outer_zip)
            .args(["--quiet"])
            .output()
            .unwrap(),
    );

    let out = dir.path().join("out");
    assert_success(
        arca()
            .args(["extract"])
            .arg(&outer_zip)
            .args(["-o"])
            .arg(&out)
            .args(["--quiet"])
            .output()
            .unwrap(),
    );
    assert!(out.join("inner.zip").is_file());
    assert!(!out.join("payload.txt").exists());
    assert!(!out.join("inner").exists());
}

#[test]
fn rejects_short_subcommand_aliases() {
    let output = arca().arg("c").output().unwrap();
    assert!(
        !output.status.success(),
        "short subcommand alias should not be accepted"
    );
}

#[test]
fn help_is_useful_for_release_users() {
    let root = assert_success(arca().arg("--help").output().unwrap());
    let root_help = String::from_utf8_lossy(&root.stdout);
    assert!(root_help.contains("Create an archive"));
    assert!(root_help.contains("Extract an archive"));
    assert!(root_help.contains("Examples:"));

    let compress = assert_success(arca().args(["compress", "--help"]).output().unwrap());
    let compress_help = String::from_utf8_lossy(&compress.stdout);
    assert!(compress_help.contains("suffix selects the format"));
    assert!(compress_help.contains("Read the ZIP password from the first stdin line"));
    assert!(compress_help.contains("weak legacy ZipCrypto"));
    assert!(compress_help.contains("create a tar-based archive"));
    assert!(compress_help.contains("instead"));

    let extract = assert_success(arca().args(["extract", "--help"]).output().unwrap());
    let extract_help = String::from_utf8_lossy(&extract.stdout);
    assert!(extract_help.contains("Destination directory"));
    assert!(extract_help.contains("Parallel jobs for ZIP extraction"));

    let list = assert_success(arca().args(["list", "--help"]).output().unwrap());
    let list_help = String::from_utf8_lossy(&list.stdout);
    assert!(list_help.contains("machine-readable JSON"));

    let test = assert_success(arca().args(["test", "--help"]).output().unwrap());
    let test_help = String::from_utf8_lossy(&test.stdout);
    assert!(test_help.contains("Parallel jobs for ZIP integrity testing"));
}

#[test]
fn zipcrypto_requires_password_flag() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("data.txt"), "hello cli\n").unwrap();
    let archive = dir.path().join("compat.zip");

    let output = arca()
        .args(["compress"])
        .arg(&input)
        .args(["-o"])
        .arg(&archive)
        .args(["--zipcrypto", "--quiet"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--zipcrypto requires"),
        "stderr did not explain missing password:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!archive.exists());
}
