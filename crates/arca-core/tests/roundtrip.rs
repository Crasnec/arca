use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use arca_core::{
    CompressOptions, Encryption, ExitCode, ExtractOptions, Password, TestOptions, compress,
    extract, list, test,
};
use filetime::{FileTime, set_file_times};
use flate2::{Compression, write::GzEncoder};
use tar::{Builder as TarBuilder, EntryType, Header};
use tempfile::tempdir;
use zip::write::FileOptions;

fn base_compress(input: PathBuf, output: PathBuf) -> CompressOptions {
    CompressOptions {
        inputs: vec![input],
        output: Some(output),
        overwrite: false,
        level: None,
        jobs: 1,
        excludes: Vec::new(),
        encryption: Encryption::None,
        auto_tar: false,
    }
}

fn write_numbered_input(root: &Path, count: usize) -> PathBuf {
    let input = root.join("input");
    fs::create_dir_all(input.join("sub")).unwrap();
    for index in 0..count {
        let parent = if index % 2 == 0 {
            input.clone()
        } else {
            input.join("sub")
        };
        fs::write(
            parent.join(format!("file-{index}.txt")),
            format!("payload {index}\n"),
        )
        .unwrap();
    }
    input
}

fn assert_numbered_output(output: &Path, count: usize) {
    for index in 0..count {
        let parent = if index % 2 == 0 {
            output.to_path_buf()
        } else {
            output.join("sub")
        };
        assert_eq!(
            fs::read_to_string(parent.join(format!("file-{index}.txt"))).unwrap(),
            format!("payload {index}\n")
        );
    }
}

#[test]
fn zip_roundtrip_single_directory_contents() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(input.join("sub")).unwrap();
    fs::write(input.join("README.txt"), "hello arca\n").unwrap();
    fs::write(input.join("sub/data.txt"), "nested\n").unwrap();

    let archive = dir.path().join("input.zip");
    compress(base_compress(input.clone(), archive.clone())).unwrap();
    test(TestOptions {
        archive: archive.clone(),
        jobs: 1,
        password: None,
    })
    .unwrap();
    let out = dir.path().join("out");
    extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    })
    .unwrap();

    assert_eq!(
        fs::read_to_string(out.join("README.txt")).unwrap(),
        "hello arca\n"
    );
    assert_eq!(
        fs::read_to_string(out.join("sub/data.txt")).unwrap(),
        "nested\n"
    );
}

#[test]
fn empty_container_archives_extract_to_empty_destination() {
    for suffix in ["zip", "tar", "tar.gz"] {
        let dir = tempdir().unwrap();
        let archive = dir.path().join(format!("empty.{suffix}"));
        write_empty_archive(&archive, suffix);

        assert!(list(archive.clone()).unwrap().is_empty());
        test(TestOptions {
            archive: archive.clone(),
            jobs: 1,
            password: None,
        })
        .unwrap();

        let out = dir.path().join("out");
        extract(ExtractOptions {
            archive,
            output: Some(out.clone()),
            overwrite: false,
            jobs: 1,
            password: None,
        })
        .unwrap();

        assert!(
            out.is_dir(),
            "empty archive should publish output directory"
        );
        assert_eq!(fs::read_dir(&out).unwrap().count(), 0);
    }
}

#[test]
fn container_extract_overwrite_replaces_existing_files() {
    for suffix in ["zip", "tar.gz"] {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        fs::create_dir_all(input.join("sub")).unwrap();
        fs::write(input.join("README.txt"), format!("new {suffix}\n")).unwrap();
        fs::write(input.join("sub/data.txt"), "nested\n").unwrap();

        let archive = dir.path().join(format!("input.{suffix}"));
        compress(base_compress(input, archive.clone())).unwrap();

        let out = dir.path().join("out");
        fs::create_dir_all(out.join("sub")).unwrap();
        fs::write(out.join("README.txt"), "old\n").unwrap();
        fs::write(out.join("sub/data.txt"), "old nested\n").unwrap();

        let err = extract(ExtractOptions {
            archive: archive.clone(),
            output: Some(out.clone()),
            overwrite: false,
            jobs: 1,
            password: None,
        })
        .unwrap_err();
        assert!(
            matches!(err, arca_core::ArcaError::Usage(_)),
            "expected usage error, got {err}"
        );
        assert_eq!(fs::read_to_string(out.join("README.txt")).unwrap(), "old\n");

        extract(ExtractOptions {
            archive,
            output: Some(out.clone()),
            overwrite: true,
            jobs: 1,
            password: None,
        })
        .unwrap();

        assert_eq!(
            fs::read_to_string(out.join("README.txt")).unwrap(),
            format!("new {suffix}\n")
        );
        assert_eq!(
            fs::read_to_string(out.join("sub/data.txt")).unwrap(),
            "nested\n"
        );
    }
}

#[test]
fn container_extract_preflight_preserves_existing_files_when_publish_target_invalid() {
    for suffix in ["zip", "tar.gz"] {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        fs::create_dir_all(&input).unwrap();
        fs::write(input.join("a.txt"), format!("new {suffix}\n")).unwrap();
        fs::write(input.join("z.txt"), "new blocked\n").unwrap();

        let archive = dir.path().join(format!("input.{suffix}"));
        compress(base_compress(input, archive.clone())).unwrap();

        let out = dir.path().join("out");
        fs::create_dir_all(out.join("z.txt")).unwrap();
        fs::write(out.join("a.txt"), "old\n").unwrap();

        assert_security_error(extract(ExtractOptions {
            archive,
            output: Some(out.clone()),
            overwrite: true,
            jobs: 1,
            password: None,
        }));
        assert_eq!(fs::read_to_string(out.join("a.txt")).unwrap(), "old\n");
        assert!(out.join("z.txt").is_dir());
    }
}

#[test]
fn single_stream_extract_overwrite_replaces_existing_file() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("data.txt");
    fs::write(&input, "new\n").unwrap();
    let archive = dir.path().join("data.txt.gz");
    compress(base_compress(input, archive.clone())).unwrap();

    let out = dir.path().join("data.out");
    fs::write(&out, "old\n").unwrap();
    let err = extract(ExtractOptions {
        archive: archive.clone(),
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    })
    .unwrap_err();
    assert!(
        matches!(err, arca_core::ArcaError::Usage(_)),
        "expected usage error, got {err}"
    );
    assert_eq!(fs::read_to_string(&out).unwrap(), "old\n");

    extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: true,
        jobs: 1,
        password: None,
    })
    .unwrap();
    assert_eq!(fs::read_to_string(&out).unwrap(), "new\n");
}

#[test]
fn single_stream_extract_without_output_writes_next_to_archive() {
    let dir = tempdir().unwrap();
    let name = format!(
        "single-stream-default-{}.txt",
        dir.path().file_name().unwrap().to_string_lossy()
    );
    let input = dir.path().join(&name);
    fs::write(&input, "default output\n").unwrap();
    let archive = dir.path().join(format!("{name}.gz"));
    compress(base_compress(input.clone(), archive.clone())).unwrap();
    fs::remove_file(&input).unwrap();

    let entries = list(archive.clone()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, name);
    assert_eq!(
        entries[0].uncompressed_size,
        "default output\n".len() as u64
    );
    assert_eq!(
        entries[0].compressed_size,
        Some(fs::metadata(&archive).unwrap().len())
    );

    let cwd_artifact = PathBuf::from(&entries[0].path);
    if cwd_artifact.exists() {
        fs::remove_file(&cwd_artifact).unwrap();
    }
    let extracted = extract(ExtractOptions {
        archive,
        output: None,
        overwrite: false,
        jobs: 1,
        password: None,
    })
    .unwrap();
    let leaked_to_cwd = cwd_artifact.exists();
    if leaked_to_cwd {
        fs::remove_file(&cwd_artifact).unwrap();
    }

    assert_eq!(extracted, input);
    assert!(!leaked_to_cwd, "default extract wrote into current dir");
    assert_eq!(fs::read_to_string(&input).unwrap(), "default output\n");
}

#[test]
fn single_stream_compress_rejects_output_equal_input_even_with_overwrite() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("data.gz");
    fs::write(&input, "original\n").unwrap();

    let mut options = base_compress(input.clone(), input.clone());
    options.overwrite = true;
    let err = compress(options).unwrap_err();
    assert!(
        matches!(err, arca_core::ArcaError::Usage(_)),
        "expected usage error, got {err}"
    );
    assert_eq!(fs::read_to_string(&input).unwrap(), "original\n");
}

#[test]
fn single_stream_extract_rejects_output_equal_archive_even_with_overwrite() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("data.txt");
    fs::write(&input, "original\n").unwrap();
    let archive = dir.path().join("data.txt.gz");
    compress(base_compress(input, archive.clone())).unwrap();

    let err = extract(ExtractOptions {
        archive: archive.clone(),
        output: Some(archive.clone()),
        overwrite: true,
        jobs: 1,
        password: None,
    })
    .unwrap_err();
    assert!(
        matches!(err, arca_core::ArcaError::Usage(_)),
        "expected usage error, got {err}"
    );
    test(TestOptions {
        archive,
        jobs: 1,
        password: None,
    })
    .unwrap();
}

#[cfg(unix)]
#[test]
fn publish_refuses_to_overwrite_existing_symlinks() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let input = dir.path().join("data.txt");
    fs::write(&input, "new\n").unwrap();

    let archive = dir.path().join("data.zip");
    let mut options = base_compress(input.clone(), archive.clone());
    compress(options.clone()).unwrap();

    let link_target = dir.path().join("target.txt");
    fs::write(&link_target, "target\n").unwrap();
    let output_link = dir.path().join("out.zip");
    symlink(&link_target, &output_link).unwrap();
    options.output = Some(output_link);
    options.overwrite = true;
    assert_security_error(compress(options));

    let out_dir = dir.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();
    symlink(&link_target, out_dir.join("data.txt")).unwrap();
    assert_security_error(extract(ExtractOptions {
        archive: archive.clone(),
        output: Some(out_dir),
        overwrite: true,
        jobs: 1,
        password: None,
    }));

    let outside_dir = dir.path().join("outside");
    fs::create_dir_all(&outside_dir).unwrap();
    let destination_link = dir.path().join("destination-link");
    symlink(&outside_dir, &destination_link).unwrap();
    assert_security_error(extract(ExtractOptions {
        archive,
        output: Some(destination_link),
        overwrite: true,
        jobs: 1,
        password: None,
    }));
    assert!(!outside_dir.join("data.txt").exists());
}

#[test]
fn aes_zip_roundtrip() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("secret.txt"), "classified\n").unwrap();

    let archive = dir.path().join("secret.zip");
    let mut options = base_compress(input, archive.clone());
    options.encryption = Encryption::Aes256(Password::new(b"secret".to_vec()));
    compress(options).unwrap();

    let out = dir.path().join("out");
    extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: Some(Password::new(b"secret".to_vec())),
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(out.join("secret.txt")).unwrap(),
        "classified\n"
    );
}

#[test]
fn password_debug_output_is_redacted() {
    let password = Password::new(b"secret".to_vec());
    let rendered = format!("{password:?}");
    assert_eq!(rendered, "Password(<redacted>)");

    let rendered = format!("{:?}", Encryption::Aes256(password));
    assert!(!rendered.contains("secret"));
}

#[test]
fn aes_zip_rejects_wrong_password() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("secret.txt"), "classified\n").unwrap();

    let archive = dir.path().join("secret.zip");
    let mut options = base_compress(input, archive.clone());
    options.encryption = Encryption::Aes256(Password::new(b"secret".to_vec()));
    compress(options).unwrap();

    assert_password_error(test(TestOptions {
        archive: archive.clone(),
        jobs: 1,
        password: Some(Password::new(b"wrong".to_vec())),
    }));
    let out = dir.path().join("out");
    assert_password_error(extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: Some(Password::new(b"wrong".to_vec())),
    }));
    assert!(!out.exists(), "failed extraction should not publish output");
}

#[cfg(not(windows))]
#[test]
fn creation_rejects_windows_reserved_names() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("CON.txt"), "bad\n").unwrap();

    let archive = dir.path().join("bad.zip");
    let err = compress(base_compress(input, archive)).unwrap_err();
    assert!(err.to_string().contains("Windows reserved"));
}

#[cfg(unix)]
#[test]
fn zip_restores_mtime_and_unix_mode() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    let script = input.join("run.sh");
    fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let mtime = FileTime::from_unix_time(1_700_000_000, 0);
    set_file_times(&script, mtime, mtime).unwrap();

    let archive = dir.path().join("input.zip");
    compress(base_compress(input, archive.clone())).unwrap();
    let out = dir.path().join("out");
    extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    })
    .unwrap();

    let extracted = out.join("run.sh");
    assert_eq!(
        fs::metadata(&extracted).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(
        FileTime::from_last_modification_time(&fs::metadata(&extracted).unwrap()).unix_seconds(),
        1_700_000_000
    );
}

#[cfg(unix)]
#[test]
fn tar_restores_mtime_and_unix_mode() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    let script = input.join("tool");
    fs::write(&script, "payload\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let mtime = FileTime::from_unix_time(1_700_000_002, 0);
    set_file_times(&script, mtime, mtime).unwrap();

    let archive = dir.path().join("input.tar");
    compress(base_compress(input, archive.clone())).unwrap();
    let out = dir.path().join("out");
    extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    })
    .unwrap();

    let extracted = out.join("tool");
    assert_eq!(
        fs::metadata(&extracted).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        FileTime::from_last_modification_time(&fs::metadata(&extracted).unwrap()).unix_seconds(),
        1_700_000_002
    );
}

#[cfg(unix)]
#[test]
fn tar_extracts_symlink_to_directory_target() {
    let dir = tempdir().unwrap();
    let archive = dir.path().join("dir-link.tar");
    {
        let file = File::create(&archive).unwrap();
        let mut tar = TarBuilder::new(file);
        let mut link_header = Header::new_gnu();
        link_header.set_entry_type(EntryType::Symlink);
        link_header.set_size(0);
        link_header.set_mode(0o777);
        tar.append_link(&mut link_header, "link", "target-dir")
            .unwrap();

        let mut dir_header = Header::new_gnu();
        dir_header.set_entry_type(EntryType::Directory);
        dir_header.set_size(0);
        dir_header.set_mode(0o755);
        tar.append_data(&mut dir_header, "target-dir", io::empty())
            .unwrap();
        append_tar_file(&mut tar, "target-dir/file.txt", b"nested\n");
        tar.finish().unwrap();
    }

    let out = dir.path().join("out");
    extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    })
    .unwrap();

    assert!(
        fs::symlink_metadata(out.join("link"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_link(out.join("link")).unwrap(),
        PathBuf::from("target-dir")
    );
    assert_eq!(
        fs::read_to_string(out.join("target-dir/file.txt")).unwrap(),
        "nested\n"
    );
}

#[test]
fn detects_input_changes_after_planning() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("data.txt"), "before\n").unwrap();
    let archive = dir.path().join("input.zip");

    let entries = arca_core::plan_entries(std::slice::from_ref(&input), &[], &archive).unwrap();
    fs::write(input.join("data.txt"), "after\n").unwrap();
    let err =
        arca_core::plan::ensure_entries_unchanged(&entries, &[input], &[], &archive).unwrap_err();
    assert!(err.to_string().contains("input tree changed"));
}

#[test]
fn zip_extract_and_test_accept_parallel_jobs() {
    let dir = tempdir().unwrap();
    let input = write_numbered_input(dir.path(), 32);

    let archive = dir.path().join("input.zip");
    compress(base_compress(input.clone(), archive.clone())).unwrap();
    test(TestOptions {
        archive: archive.clone(),
        jobs: 4,
        password: None,
    })
    .unwrap();

    let out = dir.path().join("out");
    extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 4,
        password: None,
    })
    .unwrap();

    assert_numbered_output(&out, 32);
}

#[test]
fn zip_compress_accepts_parallel_jobs() {
    let dir = tempdir().unwrap();
    let input = write_numbered_input(dir.path(), 40);
    let archive = dir.path().join("input.zip");
    let mut options = base_compress(input, archive.clone());
    options.jobs = 4;
    options.level = Some(6);
    compress(options).unwrap();

    test(TestOptions {
        archive: archive.clone(),
        jobs: 4,
        password: None,
    })
    .unwrap();

    let out = dir.path().join("out");
    extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 4,
        password: None,
    })
    .unwrap();
    assert_numbered_output(&out, 40);
}

#[test]
fn aes_zip_compress_accepts_parallel_jobs() {
    let dir = tempdir().unwrap();
    let input = write_numbered_input(dir.path(), 40);
    let archive = dir.path().join("secret.zip");
    let password = Password::new(b"secret".to_vec());
    let mut options = base_compress(input, archive.clone());
    options.jobs = 4;
    options.encryption = Encryption::Aes256(password.clone());
    compress(options).unwrap();

    test(TestOptions {
        archive: archive.clone(),
        jobs: 4,
        password: Some(password.clone()),
    })
    .unwrap();

    let out = dir.path().join("out");
    extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 4,
        password: Some(password),
    })
    .unwrap();
    assert_numbered_output(&out, 40);
}

#[test]
fn zipcrypto_zip_compress_rejects_parallel_jobs() {
    let dir = tempdir().unwrap();
    let input = write_numbered_input(dir.path(), 2);
    let archive = dir.path().join("legacy.zip");
    let mut options = base_compress(input, archive);
    options.jobs = 2;
    options.encryption = Encryption::ZipCrypto(Password::new(b"secret".to_vec()));

    let err = compress(options).unwrap_err();
    assert!(
        matches!(err, arca_core::ArcaError::Unsupported(_)),
        "expected unsupported error, got {err}"
    );
}

#[test]
fn zipcrypto_zip_rejects_wrong_password() {
    let dir = tempdir().unwrap();
    let input = write_numbered_input(dir.path(), 4);
    let archive = dir.path().join("legacy.zip");
    let mut options = base_compress(input, archive.clone());
    options.encryption = Encryption::ZipCrypto(Password::new(b"secret".to_vec()));
    compress(options).unwrap();

    assert_password_error(test(TestOptions {
        archive: archive.clone(),
        jobs: 1,
        password: Some(Password::new(b"wrong".to_vec())),
    }));
    let out = dir.path().join("out");
    assert_password_error(extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: Some(Password::new(b"wrong".to_vec())),
    }));
    assert!(!out.exists(), "failed extraction should not publish output");
}

#[test]
fn zip_test_rejects_corrupt_payload_as_integrity_error() {
    let dir = tempdir().unwrap();
    let archive = dir.path().join("corrupt.zip");
    write_stored_zip_file(&archive, "data.txt", b"stored payload\n");
    corrupt_first_occurrence(&archive, b"stored payload\n");

    let err = test(TestOptions {
        archive,
        jobs: 4,
        password: None,
    })
    .unwrap_err();
    assert_eq!(ExitCode::from(&err), ExitCode::Integrity, "{err}");
}

#[test]
fn single_stream_test_rejects_truncated_payload_as_integrity_error() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("data.txt");
    fs::write(&input, "single stream payload\n").unwrap();
    let archive = dir.path().join("data.txt.gz");
    compress(base_compress(input, archive.clone())).unwrap();
    truncate_file(&archive, 4);

    let err = list(archive.clone()).unwrap_err();
    assert_eq!(ExitCode::from(&err), ExitCode::Integrity, "{err}");
    let err = test(TestOptions {
        archive,
        jobs: 1,
        password: None,
    })
    .unwrap_err();
    assert_eq!(ExitCode::from(&err), ExitCode::Integrity, "{err}");
}

#[test]
fn single_stream_extract_rejects_truncated_payload_as_integrity_error() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("data.txt");
    fs::write(&input, "single stream payload\n").unwrap();
    let archive = dir.path().join("data.txt.gz");
    compress(base_compress(input, archive.clone())).unwrap();
    truncate_file(&archive, 4);

    let out = dir.path().join("out.txt");
    let err = extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    })
    .unwrap_err();
    assert_eq!(ExitCode::from(&err), ExitCode::Integrity, "{err}");
    assert!(!out.exists(), "failed extraction should not publish output");
}

#[test]
fn non_zip_compress_rejects_parallel_jobs() {
    let dir = tempdir().unwrap();
    let input = write_numbered_input(dir.path(), 2);
    let archive = dir.path().join("input.tar.gz");
    let mut options = base_compress(input, archive);
    options.jobs = 2;

    let err = compress(options).unwrap_err();
    assert!(
        matches!(err, arca_core::ArcaError::Unsupported(_)),
        "expected unsupported error, got {err}"
    );
}

#[test]
fn zip_list_test_and_extract_reject_traversal() {
    let dir = tempdir().unwrap();
    let archive = dir.path().join("evil.zip");
    write_zip_file(&archive, "../escape.txt", b"bad");

    assert_security_error(list(archive.clone()));
    assert_security_error(test(TestOptions {
        archive: archive.clone(),
        jobs: 4,
        password: None,
    }));
    let out = dir.path().join("out");
    assert_security_error(extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 4,
        password: None,
    }));
    assert!(!out.exists(), "failed extraction should not publish output");
}

#[test]
fn zip_list_test_and_extract_reject_unsafe_paths() {
    let bad_names = [
        "/absolute.txt",
        "\\absolute.txt",
        "C:/windows-drive.txt",
        "dir\\backslash.txt",
        "file:ads",
        "trailing-dot.",
        "trailing-space ",
        "NUL.txt",
    ];

    for bad_name in bad_names {
        let dir = tempdir().unwrap();
        let archive = dir.path().join("evil.zip");
        write_zip_file(&archive, bad_name, b"bad");

        assert_security_error(list(archive.clone()));
        assert_security_error(test(TestOptions {
            archive: archive.clone(),
            jobs: 4,
            password: None,
        }));
        let out = dir.path().join("out");
        assert_security_error(extract(ExtractOptions {
            archive,
            output: Some(out.clone()),
            overwrite: false,
            jobs: 4,
            password: None,
        }));
        assert!(!out.exists(), "failed extraction should not publish output");
    }
}

#[test]
fn zip_list_test_and_extract_reject_case_collisions() {
    let dir = tempdir().unwrap();
    let archive = dir.path().join("collision.zip");
    {
        let file = File::create(&archive).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = FileOptions::<()>::default();
        zip.start_file("Readme.txt", options).unwrap();
        zip.write_all(b"one").unwrap();
        zip.start_file("README.TXT", options).unwrap();
        zip.write_all(b"two").unwrap();
        zip.finish().unwrap();
    }

    assert_security_error(list(archive.clone()));
    assert_security_error(test(TestOptions {
        archive: archive.clone(),
        jobs: 4,
        password: None,
    }));
    assert_security_error(extract(ExtractOptions {
        archive,
        output: Some(dir.path().join("out")),
        overwrite: false,
        jobs: 4,
        password: None,
    }));
}

#[test]
fn tar_list_test_and_extract_reject_case_collisions() {
    let dir = tempdir().unwrap();
    let archive = dir.path().join("collision.tar");
    {
        let file = File::create(&archive).unwrap();
        let mut tar = TarBuilder::new(file);
        append_tar_file(&mut tar, "Readme.txt", b"one");
        append_tar_file(&mut tar, "README.TXT", b"two");
        tar.finish().unwrap();
    }

    assert_security_error(list(archive.clone()));
    assert_security_error(test(TestOptions {
        archive: archive.clone(),
        jobs: 1,
        password: None,
    }));
    let out = dir.path().join("out");
    assert_security_error(extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    }));
    assert!(!out.exists(), "failed extraction should not publish output");
}

#[test]
fn zip_test_and_extract_reject_escaping_symlink_targets() {
    let dir = tempdir().unwrap();
    let archive = dir.path().join("evil-link.zip");
    {
        let file = File::create(&archive).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.add_symlink("link", "../outside", FileOptions::<()>::default())
            .unwrap();
        zip.finish().unwrap();
    }

    assert_security_error(list(archive.clone()));
    assert_security_error(test(TestOptions {
        archive: archive.clone(),
        jobs: 4,
        password: None,
    }));
    assert_security_error(extract(ExtractOptions {
        archive,
        output: Some(dir.path().join("out")),
        overwrite: false,
        jobs: 4,
        password: None,
    }));
}

#[test]
fn zip_and_tar_reject_non_directory_prefix_conflicts() {
    let dir = tempdir().unwrap();
    let zip_archive = dir.path().join("prefix-conflict.zip");
    {
        let file = File::create(&zip_archive).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.add_symlink("link", "target", FileOptions::<()>::default())
            .unwrap();
        zip.start_file("link/file.txt", FileOptions::<()>::default())
            .unwrap();
        zip.write_all(b"bad\n").unwrap();
        zip.finish().unwrap();
    }

    assert_security_error(list(zip_archive.clone()));
    assert_security_error(test(TestOptions {
        archive: zip_archive.clone(),
        jobs: 4,
        password: None,
    }));
    let zip_out = dir.path().join("zip-out");
    assert_security_error(extract(ExtractOptions {
        archive: zip_archive,
        output: Some(zip_out.clone()),
        overwrite: false,
        jobs: 4,
        password: None,
    }));
    assert!(
        !zip_out.exists(),
        "failed extraction should not publish output"
    );

    let tar_archive = dir.path().join("prefix-conflict.tar");
    {
        let file = File::create(&tar_archive).unwrap();
        let mut tar = TarBuilder::new(file);
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        tar.append_link(&mut header, "link", "target").unwrap();
        append_tar_file(&mut tar, "link/file.txt", b"bad\n");
        tar.finish().unwrap();
    }

    assert_security_error(list(tar_archive.clone()));
    assert_security_error(test(TestOptions {
        archive: tar_archive.clone(),
        jobs: 1,
        password: None,
    }));
    let tar_out = dir.path().join("tar-out");
    assert_security_error(extract(ExtractOptions {
        archive: tar_archive,
        output: Some(tar_out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    }));
    assert!(
        !tar_out.exists(),
        "failed extraction should not publish output"
    );
}

#[test]
fn tar_list_test_and_extract_reject_hardlinks() {
    let dir = tempdir().unwrap();
    let archive = dir.path().join("hardlink.tar");
    {
        let file = File::create(&archive).unwrap();
        let mut tar = TarBuilder::new(file);
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Link);
        header.set_size(0);
        header.set_mode(0o644);
        tar.append_link(&mut header, "hardlink", "target").unwrap();
        tar.finish().unwrap();
    }

    assert_security_error(list(archive.clone()));
    assert_security_error(test(TestOptions {
        archive: archive.clone(),
        jobs: 1,
        password: None,
    }));
    let out = dir.path().join("out");
    assert_security_error(extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    }));
    assert!(!out.exists(), "failed extraction should not publish output");
}

#[test]
fn compressed_tar_rejects_hardlinks_without_publishing_output() {
    let dir = tempdir().unwrap();
    let archive = dir.path().join("hardlink.tar.gz");
    {
        let file = File::create(&archive).unwrap();
        let encoder = GzEncoder::new(file, Compression::default());
        let mut tar = TarBuilder::new(encoder);
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Link);
        header.set_size(0);
        header.set_mode(0o644);
        tar.append_link(&mut header, "hardlink", "target").unwrap();
        let encoder = tar.into_inner().unwrap();
        encoder.finish().unwrap();
    }

    assert_security_error(list(archive.clone()));
    assert_security_error(test(TestOptions {
        archive: archive.clone(),
        jobs: 1,
        password: None,
    }));
    let out = dir.path().join("out");
    assert_security_error(extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    }));
    assert!(!out.exists(), "failed extraction should not publish output");
}

#[test]
fn compressed_tar_rejects_truncated_archive_as_integrity_error() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("data.txt"), "tar payload\n").unwrap();
    let archive = dir.path().join("input.tar.gz");
    compress(base_compress(input, archive.clone())).unwrap();
    truncate_file(&archive, 32);

    let err = list(archive.clone()).unwrap_err();
    assert_eq!(ExitCode::from(&err), ExitCode::Integrity, "{err}");
    let err = test(TestOptions {
        archive: archive.clone(),
        jobs: 1,
        password: None,
    })
    .unwrap_err();
    assert_eq!(ExitCode::from(&err), ExitCode::Integrity, "{err}");

    let out = dir.path().join("out");
    let err = extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    })
    .unwrap_err();
    assert_eq!(ExitCode::from(&err), ExitCode::Integrity, "{err}");
    assert!(!out.exists(), "failed extraction should not publish output");
}

#[test]
fn tar_test_and_extract_reject_truncated_file_payload_as_integrity_error() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("data.txt"), vec![b'x'; 4096]).unwrap();
    let archive = dir.path().join("input.tar");
    compress(base_compress(input, archive.clone())).unwrap();
    truncate_file(&archive, 2048);

    let err = test(TestOptions {
        archive: archive.clone(),
        jobs: 1,
        password: None,
    })
    .unwrap_err();
    assert_eq!(ExitCode::from(&err), ExitCode::Integrity, "{err}");

    let out = dir.path().join("out");
    let err = extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    })
    .unwrap_err();
    assert_eq!(ExitCode::from(&err), ExitCode::Integrity, "{err}");
    assert!(!out.exists(), "failed extraction should not publish output");
}

#[cfg(unix)]
#[test]
fn tar_list_test_and_extract_reject_non_utf8_paths() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let dir = tempdir().unwrap();
    let archive = dir.path().join("non-utf8.tar");
    {
        let file = File::create(&archive).unwrap();
        let mut tar = TarBuilder::new(file);
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Regular);
        header.set_size(0);
        header.set_mode(0o644);
        let path = PathBuf::from(OsString::from_vec(b"bad-\xFF".to_vec()));
        tar.append_data(&mut header, path, io::empty()).unwrap();
        tar.finish().unwrap();
    }

    assert_non_utf8_path_error(list(archive.clone()));
    assert_non_utf8_path_error(test(TestOptions {
        archive: archive.clone(),
        jobs: 1,
        password: None,
    }));
    let out = dir.path().join("out");
    assert_non_utf8_path_error(extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    }));
    assert!(!out.exists(), "failed extraction should not publish output");
}

#[test]
fn tar_list_test_and_extract_reject_special_entries() {
    let dir = tempdir().unwrap();
    let archive = dir.path().join("special.tar");
    {
        let file = File::create(&archive).unwrap();
        let mut tar = TarBuilder::new(file);
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Fifo);
        header.set_size(0);
        header.set_mode(0o644);
        tar.append_data(&mut header, "fifo", io::empty()).unwrap();
        tar.finish().unwrap();
    }

    assert_security_error(list(archive.clone()));
    assert_security_error(test(TestOptions {
        archive: archive.clone(),
        jobs: 1,
        password: None,
    }));
    let out = dir.path().join("out");
    assert_security_error(extract(ExtractOptions {
        archive,
        output: Some(out.clone()),
        overwrite: false,
        jobs: 1,
        password: None,
    }));
    assert!(!out.exists(), "failed extraction should not publish output");
}

fn write_zip_file(path: &Path, name: &str, contents: &[u8]) {
    let file = File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file(name, FileOptions::<()>::default()).unwrap();
    zip.write_all(contents).unwrap();
    zip.finish().unwrap();
}

fn write_stored_zip_file(path: &Path, name: &str, contents: &[u8]) {
    let file = File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file(name, options).unwrap();
    zip.write_all(contents).unwrap();
    zip.finish().unwrap();
}

fn corrupt_first_occurrence(path: &Path, needle: &[u8]) {
    let mut bytes = fs::read(path).unwrap();
    let offset = bytes
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("payload should be present in stored zip");
    bytes[offset] ^= 0xff;
    fs::write(path, bytes).unwrap();
}

fn truncate_file(path: &Path, bytes: u64) {
    let file = File::options().write(true).open(path).unwrap();
    let len = file.metadata().unwrap().len();
    assert!(len > bytes, "test archive should be larger than truncation");
    file.set_len(len - bytes).unwrap();
}

fn append_tar_file<W: Write>(tar: &mut TarBuilder<W>, name: &str, contents: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Regular);
    header.set_size(contents.len() as u64);
    header.set_mode(0o644);
    tar.append_data(&mut header, name, contents).unwrap();
}

fn write_empty_archive(path: &Path, suffix: &str) {
    let file = File::create(path).unwrap();
    match suffix {
        "zip" => {
            zip::ZipWriter::new(file).finish().unwrap();
        }
        "tar" => {
            TarBuilder::new(file).finish().unwrap();
        }
        "tar.gz" => {
            let encoder = GzEncoder::new(file, Compression::default());
            let tar = TarBuilder::new(encoder);
            let encoder = tar.into_inner().unwrap();
            encoder.finish().unwrap();
        }
        _ => unreachable!("unsupported empty archive suffix"),
    }
}

fn assert_security_error<T>(result: arca_core::ArcaResult<T>) {
    let err = match result {
        Ok(_) => panic!("expected security error"),
        Err(err) => err,
    };
    assert!(
        matches!(err, arca_core::ArcaError::Security(_)),
        "expected security error, got {err}"
    );
}

fn assert_password_error<T>(result: arca_core::ArcaResult<T>) {
    let err = match result {
        Ok(_) => panic!("expected password error"),
        Err(err) => err,
    };
    assert!(
        matches!(err, arca_core::ArcaError::Zip(_)),
        "expected zip password error, got {err}"
    );
}

fn assert_non_utf8_path_error<T>(result: arca_core::ArcaResult<T>) {
    let err = match result {
        Ok(_) => panic!("expected non-UTF-8 path error"),
        Err(err) => err,
    };
    assert!(
        matches!(err, arca_core::ArcaError::NonUtf8Path(_)),
        "expected non-UTF-8 path error, got {err}"
    );
}
