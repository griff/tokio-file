//! A workalike of that old workhorse, dd.  It's basically the "Hello, World!"
//! of file I/O.
//!
//! You can try it out by running:
//!
//!     dd if=/dev/urandom of=/tmp/infile bs=4096 count=100
//!     cargo run --example dd -b 4096 -c 100 if=/tmp/infile of=/tmp/outfile
//!     cmp /tmp/infile /tmp/outfile

extern crate futures;
extern crate getopts;
extern crate libc;
extern crate tokio_core;
extern crate tokio_file;

use futures::future::{Future, ok};
use futures::{Stream, stream};
use getopts::Options;
use libc::{off_t};
use std::env;
use std::cell::Cell;
use std::rc::Rc;
use std::str::FromStr;
use tokio_core::reactor::Core;
use tokio_file::File;
use tokio_core::reactor::Handle;

struct Dd {
    pub bs: usize,
    pub count: usize,
    pub infile: File,
    pub outfile: File,
    pub ofs: Cell<off_t>,
}

impl Dd {
    pub fn new(infile: &str, outfile: &str, h: Handle, bs: usize, count: usize) -> Dd {
        let inf = File::open(infile, h.clone());
        let outf = File::open(outfile, h.clone());
        Dd {
            bs: bs,
            count: count,
            infile: inf.unwrap(),
            outfile: outf.unwrap(),
            ofs: Cell::new(0)}
    }
}

fn usage(opts: Options) {
    let brief = format!("Usage: dd [options] <INFILE> <OUTFILE>");
    print!("{}", opts.usage(&brief));
}


fn main() {
    let mut opts = Options::new();
    opts.optopt("b", "blocksize", "Block size in bytes", "BS");
    opts.optopt("c", "count", "Number of blocks to copy", "COUNT");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(env::args().skip(1)) {
        Ok(m) => {m},
        Err(f) => { panic!(f.to_string()) }
    };
    if matches.opt_present("h") {
        usage(opts);
        return;
    }
    let bs = match matches.opt_str("b") {
        Some(v) => usize::from_str(v.as_str()).unwrap(),
        None => 512
    };
    let count = match matches.opt_str("c") {
        Some(v) => usize::from_str(v.as_str()).unwrap(),
        None => 1
    };
    let infile = &matches.free[0];
    let outfile = &matches.free[1];

    let mut l = Core::new().unwrap();
    let dd = Dd::new(infile.as_str(), outfile.as_str(), l.handle(), bs, count);
    
    // Note: this simple example will fail if infile isn't big enough.  A robust
    // program would use loop_fn instead of stream.for_each so it can exit
    // early.
    let stream = stream::iter((0..dd.count).map(|r| Ok(r)));
    let fut = stream.for_each(|block| {
        let buf = Rc::new(vec![0; bs].into_boxed_slice());
        let bufclone = buf.clone();
        let ofs = (dd.bs * block) as off_t;
        dd.infile.read_at(buf.clone(), ofs)
        .unwrap()
        .and_then(|_| {
            let x = dd.outfile.write_at(bufclone, dd.ofs.get());
            x
            .unwrap()
            .and_then(|wlen| {
                dd.ofs.set(dd.ofs.get() + wlen as off_t);
                ok(())
            })
        })
    });
    l.run(fut).unwrap();
}
