use std::io::{Seek, SeekFrom, Write};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::engine::messages::{EngineUpdate, SessionStatus, StartRequest, TestKind};
use crate::io::alignment::AlignedBuffer;
use crate::io::unbuffered::open_unbuffered_file;
use windows_sys::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL,
};

pub fn run_benchmark(request: StartRequest, tx: Sender<EngineUpdate>) -> std::io::Result<()> {
    unsafe {
        SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }

    let path = request
        .output_file
        .clone()
        .unwrap_or_else(|| format!("{}\\test_data.bin", request.target.drive_letter).into());
    let mut file = open_unbuffered_file(&path)?;
    let start = Instant::now();
    let mut total_written = 0u64;

    let block_size = if request.test_kind == TestKind::ActionCam {
        crate::workloads::action_cam::BLOCK_SIZE_BYTES
    } else {
        crate::workloads::os_drive::BLOCK_SIZE_BYTES
    };
    let mut buffer = AlignedBuffer::new(block_size, 4096);
    buffer.fill_pattern(0xA5);

    let mut sample = 0u64;
    let limit = request.duration_seconds.max(5);
    let random_max = 256 * 1024 * 1024u64;
    let random_blocks = (random_max / block_size as u64).max(1);
    let mut last_ui_emit = Instant::now();
    let mut peak_mbps = 0.0f64;
    let mut peak_latency_ms = 0.0f64;
    let mut latency_total_ms = 0.0f64;
    let mut latency_samples = 0u64;

    if request.test_kind == TestKind::OsDrive {
        let _ = file.set_len(random_max);
    }

    let _ = tx.send(EngineUpdate::status(
        request.session_id,
        request.target.id.clone(),
        SessionStatus::Running,
        format!("running {:?} workload", request.test_kind),
    ));

    while start.elapsed().as_secs() < limit {
        if request.is_cancelled() {
            let _ = tx.send(EngineUpdate::status(
                request.session_id,
                request.target.id,
                SessionStatus::Cancelled,
                "benchmark cancelled",
            ));
            return Ok(());
        }

        if request.test_kind == TestKind::OsDrive {
            let random_block = rand::random::<u64>() % random_blocks;
            let offset = random_block * block_size as u64;
            file.seek(SeekFrom::Start(offset))?;
        }

        let block_start = Instant::now();
        file.write_all(buffer.as_slice())?;
        let latency_ms = block_start.elapsed().as_secs_f64() * 1000.0;
        total_written += block_size as u64;
        sample += 1;
        latency_total_ms += latency_ms;
        latency_samples += 1;

        let elapsed = start.elapsed().as_secs_f64().max(0.001);
        let mbps = (total_written as f64 / (1024.0 * 1024.0)) / elapsed;
        peak_mbps = peak_mbps.max(mbps);
        peak_latency_ms = peak_latency_ms.max(latency_ms);

        if last_ui_emit.elapsed() >= Duration::from_millis(120) {
            let _ = tx.send(EngineUpdate {
                session_id: request.session_id,
                target_id: request.target.id.clone(),
                status: SessionStatus::Running,
                current_mbps: mbps,
                latency_ms,
                total_written,
                verified_bytes: 0,
                usable_bytes: 0,
                target_bytes: request.target.advertised_bytes.unwrap_or_default(),
                message: format!("sample {sample}, {mbps:.1} MB/s"),
            });
            last_ui_emit = Instant::now();
        }
    }

    let avg_latency = if latency_samples == 0 {
        0.0
    } else {
        latency_total_ms / latency_samples as f64
    };
    let _ = tx.send(EngineUpdate {
        session_id: request.session_id,
        target_id: request.target.id,
        status: SessionStatus::Completed,
        current_mbps: peak_mbps,
        latency_ms: peak_latency_ms,
        total_written,
        verified_bytes: 0,
        usable_bytes: 0,
        target_bytes: 0,
        message: format!(
            "benchmark completed | peak {:.1} MB/s | peak latency {:.2} ms | avg latency {:.2} ms",
            peak_mbps, peak_latency_ms, avg_latency
        ),
    });

    Ok(())
}
