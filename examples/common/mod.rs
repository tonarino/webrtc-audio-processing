//! Functionality shared by multiple examples.

/// De-interleaves multi-channel frame `src` into `dst`.
///
/// ```text
/// e.g. A stereo frame with 3 samples:
///
/// Interleaved
/// +---+---+---+---+---+---+
/// |L0 |R0 |L1 |R1 |L2 |R2 |
/// +---+---+---+---+---+---+
///
/// Non-interleaved
/// +---+---+---+
/// |L0 |L1 |L2 |
/// +---+---+---+
/// |R0 |R1 |R2 |
/// +---+---+---+
/// ```
pub fn deinterleave<T: AsMut<[f32]>>(src: &[f32], dst: &mut [T]) {
    let num_channels = dst.len();
    let num_samples = dst[0].as_mut().len();
    assert_eq!(src.len(), num_channels * num_samples);
    for channel_index in 0..num_channels {
        for sample_index in 0..num_samples {
            dst[channel_index].as_mut()[sample_index] =
                src[num_channels * sample_index + channel_index];
        }
    }
}

/// Reverts the `deinterleave` operation.
pub fn interleave<T: AsRef<[f32]>>(src: &[T], dst: &mut [f32]) {
    let num_channels = src.len();
    let num_samples = src[0].as_ref().len();
    assert_eq!(dst.len(), num_channels * num_samples);
    for channel_index in 0..num_channels {
        for sample_index in 0..num_samples {
            dst[num_channels * sample_index + channel_index] =
                src[channel_index].as_ref()[sample_index];
        }
    }
}
