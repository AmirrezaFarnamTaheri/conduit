use std::{sync::{mpsc, Arc}, thread};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report { pub index: usize, pub value: String }
pub fn execute<T, F>(tasks: Vec<T>, workers: usize, operation: F) -> Vec<Report>
where T: Send + 'static, F: Fn(T) -> String + Send + Sync + 'static {
    if tasks.is_empty() { return Vec::new(); }
    let worker_count = workers.max(1).min(tasks.len());
    let (input_tx, input_rx) = mpsc::channel::<(usize, T)>();
    let (output_tx, output_rx) = mpsc::channel::<Report>();
    let input_rx = Arc::new(std::sync::Mutex::new(input_rx)); let operation = Arc::new(operation);
    let mut threads = Vec::new();
    for _ in 0..worker_count {
        let input_rx = Arc::clone(&input_rx); let output_tx = output_tx.clone(); let operation = Arc::clone(&operation);
        threads.push(thread::spawn(move || loop {
            let task = input_rx.lock().unwrap_or_else(|poison| poison.into_inner()).recv();
            let Ok((index, value)) = task else { break; };
            let value = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(value))).unwrap_or_else(|_| "worker panicked".to_owned());
            if output_tx.send(Report { index, value }).is_err() { break; }
        }));
    }
    for (index, task) in tasks.into_iter().enumerate() { if input_tx.send((index, task)).is_err() { break; } }
    drop(input_tx); drop(output_tx);
    for thread in threads { let _ = thread.join(); }
    let mut reports: Vec<_> = output_rx.into_iter().collect(); reports.sort_by_key(|report| report.index); reports
}
#[cfg(test)]
mod tests { use super::*; #[test] fn results_are_ordered_despite_parallel_execution() { let results = execute(vec![3, 1, 2], 3, |value| value.to_string()); assert_eq!(results.iter().map(|report| report.value.as_str()).collect::<Vec<_>>(), vec!["3", "1", "2"]); } #[test] fn zero_workers_is_safe() { assert_eq!(execute(vec![1, 2], 0, |value| value.to_string()).len(), 2); } }
