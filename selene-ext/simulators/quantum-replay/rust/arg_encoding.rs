use base64::prelude::*;

#[allow(dead_code)] // used by tests
pub fn encode_measurements(measurements: &[bool]) -> String {
    let mut bytes = Vec::with_capacity(measurements.len().div_ceil(8));
    for chunk in measurements.chunks(8) {
        let mut byte = 0;
        for (i, &bit) in chunk.iter().enumerate() {
            byte |= (bit as u8) << (7 - i);
        }
        bytes.push(byte);
    }
    BASE64_STANDARD.encode(&bytes)
}
pub fn decode_measurements(encoded: &str, measurement_count: usize) -> Result<Vec<bool>, String> {
    // Decode the base64 string into a vector of bytes.
    let bytes = BASE64_STANDARD
        .decode(encoded)
        .map_err(|e| format!("Error decoding base64 measurement string: {e}"))?;

    // Prepare a vec with the expected capacity.
    let mut bools = Vec::with_capacity(measurement_count);

    // Iterate over the bytes; each byte holds 8 bits with the most significant bit first.
    for (byte_index, byte) in bytes.iter().enumerate() {
        for bit in 0..8 {
            // Compute the overall bit index.
            let overall_index = byte_index * 8 + bit;
            // If we've reached the original number of booleans, stop.
            if overall_index >= measurement_count {
                break;
            }
            // Extract the bit (shift the byte so that the target bit is in the least-significant position).
            let flag = (byte >> (7 - bit)) & 1;
            bools.push(flag == 1);
        }
    }
    if bools.len() != measurement_count {
        return Err(format!(
            "Expected {} measurements, but got {}",
            measurement_count,
            bools.len()
        ));
    }
    Ok(bools)
}
pub fn parse_measurement_strings(
    measurements_str: String,
    counts_str: String,
) -> Result<Vec<Vec<bool>>, String> {
    let counts = decode_counts(&counts_str)?;
    let total_count = counts.iter().sum::<usize>();
    let all_measurements = decode_measurements(&measurements_str, total_count)?;
    let mut result = Vec::with_capacity(counts.len());
    let mut all_measurement_iter = all_measurements.into_iter();
    for count in counts {
        let shot_result = all_measurement_iter
            .by_ref()
            .take(count)
            .collect::<Vec<bool>>();
        result.push(shot_result);
    }
    Ok(result)
}
pub fn decode_counts(encoded: &str) -> Result<Vec<usize>, String> {
    // Decode the base64 string into a vector of unsigned 32-bit integers,
    // having been represented as little-endian bytes.
    let bytes = BASE64_STANDARD
        .decode(encoded)
        .map_err(|e| format!("Error decoding base64 count string: {e}"))?;
    let mut counts = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks(4) {
        let mut count = 0;
        for (i, &byte) in chunk.iter().enumerate() {
            count |= (byte as usize) << (i * 8);
        }
        counts.push(count);
    }
    Ok(counts)
}
#[allow(dead_code)] // used by tests
pub fn encode_counts(counts: Vec<usize>) -> String {
    let mut bytes = Vec::with_capacity(counts.len() * 4);
    for count in counts {
        for i in 0..4 {
            bytes.push((count >> (i * 8)) as u8);
        }
    }
    BASE64_STANDARD.encode(&bytes)
}
