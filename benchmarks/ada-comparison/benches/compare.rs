use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

const CORPUS: &str = include_str!("../top100.txt");

/// URLs that BOTH parsers accept, so we time identical work on both sides.
fn corpus() -> Vec<String> {
    let mut agreed = Vec::new();
    let (mut only_rust, mut only_ada, mut neither) = (0usize, 0usize, 0usize);
    for line in CORPUS.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let r = line.parse::<url::Url>().is_ok();
        let a = ada_url::Url::parse(line, None).is_ok();
        match (r, a) {
            (true, true) => agreed.push(line.to_owned()),
            (true, false) => only_rust += 1,
            (false, true) => only_ada += 1,
            (false, false) => neither += 1,
        }
    }
    eprintln!(
        "corpus: {} both-ok, {} rust-only, {} ada-only, {} neither",
        agreed.len(),
        only_rust,
        only_ada,
        neither
    );
    agreed
}

fn bench(c: &mut Criterion) {
    let urls = corpus();
    let bytes: usize = urls.iter().map(|u| u.len()).sum();

    // 1. Bulk parse of the full real-world corpus.
    let mut g = c.benchmark_group("parse_corpus");
    g.throughput(Throughput::Bytes(bytes as u64));
    g.sample_size(20);
    g.bench_function("rust-url", |b| {
        b.iter(|| {
            for u in &urls {
                black_box(black_box(u.as_str()).parse::<url::Url>().unwrap());
            }
        })
    });
    g.bench_function("ada", |b| {
        b.iter(|| {
            for u in &urls {
                black_box(ada_url::Url::parse(black_box(u.as_str()), None).unwrap());
            }
        })
    });
    g.finish();

    // 2. Parse + read back components (what a real caller does).
    let mut g = c.benchmark_group("parse_and_getters");
    g.throughput(Throughput::Bytes(bytes as u64));
    g.sample_size(20);
    g.bench_function("rust-url", |b| {
        b.iter(|| {
            for u in &urls {
                let p = u.as_str().parse::<url::Url>().unwrap();
                black_box(p.host_str());
                black_box(p.path());
                black_box(p.query());
            }
        })
    });
    g.bench_function("ada", |b| {
        b.iter(|| {
            for u in &urls {
                let p = ada_url::Url::parse(u.as_str(), None).unwrap();
                black_box(p.host());
                black_box(p.pathname());
                black_box(p.search());
            }
        })
    });
    g.finish();

    // 3. Single short URL, the shape most Deno hot paths actually see.
    let mut g = c.benchmark_group("parse_single_short");
    let short = "https://example.com/bench";
    g.throughput(Throughput::Bytes(short.len() as u64));
    g.bench_function("rust-url", |b| {
        b.iter(|| black_box(black_box(short).parse::<url::Url>().unwrap()))
    });
    g.bench_function("ada", |b| {
        b.iter(|| black_box(ada_url::Url::parse(black_box(short), None).unwrap()))
    });
    g.finish();

    // 4. Relative-URL resolution against a base. ada's only API for this
    // re-parses the base string on every call, so the fair comparison makes
    // rust-url parse the base too; the pre-parsed variant is reported
    // separately since it is what a rust-url caller would actually write.
    let mut g = c.benchmark_group("join_relative");
    let base = "https://example.com/a/b/c?x=1";
    let rel = "../d/e.html?y=2#frag";
    g.bench_function("rust-url (base reparsed)", |b| {
        b.iter(|| {
            let base = black_box(base).parse::<url::Url>().unwrap();
            black_box(base.join(black_box(rel)).unwrap())
        })
    });
    g.bench_function("rust-url (base preparsed)", |b| {
        let base = base.parse::<url::Url>().unwrap();
        b.iter(|| black_box(base.join(black_box(rel)).unwrap()))
    });
    g.bench_function("ada (base reparsed)", |b| {
        b.iter(|| black_box(ada_url::Url::parse(black_box(rel), Some(base)).unwrap()))
    });
    g.finish();

    // 4b. Module-specifier shapes: one base, many short relative specifiers.
    // This is Deno's actual resolution hot path.
    let mut g = c.benchmark_group("module_specifiers");
    let mod_base = "https://deno.land/x/oak@v12.6.1/mod.ts";
    let specs = [
        "./router.ts",
        "../deps.ts",
        "./middleware/proxy.ts",
        "sibling.ts",
        "/std/http/server.ts",
    ];
    g.bench_function("rust-url join (base preparsed)", |b| {
        let base = mod_base.parse::<url::Url>().unwrap();
        b.iter(|| {
            for s in &specs {
                black_box(base.join(black_box(s)).unwrap());
            }
        })
    });
    g.bench_function("ada (base reparsed)", |b| {
        b.iter(|| {
            for s in &specs {
                black_box(ada_url::Url::parse(black_box(*s), Some(mod_base)).unwrap());
            }
        })
    });
    g.finish();

    // 4c. Joining an *absolute* specifier onto a base. The base is ignored, so
    // this is really an absolute parse wearing a join's clothes.
    let mut g = c.benchmark_group("join_absolute");
    let abs_specs = [
        "https://deno.land/std@0.200.0/http/server.ts",
        "https://esm.sh/react@18.2.0",
        "http://example.com/a/b/c.ts",
    ];
    g.bench_function("rust-url join (base preparsed)", |b| {
        let base = mod_base.parse::<url::Url>().unwrap();
        b.iter(|| {
            for s in &abs_specs {
                black_box(base.join(black_box(s)).unwrap());
            }
        })
    });
    g.bench_function("ada (base reparsed)", |b| {
        b.iter(|| {
            for s in &abs_specs {
                black_box(ada_url::Url::parse(black_box(*s), Some(mod_base)).unwrap());
            }
        })
    });
    g.finish();

    // 5. IDN / unicode host, where rust-url pays for full IDNA tables.
    let mut g = c.benchmark_group("parse_idn");
    let idn = "https://الاسم.مثال/path";
    g.bench_function("rust-url", |b| {
        b.iter(|| black_box(black_box(idn).parse::<url::Url>().unwrap()))
    });
    g.bench_function("ada", |b| {
        b.iter(|| black_box(ada_url::Url::parse(black_box(idn), None).unwrap()))
    });
    g.finish();
}

/// Is a Deno-side caching layer worth more than tuning the parser?
/// One base, many repeated specifiers — the module-resolution shape.
fn bench_cache(c: &mut Criterion) {
    use std::collections::HashMap;
    use std::rc::Rc;

    let base = "https://deno.land/x/oak@v12.6.1/mod.ts"
        .parse::<url::Url>()
        .unwrap();
    let specs = [
        "./router.ts",
        "../deps.ts",
        "./middleware/proxy.ts",
        "sibling.ts",
        "/std/http/server.ts",
    ];

    // Per-base cache, so lookups key on the specifier alone and no key
    // allocation is needed on the hot path.
    let mut cache: HashMap<Box<str>, Rc<url::Url>> = HashMap::new();
    for s in &specs {
        cache.insert((*s).into(), Rc::new(base.join(s).unwrap()));
    }
    let mut owned_cache: HashMap<Box<str>, url::Url> = HashMap::new();
    for s in &specs {
        owned_cache.insert((*s).into(), base.join(s).unwrap());
    }

    let mut g = c.benchmark_group("resolution_strategies");
    g.bench_function("parse every time (join)", |b| {
        b.iter(|| {
            for s in &specs {
                black_box(base.join(black_box(s)).unwrap());
            }
        })
    });
    g.bench_function("cache hit -> Rc clone", |b| {
        b.iter(|| {
            for s in &specs {
                black_box(Rc::clone(cache.get(black_box(*s)).unwrap()));
            }
        })
    });
    g.bench_function("cache hit -> deep Url clone", |b| {
        b.iter(|| {
            for s in &specs {
                black_box(owned_cache.get(black_box(*s)).unwrap().clone());
            }
        })
    });
    g.bench_function("cache hit -> &Url borrow", |b| {
        b.iter(|| {
            for s in &specs {
                black_box(owned_cache.get(black_box(*s)).unwrap().as_str().len());
            }
        })
    });
    g.finish();
}

criterion_group!(benches, bench, bench_cache);
criterion_main!(benches);
