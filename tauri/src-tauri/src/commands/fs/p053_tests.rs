//! P053：直接调用 Host 生产读取路径，不替换解密/大小检查。
use super::*;
use base64::Engine;
use std::fs::File;
use tempfile::TempDir;

#[test]
fn text_preview_checks_the_real_ten_mib_boundary_and_keeps_the_file() {
    let app = tauri::test::mock_app();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("public.txt");
    for size in [
        MAX_DATA_URL_SIZE - 1,
        MAX_DATA_URL_SIZE,
        MAX_DATA_URL_SIZE + 1,
    ] {
        let bytes = vec![b'p'; size as usize];
        fs_std::write(&path, &bytes).unwrap();
        let result = futures::executor::block_on(fs_read_file_as_text(
            app.handle().clone(),
            path.to_string_lossy().into_owned(),
        ));
        if size <= MAX_DATA_URL_SIZE {
            assert_eq!(result.unwrap().as_bytes(), bytes);
        } else {
            assert!(result.unwrap_err().contains("File too large"));
        }
        assert_eq!(fs_std::read(&path).unwrap(), bytes);
    }
}

#[test]
fn data_url_preview_checks_the_real_boundary_before_base64_and_keeps_the_file() {
    let app = tauri::test::mock_app();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("public.txt");
    for size in [
        MAX_DATA_URL_SIZE - 1,
        MAX_DATA_URL_SIZE,
        MAX_DATA_URL_SIZE + 1,
    ] {
        let bytes = vec![42; size as usize];
        fs_std::write(&path, &bytes).unwrap();
        let result = futures::executor::block_on(fs_read_file_as_data_url(
            app.handle().clone(),
            path.to_string_lossy().into_owned(),
        ));
        if size <= MAX_DATA_URL_SIZE {
            let url = result.unwrap();
            let encoded = url.strip_prefix("data:text/plain;base64,").unwrap();
            assert_eq!(
                base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .unwrap(),
                bytes
            );
        } else {
            assert!(result.unwrap_err().contains("File too large"));
        }
        assert_eq!(fs_std::read(&path).unwrap(), bytes);
    }
}

#[test]
fn encrypted_input_without_managed_state_is_not_returned_as_plaintext() {
    let app = tauri::test::mock_app();
    let dir = TempDir::new().unwrap();
    let plain = dir.path().join("public.txt");
    let cipher = dir.path().join("public.solc");
    fs_std::write(&plain, b"public contents").unwrap();
    solosoul_core::attachment_crypto::encrypt_file_stream(&[42; 32], &plain, &cipher).unwrap();
    let before = fs_std::read(&cipher).unwrap();
    let error = read_file_with_attachment_decrypt(app.handle(), &cipher, 32).unwrap_err();
    assert!(error.contains("无法获取附件解密密钥"));
    assert_eq!(fs_std::read(&cipher).unwrap(), before);
}

#[tokio::test]
async fn image_preview_keeps_fifty_mib_allowance_and_shrinks_a_real_png_above_ten_mib() {
    let app = tauri::test::mock_app();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("public.png");
    let mut seed = 42u32;
    let pixels: Vec<u8> = (0..2048 * 2048 * 3)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 24) as u8
        })
        .collect();
    image::RgbImage::from_raw(2048, 2048, pixels)
        .unwrap()
        .save(&path)
        .unwrap();
    let size = fs_std::metadata(&path).unwrap().len();
    assert!(size > MAX_DATA_URL_SIZE && size <= MAX_IMAGE_PREVIEW_READ_SIZE);
    let generic =
        fs_read_file_as_data_url(app.handle().clone(), path.to_string_lossy().into_owned()).await;
    assert!(generic.unwrap_err().contains("File too large"));
    let preview = fs_read_image_preview(
        app.handle().clone(),
        path.to_string_lossy().into_owned(),
        64,
    )
    .await
    .unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(preview.strip_prefix("data:image/jpeg;base64,").unwrap())
        .unwrap();
    let image = image::load_from_memory(&bytes).unwrap();
    assert_eq!((image.width(), image.height()), (64, 64));
    assert_eq!(fs_std::metadata(&path).unwrap().len(), size);
}

#[tokio::test]
async fn image_preview_rejects_fifty_mib_plus_one_before_trying_to_decode() {
    let app = tauri::test::mock_app();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("public.bin");
    let file = File::create(&path).unwrap();
    file.set_len(MAX_IMAGE_PREVIEW_READ_SIZE + 1).unwrap();
    drop(file);
    let error = fs_read_image_preview(
        app.handle().clone(),
        path.to_string_lossy().into_owned(),
        64,
    )
    .await
    .unwrap_err();
    assert!(error.contains("File too large"));
    assert_eq!(
        fs_std::metadata(path).unwrap().len(),
        MAX_IMAGE_PREVIEW_READ_SIZE + 1
    );
}
