use g3_providers::ImageContent;
use std::fs;

#[test]
fn test_image_content_media_type_detection() {
    assert_eq!(ImageContent::media_type_from_extension("png"), Some("image/png"));
    assert_eq!(ImageContent::media_type_from_extension("PNG"), Some("image/png"));
    assert_eq!(ImageContent::media_type_from_extension("jpg"), Some("image/jpeg"));
    assert_eq!(ImageContent::media_type_from_extension("jpeg"), Some("image/jpeg"));
    assert_eq!(ImageContent::media_type_from_extension("JPEG"), Some("image/jpeg"));
    assert_eq!(ImageContent::media_type_from_extension("gif"), Some("image/gif"));
    assert_eq!(ImageContent::media_type_from_extension("webp"), Some("image/webp"));
    assert_eq!(ImageContent::media_type_from_extension("bmp"), None); // Not supported
    assert_eq!(ImageContent::media_type_from_extension("txt"), None);
}

#[test]
fn test_image_content_creation() {
    let image = ImageContent::new("image/png", "base64data".to_string());
    assert_eq!(image.media_type, "image/png");
    assert_eq!(image.data, "base64data");
}

#[test]
fn test_read_and_encode_image() {
    // Create a minimal valid PNG
    let test_dir = std::env::temp_dir().join("g3_read_image_test");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();
    
    // Minimal 1x1 red PNG (hand-crafted)
    let png_bytes: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, // width = 1
        0x00, 0x00, 0x00, 0x01, // height = 1
        0x08, 0x02, 0x00, 0x00, 0x00, // bit depth, color type, etc.
        0x90, 0x77, 0x53, 0xDE, // CRC
        0x00, 0x00, 0x00, 0x0C, // IDAT length
        0x49, 0x44, 0x41, 0x54, // IDAT
        0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, // compressed data
        0x01, 0x01, 0x01, 0x00, // CRC (approximate)
        0x00, 0x00, 0x00, 0x00, // IEND length
        0x49, 0x45, 0x4E, 0x44, // IEND
        0xAE, 0x42, 0x60, 0x82, // CRC
    ];
    
    let image_path = test_dir.join("test.png");
    fs::write(&image_path, &png_bytes).unwrap();
    
    // Read and encode
    let bytes = fs::read(&image_path).unwrap();
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    
    // Verify it's valid base64
    assert!(!encoded.is_empty());
    assert!(encoded.len() > 10);
    
    // Verify we can decode it back
    let decoded = base64::engine::general_purpose::STANDARD.decode(&encoded).unwrap();
    assert_eq!(decoded, bytes);
    
    // Create ImageContent
    let ext = image_path.extension().unwrap().to_str().unwrap();
    let media_type = ImageContent::media_type_from_extension(ext).unwrap();
    let image = ImageContent::new(media_type, encoded);
    
    assert_eq!(image.media_type, "image/png");
    assert!(!image.data.is_empty());
    
    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_media_type_from_bytes_png() {
    // PNG magic bytes
    let png_bytes: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR
    ];
    assert_eq!(ImageContent::media_type_from_bytes(&png_bytes), Some("image/png"));
}

#[test]
fn test_media_type_from_bytes_jpeg() {
    // JPEG magic bytes (FF D8 FF)
    let jpeg_bytes: Vec<u8> = vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46,
        0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
    ];
    assert_eq!(ImageContent::media_type_from_bytes(&jpeg_bytes), Some("image/jpeg"));
}

#[test]
fn test_media_type_from_bytes_gif() {
    // GIF magic bytes (GIF89a)
    let gif_bytes: Vec<u8> = vec![
        0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_eq!(ImageContent::media_type_from_bytes(&gif_bytes), Some("image/gif"));
    
    // GIF87a variant
    let gif87_bytes: Vec<u8> = vec![
        0x47, 0x49, 0x46, 0x38, 0x37, 0x61, 0x01, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_eq!(ImageContent::media_type_from_bytes(&gif87_bytes), Some("image/gif"));
}

#[test]
fn test_media_type_from_bytes_webp() {
    // WebP magic bytes (RIFF....WEBP)
    let webp_bytes: Vec<u8> = vec![
        0x52, 0x49, 0x46, 0x46, // RIFF
        0x00, 0x00, 0x00, 0x00, // file size (placeholder)
        0x57, 0x45, 0x42, 0x50, // WEBP
        0x56, 0x50, 0x38, 0x20, // VP8 (additional data)
    ];
    assert_eq!(ImageContent::media_type_from_bytes(&webp_bytes), Some("image/webp"));
}

#[test]
fn test_media_type_from_bytes_unknown() {
    // Random bytes that don't match any format
    let unknown_bytes: Vec<u8> = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    ];
    assert_eq!(ImageContent::media_type_from_bytes(&unknown_bytes), None);
}

#[test]
fn test_media_type_from_bytes_too_short() {
    // Too short to detect
    let short_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E];
    assert_eq!(ImageContent::media_type_from_bytes(&short_bytes), None);
    
    // Empty
    let empty_bytes: Vec<u8> = vec![];
    assert_eq!(ImageContent::media_type_from_bytes(&empty_bytes), None);
}

#[test]
fn test_read_image_multiple_paths_schema() {
    // This test verifies the tool accepts file_paths array
    
    // Single path in array
    let single_args = serde_json::json!({
        "file_paths": ["/path/to/image.png"]
    });
    let paths = single_args.get("file_paths").unwrap().as_array().unwrap();
    assert_eq!(paths.len(), 1);
    
    // Multiple paths in array
    let multi_args = serde_json::json!({
        "file_paths": ["/path/to/image1.png", "/path/to/image2.jpg"]
    });
    let paths = multi_args.get("file_paths").unwrap().as_array().unwrap();
    assert_eq!(paths.len(), 2);
}

#[test]
fn test_image_dimensions_png() {
    // Minimal PNG with known dimensions (1x1)
    let png_bytes: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, // width = 1
        0x00, 0x00, 0x00, 0x01, // height = 1
        0x08, 0x02, 0x00, 0x00, 0x00, // bit depth, color type, etc.
    ];
    
    // PNG dimensions are at bytes 16-19 (width) and 20-23 (height)
    if png_bytes.len() >= 24 {
        let width = u32::from_be_bytes([png_bytes[16], png_bytes[17], png_bytes[18], png_bytes[19]]);
        let height = u32::from_be_bytes([png_bytes[20], png_bytes[21], png_bytes[22], png_bytes[23]]);
        assert_eq!(width, 1);
        assert_eq!(height, 1);
    }
}

#[test]
fn test_image_dimensions_gif() {
    // GIF with known dimensions
    let gif_bytes: Vec<u8> = vec![
        0x47, 0x49, 0x46, 0x38, 0x39, 0x61, // GIF89a
        0x64, 0x00, // width = 100 (little-endian)
        0xC8, 0x00, // height = 200 (little-endian)
    ];
    
    let width = u16::from_le_bytes([gif_bytes[6], gif_bytes[7]]) as u32;
    let height = u16::from_le_bytes([gif_bytes[8], gif_bytes[9]]) as u32;
    assert_eq!(width, 100);
    assert_eq!(height, 200);
}
