use std::time::{Duration, Instant};

use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};

// use kanidmd_lib::be::dbentry::DbBackup;
use kanidmd_lib::be::BackendTransaction;
use kanidmd_lib::entry::{Entry, EntryInit, EntryNew};
use kanidmd_lib::entry_init;
use kanidmd_lib::prelude::{Attribute, EntryClass};
use kanidmd_lib::server::QueryServerTransaction;
use kanidmd_lib::testkit::{build_the_schema, setup_idm_scaling_test};
use kanidmd_lib::utils::duration_from_epoch_now;
use kanidmd_lib::value::Value;

// so we're doing things consistently
fn get_base_entry_user() -> Entry<EntryInit, EntryNew> {
    entry_init!(
        (Attribute::Class, EntryClass::Object.to_value()),
        (Attribute::Class, EntryClass::Person.to_value()),
        (Attribute::Class, EntryClass::Account.to_value()),
        (Attribute::Description, Value::new_utf8s("criterion"))
    )
}

pub fn scaling_user_create_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("user_create_single");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(120));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("oh no I couldn't make a runtime");
    let backup = rt.block_on(async {
        // build a "proper" versino of the schema/database
        let qs = build_the_schema().await;
        let mut qs_read = qs.read().await;
        let be_txn = qs_read.get_be_txn();
        be_txn.get_backup().unwrap()
    });

    drop(rt);

    let base_user = get_base_entry_user();

    for size in &[
        // 100, 250, 500, 1000, 1500, 2000,
        5000,
        // 10000,
    ] {
        group.throughput(Throughput::Elements(*size));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_custom(|iters| {
                let mut elapsed = Duration::from_secs(0);
                println!("iters, size -> {iters:?}, {size:?}");

                for _i in 0..iters {
                    let mut rt = tokio::runtime::Builder::new_current_thread();
                    elapsed = rt
                        .enable_all()
                        .worker_threads(4)
                        .build()
                        .expect("Failed building the Runtime")
                        .block_on(async {
                            let (idms, _idms_delayed, _idms_audit) =
                                setup_idm_scaling_test(&backup).await;

                            let ct = duration_from_epoch_now();
                            let start = Instant::now();
                            for counter in 0..size {
                                let mut idms_prox_write = idms.proxy_write(ct).await;
                                let name = format!("testperson_{counter}");
                                let mut e1 = base_user.clone();
                                e1.add_ava(Attribute::Name, Value::new_iname(&name));
                                e1.add_ava(Attribute::DisplayName, Value::new_utf8s(&name));

                                let cr = idms_prox_write.qs_write.internal_create(vec![e1]);
                                if let Err(err) = cr {
                                    panic!("Something failed in the create: {:?}", err);
                                }
                                // assert!(cr.is_ok());

                                idms_prox_write.commit().expect("Must not fail");
                            }
                            elapsed.checked_add(start.elapsed()).unwrap()
                        });
                }
                elapsed
            });
        });
    }
    group.finish();
}

pub fn scaling_user_create_batched(c: &mut Criterion) {
    let mut group = c.benchmark_group("user_create_batched");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(120));

    let entry = get_base_entry_user();

    for size in &[
        // 100, 250, 500, 1000, 1500, 2000,
        5000,
        // 10000
    ] {
        group.throughput(Throughput::Elements(*size));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_custom(|iters| {
                let mut elapsed = Duration::from_secs(0);
                println!("iters, size -> {iters:?}, {size:?}");

                let data: Vec<_> = (0..size)
                    .map(|i| {
                        let name = format!("testperson_{i}");
                        let mut entry = entry.clone();
                        entry.add_ava(Attribute::Name, Value::new_iname(&name));
                        entry.add_ava(Attribute::DisplayName, Value::new_utf8s(&name));
                        entry
                    })
                    .collect();

                for _i in 0..iters {
                    let mut rt = tokio::runtime::Builder::new_current_thread();
                    elapsed = rt
                        .enable_all()
                        .build()
                        .expect("Failed building the Runtime")
                        .block_on(async {
                            let (idms, _idms_delayed, _idms_audit) =
                                kanidmd_lib::testkit::setup_idm_test().await;

                            let ct = duration_from_epoch_now();
                            let start = Instant::now();

                            let mut idms_prox_write = idms.proxy_write(ct).await;
                            let cr = idms_prox_write.qs_write.internal_create(data.clone());
                            assert!(cr.is_ok());

                            idms_prox_write.commit().expect("Must not fail");
                            elapsed.checked_add(start.elapsed()).unwrap()
                        });
                }
                elapsed
            });
        });
    }
    group.finish();
}

criterion_group!(
    name = scaling_basic;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(15))
        .with_plots();
    targets = scaling_user_create_single, scaling_user_create_batched
);
criterion_main!(scaling_basic);
