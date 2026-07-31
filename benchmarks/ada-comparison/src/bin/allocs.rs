use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

static ALLOC: AtomicUsize = AtomicUsize::new(0);
static REALLOC: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOC.fetch_add(1, Relaxed);
        BYTES.fetch_add(l.size(), Relaxed);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) { System.dealloc(p, l) }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        REALLOC.fetch_add(1, Relaxed);
        BYTES.fetch_add(n.saturating_sub(l.size()), Relaxed);
        System.realloc(p, l, n)
    }
}
#[global_allocator]
static A: Counting = Counting;

fn reset() { ALLOC.store(0, Relaxed); REALLOC.store(0, Relaxed); BYTES.store(0, Relaxed); }
fn report(label: &str, out_len: usize) {
    println!("{:<34} allocs={} reallocs={} bytes={} result_len={}",
        label, ALLOC.load(Relaxed), REALLOC.load(Relaxed), BYTES.load(Relaxed), out_len);
}

fn main() {
    let base: url::Url = "https://example.com/a/b/c?x=1".parse().unwrap();

    for rel in ["../d/e.html?y=2#frag", "./mod.ts", "/abs/path/mod.ts", "sibling.ts"] {
        reset();
        let r = base.join(rel).unwrap();
        report(&format!("join({:?})", rel), r.as_str().len());
        println!("{:>36}reserved={} (input.len)", "", rel.len());
    }

    reset();
    let u: url::Url = "https://example.com/bench".parse().unwrap();
    report("parse(\"https://example.com/bench\")", u.as_str().len());

    reset();
    let u2: url::Url = "https://example.com/a/b/c?x=1".parse().unwrap();
    report("parse(base)", u2.as_str().len());
}
