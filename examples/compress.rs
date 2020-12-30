use crystal_planes::rad::compress;
use crystal_planes::rad::ffs;
fn main() -> Result<(), Box<std::error::Error>> {
    let extents = ffs::Extents::load("extents.bin").ok_or("failed to load extents.bin")?;

    for extents in extents.0.iter() {
        compress::ExtentsCompressed::from_extents(&extents);
    }

    Ok(())
}
