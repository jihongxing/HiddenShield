# Watermark Extraction Failure Analysis

## Problem
After implementing the layered verification system (adding output file hashes), watermark extraction is failing with 0% confidence. The user reports: "我就是新生成的文件，新加密，再解密的过程报错的"

## Changes Made
1. Changed `file_hash` from 4 bytes to 2 bytes in `WatermarkPayload`
2. Added `ai_flags: AIContentFlags` (2 bytes) to the payload
3. Updated all three pipelines (video, image, audio) to use the new 5-parameter `WatermarkPayload::new()` signature

## Payload Structure (32 bytes total)
```
[0..4]   Magic "HD5H" (4 bytes)
[4..12]  User Seed (8 bytes)
[12..20] Timestamp (8 bytes)
[20..24] Device ID (4 bytes)
[24..26] AI Content Flags (2 bytes) ← NEW
[26..28] File Hash (2 bytes) ← CHANGED from 4 bytes
[28..32] HMAC Auth Tag (4 bytes)
```

## Code Review

### encode_payload() - Line 286-301
```rust
pub fn encode_payload(payload: &WatermarkPayload) -> [u8; PAYLOAD_BYTES] {
    let mut buf = [0u8; PAYLOAD_BYTES];
    buf[0..4].copy_from_slice(&payload.magic);
    buf[4..12].copy_from_slice(&payload.user_seed);
    buf[12..20].copy_from_slice(&payload.timestamp.to_be_bytes());
    buf[20..24].copy_from_slice(&payload.device_id);
    let ai_flags_packed = payload.ai_flags.pack().to_be_bytes();
    buf[24..26].copy_from_slice(&ai_flags_packed);  // ✓ Correct
    buf[26..28].copy_from_slice(&payload.file_hash); // ✓ Correct (2 bytes)
    // Compute HMAC auth tag over first 28 bytes
    let mut data = [0u8; 28];
    data.copy_from_slice(&buf[..28]);
    let tag = compute_auth_tag(&data);
    buf[28..32].copy_from_slice(&tag);
    buf
}
```
✓ This looks correct

### decode_payload() - Line 304-346
```rust
pub fn decode_payload(bytes: &[u8; PAYLOAD_BYTES]) -> Result<WatermarkPayload, PipelineError> {
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    if magic != MAGIC {
        return Err(PipelineError::WatermarkExtractFailed(format!(
            "magic mismatch: expected {:02X?}, got {:02X?}",
            MAGIC, magic
        )));
    }

    let mut user_seed = [0u8; 8];
    user_seed.copy_from_slice(&bytes[4..12]);
    let timestamp = u64::from_be_bytes(bytes[12..20].try_into().unwrap());
    let mut device_id = [0u8; 4];
    device_id.copy_from_slice(&bytes[20..24]);
    let ai_flags_bits = u16::from_be_bytes(bytes[24..26].try_into().unwrap());
    let ai_flags = AIContentFlags::unpack(ai_flags_bits);
    let mut file_hash = [0u8; 2];
    file_hash.copy_from_slice(&bytes[26..28]); // ✓ Correct (2 bytes)
    let mut stored_tag = [0u8; 4];
    stored_tag.copy_from_slice(&bytes[28..32]);

    // Verify HMAC auth tag
    let mut data = [0u8; 28];
    data.copy_from_slice(&bytes[..28]);
    let computed_tag = compute_auth_tag(&data);
    if stored_tag != computed_tag {
        return Err(PipelineError::WatermarkExtractFailed(format!(
            "HMAC auth tag mismatch: stored {:02X?}, computed {:02X?}",
            stored_tag, computed_tag
        )));
    }

    Ok(WatermarkPayload {
        magic,
        user_seed,
        timestamp,
        device_id,
        ai_flags,
        file_hash,
        auth_tag: stored_tag,
    })
}
```
✓ This also looks correct

### WatermarkPayload::new() - Line 239-264
```rust
pub fn new(
    user_seed: [u8; 8],
    timestamp: u64,
    device_id: [u8; 4],
    file_hash: [u8; 2],
    ai_flags: AIContentFlags,
) -> Self {
    let mut buf = [0u8; 28];
    buf[0..4].copy_from_slice(&MAGIC);
    buf[4..12].copy_from_slice(&user_seed);
    buf[12..20].copy_from_slice(&timestamp.to_be_bytes());
    buf[20..24].copy_from_slice(&device_id);
    let ai_flags_packed = ai_flags.pack().to_be_bytes();
    buf[24..26].copy_from_slice(&ai_flags_packed);
    buf[26..28].copy_from_slice(&file_hash);
    let auth_tag = compute_auth_tag(&buf);
    Self {
        magic: MAGIC,
        user_seed,
        timestamp,
        device_id,
        ai_flags,
        file_hash,
        auth_tag,
    }
}
```
✓ This is correct

## All WatermarkPayload::new() Call Sites

### Video Pipeline (scheduler.rs:232)
```rust
let payload = WatermarkPayload::new(
    id_bytes.user_seed, 
    timestamp, 
    id_bytes.device_id, 
    file_hash,  // 2 bytes from compute_file_hash_prefix()
    ai_flags
);
```
✓ Correct - 5 parameters

### Image Pipeline (scheduler.rs:529)
```rust
let payload = WatermarkPayload::new(
    id_bytes.user_seed, 
    timestamp, 
    id_bytes.device_id, 
    file_hash,  // 2 bytes from compute_file_hash_prefix()
    ai_flags
);
```
✓ Correct - 5 parameters

### Audio Pipeline (scheduler.rs:690)
```rust
let payload = WatermarkPayload::new(
    id_bytes.user_seed, 
    timestamp, 
    id_bytes.device_id, 
    file_hash,  // 2 bytes from compute_file_hash_prefix()
    ai_flags
);
```
✓ Correct - 5 parameters

### Test Code (image_watermark.rs:557, 587, 612)
All test code has been updated to use the correct 5-parameter signature with 2-byte file_hash.

## Hypothesis

The serialization/deserialization code looks correct. The issue might be:

1. **Compilation issue**: The old code might still be running. Need to rebuild.
2. **Database migration**: Old records in the database might have 4-byte hashes stored.
3. **HMAC secret mismatch**: If the HMAC secret changed between embedding and extraction.
4. **Bit-level corruption**: The watermark embedding/extraction algorithm itself might have an issue.

## Next Steps

1. Verify the project compiles successfully
2. Test with a completely fresh file (not from old database)
3. Add debug logging to see what's being extracted vs what's expected
4. Check if the magic number is being detected correctly (this would rule out embedding issues)
