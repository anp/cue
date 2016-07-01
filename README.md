# cue

[![Build Status](https://img.shields.io/travis/dikaiosune/cue/master.svg?style=flat-square)](https://travis-ci.org/dikaiosune/cue) [![crates.io](https://img.shields.io/crates/v/cue.svg?style=flat-square)](https://crates.io/crates/cue/) [![API Docs](https://img.shields.io/badge/API-docs-blue.svg?style=flat-square)](https://dikaiosune.github.io/cue) [![License](https://img.shields.io/badge/license-MIT-lightgray.svg?style=flat-square)](https://github.com/dikaiosune/cue/blob/master/LICENSE)

cue is currently a very basic library for providing a "streaming" parallel pipeline for long-running tasks which need to control memory usage. It's specifically intended for scenarios where:

* An expensive computation needs to be run in parallel on many inputs.
* Many worker threads are desired (for example, I commonly use this on 32 CPU machines).
* The number of worker inputs in memory must be limited.
* The results of the computation need to be aggregated as they're produced, instead of when the worker threads join.
* Aggregating the results has some overhead and may occasionally block several workers if it's handled in the computation threads (i.e. writing to a file, network socket, etc.).
* Accumulating all of the results in memory is not practical (either due to the size of individual result values, the number of items to process, or both).

In writing some long-running CLI tools, I found myself using a pattern for this frequently enough that I put it in a library.

## Usage

Here's a basic usage example. This will:

1. Spin up a threadpool of scoped threads (no need for `Arc<T>`).
2. Submit all of the items from the iterator to the worker pool, blocking on submissions which would overfill the work queue's buffer.
3. In parallel, remove each work item from the queue, process it with the worker closure, and submit it to the "results" queue.
4. The result closure will be applied to every item in the results queue, serializing/joining the results.
5. Logging: the `debug!` log macro will be invoked every 10,000 work items that are processed (this can be disabled -- see `Cargo.toml` for features).

```rust
extern crate cue;

fn main() {
    cue::pipeline("demo", // naming the pipeline allows for better logging if multiple are running

                  // number of worker threads needed, result thread will be spun up in addition
                  8,

                  // an iterator which yields items of the desired work type -- should be lazy
                  // otherwise it doesn't make much sense to use a bounded work queue
                  create_lazy_iterator_with_lots_of_items(),

                  // item must match the Item type of the iterator above
                  |item| { do_super_duper_expensive_task_which_returns_result(item) },

                  // r here must match the return type of the worker closure
                  |r| { write_result_to_disk_which_may_take_a_while(r); });

    println!("Done! The work has been processed in parallel.");
}
```

For an example, see the test in `src/lib.rs`. For documentation, see the currently somewhat sparse [API docs](https://dikaiosune.github.io/cue).

## License

MIT, see `LICENSE`.
