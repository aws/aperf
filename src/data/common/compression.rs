use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use flate2::write::ZlibEncoder;
use flate2::Compression;

pub fn zlib_encoder() -> ZlibEncoder<Vec<u8>> {
    // In APerf's usecase, compression level 3 gives the best size vs time tradeoff.
    ZlibEncoder::new(Vec::new(), Compression::new(3))
}

/// Splits a zlib stream into the base64 chunks stored in the report, since browsers parse many short
/// strings much faster than one long string.
pub fn base64_chunks(zlib_stream: &[u8]) -> Vec<String> {
    // The steam is splitted into 3MiB chunks. Any value that is a multiply of 3 can work here,
    // since base64 turns every 3 bytes into 4 characters, and such chunk size means no chunk
    // requires additional paddings and can be splitted and processed separately.
    // 3 MiB is a heuristic value, but in a simple benchmark it gave the lowest median for
    // processing latency.
    zlib_stream
        .chunks(3 * 1024 * 1024)
        .map(|chunk| BASE64.encode(chunk))
        .collect()
}
