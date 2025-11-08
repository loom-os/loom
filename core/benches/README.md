# EventBus Performance Benchmarks

This directory contains the EventBus performance benchmark suite written using [Criterion.rs](https://github.com/bheisler/criterion.rs).

## 📊 Benchmark Overview

### Test Scenarios

| Benchmark Group                  | Description                          | Test Parameters                    |
| -------------------------------- | ------------------------------------ | ---------------------------------- |
| `eventbus_single_publisher`      | Single publisher-consumer throughput | 100/1K/10K events                  |
| `eventbus_concurrent_publishers` | Concurrent publishers throughput     | 2/4/8 publishers × 500 events      |
| `eventbus_qos_levels`            | QoS level performance comparison     | Realtime/Batched/Background        |
| `eventbus_publish_latency`       | Single event publish latency         | Single event publication           |
| `eventbus_multiple_subscribers`  | Multiple subscribers scenario        | 2/5/10 subscribers × 500 events    |
| `eventbus_event_filtering`       | Event filtering overhead             | With filter vs without (1K events) |

## 🚀 Running Benchmarks

### Quick Test (Recommended)

```bash
# Run quick benchmark (reduced iterations and sampling time)
cargo bench --bench event_bus_benchmark -- --quick
```

### Full Test

```bash
# Run full benchmark (with complete statistical analysis)
cargo bench --bench event_bus_benchmark
```

### Run Specific Benchmarks

```bash
# Test single publisher scenario only
cargo bench --bench event_bus_benchmark -- single_publisher

# Test concurrent publishers only
cargo bench --bench event_bus_benchmark -- concurrent_publishers

# Test QoS levels only
cargo bench --bench event_bus_benchmark -- qos_levels
```

### Benchmark Options

```bash
# Save baseline for future comparison
cargo bench --bench event_bus_benchmark -- --save-baseline my-baseline

# Compare with previous baseline
cargo bench --bench event_bus_benchmark -- --baseline my-baseline

# Specify warm-up and measurement time
cargo bench --bench event_bus_benchmark -- --warm-up-time 5 --measurement-time 10
```

## 📈 Performance Baseline Data

Test results based on Apple M1 MacBook Air (2025-11-08):

### Single Publisher Throughput

```
100 events:    272-279 Kelem/s  (~3.6 ms)
1,000 events:  568-579 Kelem/s  (~1.7 ms)
10,000 events: 632-666 Kelem/s  (~15 ms)
```

### Concurrent Publishers Throughput

```
2 publishers × 500:  940-947 Kelem/s   (~1.06 ms)
4 publishers × 500:  1.09-1.15 Melem/s (~1.76 ms)
8 publishers × 500:  918-954 Kelem/s   (~4.3 ms)
```

### QoS Level Performance

```
Realtime:   12-495 Kelem/s (high variance due to drop strategy)
Batched:    [stable batched performance]
Background: [maximum buffer, most stable]
```

### Key Performance Metrics

- **Peak Throughput**: 1.15 Melem/s (4 concurrent publishers)
- **Stable Throughput**: 600-700 Kelem/s (single publisher)
- **Concurrency Scaling**: Optimal at 4 publishers

## 🏗️ Benchmark Architecture

### File Structure

```
benches/
├── README.md                    # This file
└── event_bus_benchmark.rs       # EventBus benchmark suite
```

### Code Structure

```rust
// Helper functions
fn make_event(id: u64, event_type: &str) -> Event

// Benchmark functions
fn bench_single_publisher(c: &mut Criterion)
fn bench_concurrent_publishers(c: &mut Criterion)
fn bench_qos_levels(c: &mut Criterion)
fn bench_publish_latency(c: &mut Criterion)
fn bench_multiple_subscribers(c: &mut Criterion)
fn bench_event_filtering(c: &mut Criterion)

// Criterion configuration
criterion_group!(benches, ...);
criterion_main!(benches);
```

## 📝 Benchmark Design

### Testing Methodology

1. **Runtime Management**: Create new Tokio runtime per iteration to avoid state pollution
2. **Throughput Measurement**: Use `Throughput::Elements` to mark element count
3. **Statistical Analysis**: Criterion automatically performs multiple sampling and analysis
4. **Result Visualization**: Auto-generates HTML reports (requires gnuplot or uses plotters backend)

### Async Benchmark Pattern

```rust
b.iter(|| {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Async test code
        let bus = EventBus::new().await.unwrap();
        // ...
    })
});
```

### QoS Level Testing

- **Realtime**: 64 buffer size, tests low-latency scenarios
- **Batched**: 1024 buffer size, tests balanced scenarios
- **Background**: 4096 buffer size, tests high-throughput scenarios

## 📊 Reports and Results

### Viewing Results

```bash
# Benchmark reports are saved at
open target/criterion/report/index.html

# Or view specific test group reports
open target/criterion/eventbus_single_publisher/report/index.html
```

### Result File Locations

```
target/criterion/
├── eventbus_single_publisher/
│   ├── report/
│   │   └── index.html
│   └── base/
│       └── estimates.json
├── eventbus_concurrent_publishers/
├── eventbus_qos_levels/
├── eventbus_publish_latency/
├── eventbus_multiple_subscribers/
└── eventbus_event_filtering/
```

## 🔧 Configuration and Optimization

### Cargo.toml Configuration

```toml
[[bench]]
name = "event_bus_benchmark"
harness = false  # Use Criterion instead of default harness
```

### Performance Optimization Tips

1. **Warm-up Phase**: Criterion automatically warms up to ensure JIT optimization
2. **Measurement Precision**: Short operations automatically increase iteration count
3. **Noise Control**: Close background apps to reduce measurement noise
4. **CPU Frequency**: Run plugged in to avoid battery mode throttling

### Environment Requirements

- Rust 1.70+ (async support)
- Tokio runtime (async executor)
- Criterion 0.5+ (benchmarking framework)
- Optional: gnuplot (chart generation)

## 🎯 Use Cases

### 1. Performance Regression Testing

```bash
# Save current performance baseline
cargo bench --bench event_bus_benchmark -- --save-baseline main

# Compare after code changes
cargo bench --bench event_bus_benchmark -- --baseline main
```

### 2. Performance Optimization Verification

```bash
# Before optimization
cargo bench --bench event_bus_benchmark -- --save-baseline before

# After optimization
cargo bench --bench event_bus_benchmark -- --save-baseline after

# Compare analysis
critcmp before after  # Requires cargo-criterion installation
```

### 3. Capacity Planning

- Estimate system capacity based on throughput data
- Identify performance bottlenecks (CPU/memory/network)
- Develop scaling strategies

## 🐛 Troubleshooting

### Common Issues

**Q: Benchmarks running slowly?**

```bash
# Use --quick option to reduce sampling
cargo bench -- --quick
```

**Q: Results unstable/high variance?**

- Close background applications
- Ensure low system load
- Increase measurement time: `--measurement-time 30`

**Q: Compilation errors?**

```bash
# Ensure dependencies are up to date
cargo update
cargo clean
cargo bench --bench event_bus_benchmark
```

**Q: Gnuplot warnings?**

- Safe to ignore, will automatically use plotters backend
- Or install gnuplot: `brew install gnuplot`

## 📚 Related Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [EventBus Pressure Tests](../tests/pressure/)
- [EventBus Implementation](../src/event.rs)
- [Test Suite Documentation](../tests/README.md)

## 🔄 Continuous Integration

Recommended to run benchmarks regularly in CI/CD:

```yaml
# GitHub Actions example
- name: Run benchmarks
  run: cargo bench --bench event_bus_benchmark -- --quick

- name: Store benchmark result
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: "cargo"
    output-file-path: target/criterion/output.txt
```

## 📖 Further Reading

### Adding New Benchmarks

1. Add new function in `event_bus_benchmark.rs`:

```rust
fn bench_my_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_scenario");
    group.bench_function("test_name", |b| {
        b.iter(|| {
            // Test code
        });
    });
    group.finish();
}
```

2. Add to `criterion_group!`:

```rust
criterion_group!(
    benches,
    bench_single_publisher,
    // ... other tests
    bench_my_scenario,  // New addition
);
```

### Performance Analysis Best Practices

1. **Establish Baseline**: Save performance baseline before major changes
2. **Isolate Variables**: Test only one variable at a time
3. **Multiple Validations**: Important optimizations require multiple verifications
4. **Documentation**: Document performance optimization decisions and results

---

**Maintainers**: Loom Team  
**Last Updated**: 2025-11-08  
**Version**: v0.1.0
