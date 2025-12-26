use reactor::RuntimeBuilder;
use reactor::fs::File;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn file_read_write_roundtrip() {
    let mut runtime = RuntimeBuilder::new().enable_fs().build();

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock drift")
        .as_nanos();

    let path = std::env::temp_dir().join(format!(
        "reactor-file-{}-{}.tmp",
        std::process::id(),
        unique
    ));
    let path_string = path.to_string_lossy().into_owned();

    runtime
        .block_on(async {
            let writer = File::create(&path_string).await?;
            writer.write_all(b"hello world").await?;
            drop(writer);

            let reader = File::open(&path_string).await?;
            let mut buffer = [0u8; 11];
            let n = reader.read(&mut buffer).await?;

            assert_eq!(n, 11);
            assert_eq!(&buffer[..n], b"hello world");

            Ok::<(), std::io::Error>(())
        })
        .expect("file operations should succeed");

    let _ = std::fs::remove_file(path);
}
