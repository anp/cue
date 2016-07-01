//! Run a computation in parallel, and register a "joiner" closure to aggregate/serialize the
//! results. Example:
//!
//! ```
//! use std::collections::BTreeMap;
//! use cue::pipeline;
//!
//! let mut results = BTreeMap::new();
//!
//! pipeline("test123",      // name of the pipeline for logging
//!          4,              // number of worker threads
//!          (0..100_000),   // iterator with work items
//!          |n| (n, n * 5), // computation to apply in parallel to work items
//!          |r| {           // aggregation to apply to work results
//!              results.insert(r.0, r.1);
//!          }
//! );
//!
//! for i in 0..100 {
//!     assert!(Some(&(i * 5)) == results.get(&i));
//! }
//! ```

#[cfg(feature="log")]
#[macro_use]
extern crate log;

extern crate crossbeam;
extern crate syncbox;

use crossbeam::scope;
use crossbeam::sync::MsQueue;
use syncbox::LinkedQueue;

enum WorkItem<T> {
    Available(T),
    PoisonPill,
}

enum WorkResult<T> {
    Available(T),
    WorkerTerminated,
}

pub fn pipeline<Q, R, QF, JF, W>(name: &str,
                                 num_workers: usize,
                                 work: W,
                                 worker: QF,
                                 mut joiner: JF)
    where Q: Send + Sized,
          R: Send + Sized,
          QF: Fn(Q) -> R + Sync,
          JF: FnMut(R) + Send + Sync,
          W: Iterator<Item = Q>
{
    let results = MsQueue::<WorkResult<R>>::new();
    let queries = LinkedQueue::<WorkItem<Q>>::with_capacity(num_workers * 20);

    scope(|scope| {
        // results consumer
        scope.spawn(|| {
            let mut num_ended = 0;
            let mut num_processed = 0;

            while num_ended < num_workers {

                match results.pop() {

                    WorkResult::Available(result) => {
                        joiner(result);

                        num_processed += 1;
                        log(name, num_processed);
                    }

                    WorkResult::WorkerTerminated => num_ended += 1,
                }
            }
        });

        // workers
        for _ in 0..num_workers {
            scope.spawn(|| {

                // note that this blocks if the buffer is empty
                while let WorkItem::Available(query) = queries.take() {

                    let result = worker(query);
                    results.push(WorkResult::Available(result));
                }

                results.push(WorkResult::WorkerTerminated);
            });
        }

        // put work on the queue from the iterator
        for query in work {
            // note that this blocks if the buffer is full
            queries.put(WorkItem::Available(query));
        }

        // tell all the workers there's no more work left
        for _ in 0..num_workers {
            queries.put(WorkItem::PoisonPill);
        }
    });
}

#[cfg(feature="log")]
fn log(name: &str, num_done: usize) {
    if num_done % 10_000 == 0 {
        debug!("{} pipeline has processed {} work items.", name, num_done);
    }
}

#[cfg(not(feature="log"))]
fn log(_: &str, _: usize) {}

#[cfg(test)]
mod tests {
    #[test]
    fn basic_test() {
        use std::collections::BTreeMap;
        use super::pipeline;

        let mut results = BTreeMap::new();

        pipeline("test123", 4, (0..100_000), |n| (n, n * 5), |r| {
            results.insert(r.0, r.1);
        });

        for i in 0..100 {
            assert!(Some(&(i * 5)) == results.get(&i));
        }
    }
}
