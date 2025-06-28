use futures::Stream;
use std::sync::Arc;
use std::sync::mpsc as smpsc;
use tokio::sync::mpsc as ampsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// This data structure is for running analytics on large data
/// sets with Rayon where the input is large, and the output is
/// small. Pushing data into the thread pool will asynchronously
/// block until the threadpool is ready to accept more data.
///
/// This keeps memory usage down when parallelizing compute
/// operations. However, output is collected in bulk using an
/// unbounded channel to avoid blocking the thread pool
/// unnecessarily. If the output data is large, this is not the
/// right abstraction.
pub struct BackpressuredAsyncRayon<
    F: Fn(D) -> R + Send + Sync + 'static,
    D: Send + 'static,
    R: Send + 'static,
> {
    process_func: Arc<F>,
    in_send: ampsc::Sender<D>,
    in_recv: Arc<std::sync::Mutex<ampsc::Receiver<D>>>,
    out_send: ampsc::UnboundedSender<R>,
    out_recv: ampsc::UnboundedReceiver<R>,
}
impl<F: Fn(D) -> R + Send + Sync + 'static, D: Send + 'static, R: Send + 'static>
    BackpressuredAsyncRayon<F, D, R>
{
    pub fn new(backfill_size: usize, process_func: F) -> Self {
        let (in_send, in_recv) = ampsc::channel(backfill_size);
        let (out_send, out_recv) = ampsc::unbounded_channel();
        return Self {
            process_func: Arc::new(process_func),
            in_send,
            in_recv: Arc::new(std::sync::Mutex::new(in_recv)),
            out_send,
            out_recv,
        };
    }
    pub async fn push_data(&self, data: D) {
        self.in_send
            .send(data)
            .await
            .expect("Unreachable: we own a receiver");
        let receiver = Arc::clone(&self.in_recv);
        let result_sender = self.out_send.clone();
        let process_func = Arc::clone(&self.process_func);
        rayon::spawn_fifo(move || {
            let mut receiver = receiver.lock().unwrap();
            let data = receiver.blocking_recv();
            drop(receiver);
            if let Some(data) = data {
                let result = process_func(data);
                let _ = result_sender.send(result);
            }
        });
    }
    pub fn to_stream(self) -> impl Stream<Item = R> + Send + Unpin + 'static {
        drop(self.in_send);
        drop(self.in_recv);
        drop(self.out_send);
        return UnboundedReceiverStream::new(self.out_recv);
    }
    pub async fn collect(mut self) -> Vec<R> {
        drop(self.in_send);
        drop(self.in_recv);
        drop(self.out_send);
        let mut results = Vec::new();
        while self.out_recv.recv_many(&mut results, usize::MAX).await > 0 {}
        return results;
    }
    pub async fn try_collect(mut self) -> Result<Vec<<R as Try>::Success>, <R as Try>::Error>
    where
        R: Try,
    {
        drop(self.in_send);
        drop(self.in_recv);
        drop(self.out_send);
        let mut results = Vec::new();
        while let Some(result) = self.out_recv.recv().await {
            results.push(result.as_result()?);
        }
        return Ok(results);
    }
}

pub struct ReceiverIterator<T>(smpsc::Receiver<T>);
impl<T> Iterator for ReceiverIterator<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        return self.0.recv().ok();
    }
}

/// This data structure is for running analytics on large data
/// sets with Rayon where the input is large, and the output is
/// small. Pushing data into the thread pool will asynchronously
/// block until the threadpool is ready to accept more data.
///
/// This keeps memory usage down when parallelizing compute
/// operations. However, output is collected in bulk using an
/// unbounded channel to avoid blocking the thread pool
/// unnecessarily. If the output data is large, this is not the
/// right abstraction.
pub struct BackpressuredRayon<
    F: Fn(D) -> R + Send + Sync + 'static,
    D: Send + 'static,
    R: Send + 'static,
> {
    process_func: Arc<F>,
    in_send: smpsc::SyncSender<D>,
    in_recv: Arc<std::sync::Mutex<smpsc::Receiver<D>>>,
    out_send: smpsc::Sender<R>,
    out_recv: smpsc::Receiver<R>,
}
impl<F: Fn(D) -> R + Send + Sync + 'static, D: Send + 'static, R: Send + 'static>
    BackpressuredRayon<F, D, R>
{
    pub fn new(backfill_size: usize, process_func: F) -> Self {
        let (in_send, in_recv) = smpsc::sync_channel(backfill_size);
        let (out_send, out_recv) = smpsc::channel();
        return Self {
            process_func: Arc::new(process_func),
            in_send,
            in_recv: Arc::new(std::sync::Mutex::new(in_recv)),
            out_send,
            out_recv,
        };
    }
    pub fn push_data(&self, data: D) {
        self.in_send
            .send(data)
            .expect("Unreachable: we own a receiver");
        let receiver = Arc::clone(&self.in_recv);
        let result_sender = self.out_send.clone();
        let process_func = Arc::clone(&self.process_func);
        rayon::spawn_fifo(move || {
            let receiver = receiver.lock().unwrap();
            let data = receiver.recv();
            drop(receiver);
            if let Ok(data) = data {
                let result = process_func(data);
                let _ = result_sender.send(result);
            }
        });
    }
    pub fn to_receiver(self) -> impl Iterator<Item = R> + Send + 'static {
        drop(self.in_send);
        drop(self.in_recv);
        drop(self.out_send);
        return ReceiverIterator(self.out_recv);
    }
    pub fn collect(self) -> Vec<R> {
        drop(self.in_send);
        drop(self.in_recv);
        drop(self.out_send);
        let mut results = Vec::new();
        while let Ok(result) = self.out_recv.recv() {
            results.push(result);
        }
        return results;
    }
    pub fn try_collect(self) -> Result<Vec<<R as Try>::Success>, <R as Try>::Error>
    where
        R: Try,
    {
        drop(self.in_send);
        drop(self.in_recv);
        drop(self.out_send);
        let mut results = Vec::new();
        while let Ok(result) = self.out_recv.recv() {
            results.push(result.as_result()?);
        }
        return Ok(results);
    }
}

/// Simple trait wrapper for results
pub trait Try {
    type Success;
    type Error;
    fn as_result(self) -> Result<Self::Success, Self::Error>;
}
impl<S, E> Try for Result<S, E> {
    type Success = S;
    type Error = E;
    #[inline(always)]
    fn as_result(self) -> Result<Self::Success, Self::Error> {
        return self;
    }
}
