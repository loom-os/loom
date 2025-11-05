use tokio::sync::mpsc;

#[inline]
fn f32_to_i16(s: f32) -> i16 {
    let s = s.clamp(-1.0, 1.0);
    (s * i16::MAX as f32) as i16
}

#[inline]
fn u16_to_i16(s: u16) -> i16 {
    // Map 0..=65535 to -32768..=32767
    (s as i32 - 32768) as i16
}

#[inline]
fn u8_to_i16(s: u8) -> i16 {
    // Map 0..=255 to approximately -32768..=+32767 by centering at 128 and scaling
    (s as i16 - 128) << 8
}

fn emit_chunks<F: FnMut(Vec<i16>)>(
    data: &[i16],
    acc: &mut Vec<i16>,
    chunk_samples: usize,
    mut emit: F,
) {
    acc.extend_from_slice(data);
    while acc.len() >= chunk_samples {
        let chunk: Vec<i16> = acc.drain(..chunk_samples).collect();
        emit(chunk);
    }
}

#[test]
fn test_f32_to_i16_bounds() {
    assert_eq!(f32_to_i16(-1.0), i16::MIN + 1); // scaling cast behavior
    assert_eq!(f32_to_i16(0.0), 0);
    assert!(f32_to_i16(1.0) >= i16::MAX - 1);
    // Out-of-range clamps
    assert!(f32_to_i16(2.0) >= i16::MAX - 1);
    assert_eq!(f32_to_i16(-2.0), i16::MIN + 1);
}

#[test]
fn test_u16_to_i16_mapping() {
    assert_eq!(u16_to_i16(0), -32768);
    assert_eq!(u16_to_i16(32768), 0);
    assert_eq!(u16_to_i16(65535), 32767);
}

#[tokio::test]
async fn test_push_and_ship_chunking() {
    let chunk_samples = 8usize;
    let (tx, mut rx) = mpsc::channel::<Vec<i16>>(10);
    let mut acc: Vec<i16> = Vec::new();

    // Feed 3.5 chunks worth of samples in uneven slices
    let input: Vec<i16> = (0..(chunk_samples * 3 + 4)).map(|i| i as i16).collect();
    // First half
    emit_chunks(&input[0..5], &mut acc, chunk_samples, |c| {
        let _ = tx.try_send(c);
    });
    // Second half to complete first chunk
    emit_chunks(&input[5..8], &mut acc, chunk_samples, |c| {
        let _ = tx.try_send(c);
    });
    // Two more whole chunks
    emit_chunks(
        &input[8..(8 + chunk_samples)],
        &mut acc,
        chunk_samples,
        |c| {
            let _ = tx.try_send(c);
        },
    );
    emit_chunks(
        &input[(8 + chunk_samples)..(8 + 2 * chunk_samples)],
        &mut acc,
        chunk_samples,
        |c| {
            let _ = tx.try_send(c);
        },
    );
    // Remainder (4 samples) stays buffered
    emit_chunks(
        &input[(8 + 2 * chunk_samples)..],
        &mut acc,
        chunk_samples,
        |c| {
            let _ = tx.try_send(c);
        },
    );

    // Receive 3 chunks
    let mut chunks = Vec::new();
    while let Ok(chunk) = rx.try_recv() {
        chunks.push(chunk);
    }
    assert_eq!(chunks.len(), 3);
    assert!(acc.len() > 0 && acc.len() < chunk_samples); // remainder buffered

    // Verify the concatenation of delivered chunks equals the first 3*chunk_samples inputs
    let delivered: Vec<i16> = chunks.into_iter().flatten().collect();
    assert_eq!(&delivered[..], &input[0..(3 * chunk_samples)]);
}

#[test]
fn test_u8_to_i16_mapping() {
    assert_eq!(u8_to_i16(0), -32768);
    assert_eq!(u8_to_i16(128), 0);
    assert!(u8_to_i16(255) > 32000);
}
