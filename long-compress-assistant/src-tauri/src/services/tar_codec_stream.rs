//! A bounded TAR-to-codec bridge. The consumer keeps the existing pull encoder
//! API (including fallible finalization); no complete TAR is staged on disk.
use super::{CompressionRuntime, TarPreparation};
use crate::services::{compression_entries::CompressionEntry, native_compression::tar};
use anyhow::Result;
use std::io::{self, BufWriter, Cursor, Read, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::time::Duration;
use tauri::Window;

const CHUNK_BYTES: usize = 64 * 1024;
const QUEUED_CHUNKS: usize = 2;

struct PipeWriter(SyncSender<Vec<u8>>);
impl Write for PipeWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() { return Ok(0); }
        let size = bytes.len().min(CHUNK_BYTES);
        self.0.send(bytes[..size].to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "TAR 编码读取已停止"))?;
        Ok(size)
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

pub(super) struct PipeReader<'a, R> {
    runtime: &'a R,
    receiver: Receiver<Vec<u8>>,
    current: Cursor<Vec<u8>>,
}
impl<R: CompressionRuntime> Read for PipeReader<'_, R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        if bytes.is_empty() { return Ok(0); }
        loop {
            self.runtime.check_cancellation().map_err(|error| io::Error::other(error.to_string()))?;
            let read = self.current.read(bytes)?;
            if read > 0 { return Ok(read); }
            match self.receiver.recv_timeout(Duration::from_millis(25)) {
                Ok(chunk) => self.current = Cursor::new(chunk),
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => return Ok(0),
            }
        }
    }
}

pub(super) fn with_reader<R: CompressionRuntime + Sync, T>(
    runtime: &R,
    window: Option<&Window>,
    task_id: &str,
    entries: Vec<CompressionEntry>,
    utf8_pax_paths: bool,
    encode: impl FnOnce(&mut PipeReader<'_, R>) -> Result<T>,
) -> Result<T> {
    let (sender, receiver) = mpsc::sync_channel(QUEUED_CHUNKS);
    std::thread::scope(|scope| {
        let producer = scope.spawn(move || -> Result<()> {
            let writer = BufWriter::with_capacity(CHUNK_BYTES, PipeWriter(sender));
            let mut builder = ::tar::Builder::new(writer);
            // Source bytes are real progress, but leave room for codec finish
            // and the service's existing verification/publication stages.
            tar::write_collected_entries_with_pax(&TarPreparation(runtime, 0.95), window, task_id, &entries, &mut builder, utf8_pax_paths)?;
            builder.into_inner()?.flush()?;
            Ok(())
        });
        let encoded = {
            let mut reader = PipeReader { runtime, receiver, current: Cursor::new(Vec::new()) };
            encode(&mut reader)
        }; // Drop the receiver before joining: a failed consumer unblocks send.
        let produced = producer.join().map_err(|_| anyhow::anyhow!("TAR 生成线程异常终止"));
        match encoded {
            Ok(output) => { produced??; Ok(output) }
            Err(error) => {
                // Cancellation remains cancellation, not a downstream EOF or
                // broken pipe. Other encoder/storage errors retain their cause.
                runtime.check_cancellation()?;
                Err(error)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pipe_writer_never_queues_an_unbounded_caller_buffer() {
        let (sender, receiver) = mpsc::sync_channel(QUEUED_CHUNKS);
        let mut writer = PipeWriter(sender);
        assert_eq!(writer.write(&vec![42; CHUNK_BYTES * 8]).unwrap(), CHUNK_BYTES);
        assert_eq!(receiver.recv().unwrap().len(), CHUNK_BYTES);
        drop(receiver);
        assert_eq!(writer.write(b"stopped").unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    }

    fn source_entries(root: &std::path::Path) -> Vec<CompressionEntry> {
        let source = root.join("payload.bin");
        std::fs::write(&source, vec![42; CHUNK_BYTES * 8]).unwrap();
        crate::services::compression_entries::collect(&[source.to_string_lossy().into_owned()], true, true).unwrap()
    }

    #[test]
    fn producer_error_is_not_hidden_by_successful_codec_eof() {
        let temp = tempfile::tempdir().unwrap();
        let entries = source_entries(temp.path());
        std::fs::remove_file(&entries[0].path).unwrap();
        let runtime = crate::services::compression_service::CompressionService::for_testing();
        let error = with_reader(&runtime, None, "source-disappeared", entries, false, |input| {
            Ok(std::io::copy(input, &mut std::io::sink())?)
        }).unwrap_err();
        assert!(error.chain().any(|cause| cause.downcast_ref::<io::Error>()
            .is_some_and(|cause| cause.kind() == io::ErrorKind::NotFound)), "{error:?}");
    }

    #[test]
    fn consumer_failure_unblocks_the_bounded_producer_and_keeps_storage_error() {
        let temp = tempfile::tempdir().unwrap();
        let entries = source_entries(temp.path());
        let runtime = crate::services::compression_service::CompressionService::for_testing();
        let error = with_reader(&runtime, None, "storage-full", entries, false, |_input| {
            Err::<(), _>(io::Error::new(io::ErrorKind::StorageFull, "simulated destination full").into())
        }).unwrap_err();
        assert_eq!(error.downcast_ref::<io::Error>().unwrap().kind(), io::ErrorKind::StorageFull);
    }

    #[test]
    fn cancellation_drops_the_receiver_before_joining_the_producer() {
        let temp = tempfile::tempdir().unwrap();
        let entries = source_entries(temp.path());
        let runtime = crate::services::compression_service::CompressionService::for_testing();
        let error = with_reader(&runtime, None, "cancel", entries, false, |input| {
            input.read_exact(&mut [0u8; 1])?;
            runtime.cancel();
            Ok(std::io::copy(input, &mut std::io::sink())?)
        }).unwrap_err();
        assert!(matches!(error.downcast_ref::<crate::services::compression_service::CompressionError>(),
            Some(crate::services::compression_service::CompressionError::Cancelled)), "{error:?}");
    }
}
