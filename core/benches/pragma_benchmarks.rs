use std::rc::Rc;
use std::sync::Arc;
use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use limbo_core::{Connection, Database, PlatformIO, UnixIO, IO};

fn bench_execute_select_count(criterion: &mut Criterion) {

    #[allow(clippy::arc_with_non_send_sync)]
    let io = Arc::new(PlatformIO::new().unwrap());
    let db = Database::open_file(io.clone(), "../database.db", false).unwrap();
    let limbo_conn = db.connect().unwrap();
    limbo_conn.load_extension("../target/release/liblimbo_pragma.so").unwrap();

    let mut create_sql = String::new();
    create_sql.push_str("CREATE TABLE IF NOT EXISTS big (a0 TEXT");
    for i in 1..1000 {
        create_sql.push_str(format!(", a{} TEXT", i).as_str());
    }
    create_sql.push_str(")");
    limbo_conn.execute(create_sql).unwrap();

    let mut group = criterion.benchmark_group("pragma_vtab");
    group.bench_function("pragma_table_info", |b| {
        exec("SELECT * FROM pragma_table_info('big')", &io, &limbo_conn, b);
    });

    group.bench_function("pragma_table_info_ext", |b| {
        exec("SELECT * FROM pragma_table_info_ext('big')", &io, &limbo_conn, b);
    });

    group.finish();
}

fn exec(sql: &str, io: &Arc<UnixIO>, limbo_conn: &Rc<Connection>, b: &mut Bencher) {
    let mut stmt = limbo_conn.prepare(sql).unwrap();
    let io = io.clone();
    b.iter(|| {
        loop {
            match stmt.step().unwrap() {
                limbo_core::StepResult::Row => {
                    black_box(stmt.row());
                }
                limbo_core::StepResult::IO => {
                    let _ = io.run_once();
                }
                limbo_core::StepResult::Done => {
                    break;
                }
                limbo_core::StepResult::Interrupt | limbo_core::StepResult::Busy => {
                    unreachable!();
                }
            }
        }
        stmt.reset();
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_execute_select_count
}
criterion_main!(benches);
