use crystal_planes::rad::ffs;
use rand::prelude::*;
use std::io::Write;
use std::{fs::File, io::BufWriter};

fn log(v: f32) -> f32 {
    -(v.log2())
}

fn main() -> Result<(), Box<std::error::Error>> {
    let extents = ffs::Extents::load("extents.bin").ok_or("failed to load extents.bin")?;

    let ffs = extents
        .0
        .iter()
        .flat_map(|v| v.iter().flat_map(|e| e.ffs.iter()))
        .cloned()
        .collect::<Vec<f32>>();

    // let ffs = ffs.iter().map(|f| -(f.log2())).collect::<Vec<_>>();

    let min = ffs.iter().fold(f32::NAN, |a, v| a.min(*v));
    let max = ffs.iter().fold(f32::NAN, |a, v| a.max(*v));
    let n = 256;

    // let min = 0.1f32;
    // let max = 0.9f32;
    // let n = 2;
    println!("minmax: {} {}", min, max);
    let minl = log(min);
    let maxl = log(max + 0.0001);
    let range_len = (maxl - minl) / (n as f32);
    let bin_upper = (0..n)
        .scan(min, |a, i| {
            *a = (-(minl + (i + 1) as f32 * range_len)).exp2();
            Some(*a)
        })
        .collect::<Vec<_>>();

    // let bin_upper_log = (0..n)
    //     .scan(min, |a, i| {
    //         *a = minl + (i + 1) as f32 * range_len;
    //         Some(*a)
    //     })
    //     .collect::<Vec<_>>();

    println!("max: {} {} {}", max, n as f32 * range_len, range_len);
    println!("bins: {:?}", bin_upper);

    let mut f = Box::new(BufWriter::new(File::create("ffs.txt")?));
    println!("sampling...");
    let mut count = vec![0; n];
    for i in ffs.choose_multiple(&mut rand::thread_rng(), 10000) {
        write!(f, "{}\n", i)?;
    }
    println!("histogram");
    for i in ffs.iter() {
        // let i = log(*i);
        count[(0..n)
            .position(|bin| bin_upper[bin] >= *i)
            .ok_or_else(|| format!("failed to find bin for {}", i))?] += 1;
    }
    println!("{:?}", count);
    let mut f = Box::new(BufWriter::new(File::create("hist.txt")?));

    for c in count {
        write!(f, "{}\n", c)?;
    }
    Ok(())
}
