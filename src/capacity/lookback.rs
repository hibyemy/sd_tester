use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::mpsc::Sender;
use std::time::Instant;

use xxhash_rust::xxh3::xxh3_64;

use crate::engine::messages::{EngineUpdate, SessionStatus, StartRequest};
use crate::io::alignment::AlignedBuffer;
use crate::io::unbuffered::open_unbuffered_file;

const BLOCK: usize = 4096;
const CHUNK: usize = 4 * 1024 * 1024;
const ONE_GB: u64 = 1024 * 1024 * 1024;
const FIRST_MB: usize = 1024 * 1024;
const DEFAULT_BRUTE_TARGET: u64 = 8 * ONE_GB;
const PROGRESS_INTERVAL: u64 = 64 * 1024 * 1024;

pub fn run_capacity(request: StartRequest, tx: Sender<EngineUpdate>) -> std::io::Result<()> {
    let path = request
        .output_file
        .clone()
        .unwrap_or_else(|| format!("{}\\capacity_test.bin", request.target.drive_letter).into());
    let mut file = open_unbuffered_file(&path)?;
    let start = Instant::now();
    let mut chunk_buf = AlignedBuffer::new(CHUNK, 4096);
    let mut total_written = 0u64;
    let mut verified_bytes = 0u64;
    let mut next_progress = PROGRESS_INTERVAL;
    let mut next_checkpoint = ONE_GB;
    let seed = request.session_id
        ^ (request.target.drive_letter.as_bytes().first().copied().unwrap_or(b'X') as u64);
    let target_bytes = align_down_capacity(request.target.advertised_bytes.unwrap_or(DEFAULT_BRUTE_TARGET));

    let _ = tx.send(EngineUpdate::status(
        request.session_id,
        request.target.id.clone(),
        SessionStatus::Running,
        "capacity look-back running",
    ));

    while total_written < target_bytes {
        if request.is_cancelled() {
            let mut cancelled = EngineUpdate::status(
                request.session_id,
                request.target.id,
                SessionStatus::Cancelled,
                "capacity look-back cancelled",
            );
            cancelled.total_written = total_written;
            cancelled.verified_bytes = verified_bytes;
            cancelled.usable_bytes = verified_bytes;
            cancelled.target_bytes = target_bytes;
            let _ = tx.send(cancelled);
            return Ok(());
        }

        let remaining = target_bytes.saturating_sub(total_written) as usize;
        let write_len = remaining.min(CHUNK);
        let aligned_write_len = write_len - (write_len % BLOCK);
        let block_start_index = total_written / BLOCK as u64;
        fill_chunk(
            &mut chunk_buf.as_mut_slice()[..aligned_write_len],
            block_start_index,
            seed,
        );
        file.write_all(&chunk_buf.as_slice()[..aligned_write_len])?;
        total_written += aligned_write_len as u64;

        if total_written >= next_checkpoint || total_written == target_bytes {
            let first_window = FIRST_MB as u64;
            if !verify_window(&mut file, 0, first_window, seed)? {
                return emit_counterfeit_failure(
                    &tx,
                    request.session_id,
                    request.target.id,
                    total_written,
                    verified_bytes,
                    target_bytes,
                    "look-back verify failed at first region",
                );
            }

            let newest_start = total_written.saturating_sub(first_window);
            let newest_len = (total_written - newest_start).min(first_window);
            if !verify_window(&mut file, newest_start, newest_len, seed)? {
                return emit_counterfeit_failure(
                    &tx,
                    request.session_id,
                    request.target.id,
                    total_written,
                    verified_bytes,
                    target_bytes,
                    "look-back verify failed at newest region",
                );
            }

            verified_bytes = total_written;
            next_checkpoint = next_checkpoint.saturating_add(ONE_GB);
            file.seek(SeekFrom::Start(total_written))?;
        }

        if total_written >= next_progress || total_written == target_bytes {
            send_capacity_progress(
                &tx,
                &request,
                total_written,
                verified_bytes,
                target_bytes,
                start,
                "capacity look-back running",
            );
            next_progress = next_progress.saturating_add(PROGRESS_INTERVAL);
        }
    }

    let mut done = EngineUpdate::status(
        request.session_id,
        request.target.id,
        SessionStatus::Completed,
        "capacity look-back completed",
    );
    done.total_written = total_written;
    done.verified_bytes = verified_bytes;
    done.usable_bytes = verified_bytes;
    done.target_bytes = target_bytes;
    let _ = tx.send(done);
    Ok(())
}

pub fn run_capacity_bruteforce(
    request: StartRequest,
    tx: Sender<EngineUpdate>,
) -> std::io::Result<()> {
    let path = request.output_file.clone().unwrap_or_else(|| {
        format!("{}\\capacity_bruteforce.bin", request.target.drive_letter).into()
    });
    let mut file = open_unbuffered_file(&path)?;
    let target_bytes = align_down_capacity(request.target.advertised_bytes.unwrap_or(DEFAULT_BRUTE_TARGET));
    let seed = request.session_id.rotate_left(17) ^ 0xA5A5_A5A5_5A5A_5A5A;
    let mut write_chunk = AlignedBuffer::new(CHUNK, 4096);
    let mut verify_block = AlignedBuffer::new(BLOCK, 4096);
    let mut expected_block = AlignedBuffer::new(BLOCK, 4096);
    let mut total_written = 0u64;
    let mut next_progress = PROGRESS_INTERVAL;
    let start = Instant::now();

    let _ = tx.send(EngineUpdate::status(
        request.session_id,
        request.target.id.clone(),
        SessionStatus::Running,
        "capacity brute-force running",
    ));

    while total_written < target_bytes {
        if request.is_cancelled() {
            let mut cancelled = EngineUpdate::status(
                request.session_id,
                request.target.id,
                SessionStatus::Cancelled,
                "capacity brute-force cancelled",
            );
            cancelled.total_written = total_written;
            cancelled.target_bytes = target_bytes;
            let _ = tx.send(cancelled);
            return Ok(());
        }

        let remaining = target_bytes.saturating_sub(total_written) as usize;
        let write_len = remaining.min(CHUNK);
        let aligned_write_len = write_len - (write_len % BLOCK);
        let block_start_index = total_written / BLOCK as u64;
        fill_chunk(
            &mut write_chunk.as_mut_slice()[..aligned_write_len],
            block_start_index,
            seed,
        );
        file.write_all(&write_chunk.as_slice()[..aligned_write_len])?;
        total_written += aligned_write_len as u64;

        if total_written >= next_progress || total_written == target_bytes {
            send_capacity_progress(
                &tx,
                &request,
                total_written,
                0,
                target_bytes,
                start,
                "bruteforce write phase",
            );
            next_progress = next_progress.saturating_add(PROGRESS_INTERVAL);
        }
    }

    let mut verified_bytes = 0u64;
    file.seek(SeekFrom::Start(0))?;
    while verified_bytes < target_bytes {
        if request.is_cancelled() {
            let mut cancelled = EngineUpdate::status(
                request.session_id,
                request.target.id,
                SessionStatus::Cancelled,
                "capacity brute-force cancelled in verify phase",
            );
            cancelled.total_written = total_written;
            cancelled.verified_bytes = verified_bytes;
            cancelled.usable_bytes = verified_bytes;
            cancelled.target_bytes = target_bytes;
            let _ = tx.send(cancelled);
            return Ok(());
        }

        let block_index = verified_bytes / BLOCK as u64;
        file.read_exact(verify_block.as_mut_slice())?;
        fill_block(expected_block.as_mut_slice(), block_index, seed);
        if verify_block.as_slice() != expected_block.as_slice()
            || verify_block.as_slice().iter().all(|b| *b == 0)
        {
            let mut fail = EngineUpdate::status(
                request.session_id,
                request.target.id,
                SessionStatus::Failed,
                "bruteforce verify mismatch: counterfeit or wrap-around detected",
            );
            fail.total_written = total_written;
            fail.verified_bytes = verified_bytes;
            fail.usable_bytes = verified_bytes;
            fail.target_bytes = target_bytes;
            let _ = tx.send(fail);
            return Ok(());
        }
        verified_bytes += BLOCK as u64;

        if verified_bytes % PROGRESS_INTERVAL == 0 || verified_bytes == target_bytes {
            send_capacity_progress(
                &tx,
                &request,
                total_written,
                verified_bytes,
                target_bytes,
                start,
                "bruteforce verify phase",
            );
        }
    }

    let mut done = EngineUpdate::status(
        request.session_id,
        request.target.id,
        SessionStatus::Completed,
        "capacity brute-force completed",
    );
    done.total_written = total_written;
    done.verified_bytes = verified_bytes;
    done.usable_bytes = verified_bytes;
    done.target_bytes = target_bytes;
    let _ = tx.send(done);
    Ok(())
}

fn send_capacity_progress(
    tx: &Sender<EngineUpdate>,
    request: &StartRequest,
    total_written: u64,
    verified_bytes: u64,
    target_bytes: u64,
    start: Instant,
    phase: &str,
) {
    let elapsed = start.elapsed().as_secs_f64().max(0.001);
    let mbps = (total_written as f64 / (1024.0 * 1024.0)) / elapsed;
    let _ = tx.send(EngineUpdate {
        session_id: request.session_id,
        target_id: request.target.id.clone(),
        status: SessionStatus::Running,
        current_mbps: mbps,
        latency_ms: 0.0,
        total_written,
        verified_bytes,
        usable_bytes: verified_bytes,
        target_bytes,
        message: format!(
            "{phase}: wrote {:.2} GiB verified {:.2} GiB",
            total_written as f64 / ONE_GB as f64,
            verified_bytes as f64 / ONE_GB as f64
        ),
    });
}

fn emit_counterfeit_failure(
    tx: &Sender<EngineUpdate>,
    session_id: u64,
    target_id: String,
    total_written: u64,
    verified_bytes: u64,
    target_bytes: u64,
    reason: &str,
) -> std::io::Result<()> {
    let mut fail = EngineUpdate::status(
        session_id,
        target_id,
        SessionStatus::Failed,
        format!("Hardware Wrap-Around / Counterfeit Detected: {reason}"),
    );
    fail.total_written = total_written;
    fail.verified_bytes = verified_bytes;
    fail.usable_bytes = verified_bytes;
    fail.target_bytes = target_bytes;
    let _ = tx.send(fail);
    Ok(())
}

fn verify_window(file: &mut std::fs::File, start: u64, len: u64, seed_base: u64) -> std::io::Result<bool> {
    let mut observed = AlignedBuffer::new(BLOCK, 4096);
    let mut expected = AlignedBuffer::new(BLOCK, 4096);
    let aligned_len = len - (len % BLOCK as u64);
    file.seek(SeekFrom::Start(start))?;
    let mut offset = 0u64;
    while offset < aligned_len {
        file.read_exact(observed.as_mut_slice())?;
        let block_index = (start + offset) / BLOCK as u64;
        fill_block(expected.as_mut_slice(), block_index, seed_base);
        if observed.as_slice() != expected.as_slice() || observed.as_slice().iter().all(|b| *b == 0)
        {
            return Ok(false);
        }
        offset += BLOCK as u64;
    }
    Ok(true)
}

fn fill_chunk(chunk: &mut [u8], block_start: u64, seed_base: u64) {
    for (i, block) in chunk.chunks_mut(BLOCK).enumerate() {
        fill_block(block, block_start + i as u64, seed_base);
    }
}

fn fill_block(block: &mut [u8], idx: u64, seed_base: u64) {
    let id_tag = idx ^ seed_base;
    let checksum = xxh3_64(&(idx ^ seed_base.rotate_left(11)).to_le_bytes());
    block[..8].copy_from_slice(&id_tag.to_le_bytes());
    block[8..16].copy_from_slice(&checksum.to_le_bytes());

    let mut x = idx
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(seed_base.rotate_left(23))
        ^ 0xD6E8_FEB8_6659_FD93;
    for chunk in block[16..].chunks_mut(8) {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        let value = x.wrapping_mul(0x2545_F491_4F6C_DD1D).to_le_bytes();
        chunk.copy_from_slice(&value[..chunk.len()]);
    }
}

fn align_down_capacity(bytes: u64) -> u64 {
    let aligned = bytes - (bytes % CHUNK as u64);
    aligned.max(CHUNK as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_block_stable() {
        let mut a = [0u8; BLOCK];
        let mut b = [0u8; BLOCK];
        fill_block(&mut a, 123, 777);
        fill_block(&mut b, 123, 777);
        assert_eq!(a, b);
    }
}
