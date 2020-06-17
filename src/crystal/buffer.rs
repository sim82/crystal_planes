#[derive(Clone)]
pub struct RadBuffer {
    pub r: Vec<f32>,
    pub g: Vec<f32>,
    pub b: Vec<f32>,
}
pub type RadSlice<'a> = (&'a [f32], &'a [f32], &'a [f32]);
pub type MutRadSlice<'a> = (&'a mut [f32], &'a mut [f32], &'a mut [f32]);

pub fn aligned_vector<T>(len: usize, align: usize) -> Vec<T> {
    let t_size = std::mem::size_of::<T>();
    let t_align = std::mem::align_of::<T>();
    let layout = if t_align >= align {
        std::alloc::Layout::from_size_align(t_size * len, t_align).unwrap()
    } else {
        std::alloc::Layout::from_size_align(t_size * len, align).unwrap()
    };
    unsafe {
        let mem = std::alloc::alloc(layout);
        assert_eq!((mem as usize) % 16, 0);
        Vec::<T>::from_raw_parts(mem as *mut T, len, len)
    }
}

pub fn aligned_vector_init<T: Copy>(len: usize, align: usize, init: T) -> Vec<T> {
    let mut v = aligned_vector::<T>(len, align);
    for x in v.iter_mut() {
        *x = init;
    }
    v
}

impl RadBuffer {
    /// Utility for making specifically aligned vectors

    pub fn new(size: usize) -> RadBuffer {
        // println!("RadBuf: size: {}", size * 3 * 4);
        RadBuffer {
            r: aligned_vector_init(size, 64, 0f32),
            g: aligned_vector_init(size, 64, 0f32),
            b: aligned_vector_init(size, 64, 0f32),
        }
    }

    #[allow(unused)]
    pub fn slice(&self, i: std::ops::Range<usize>) -> RadSlice<'_> {
        (&self.r[i.clone()], &self.g[i.clone()], &self.b[i.clone()])
    }
    #[allow(unused)]
    pub fn slice_mut(&mut self, i: std::ops::Range<usize>) -> MutRadSlice<'_> {
        (
            &mut self.r[i.clone()],
            &mut self.g[i.clone()],
            &mut self.b[i.clone()],
        )
    }
    // this is a bit redundant, but found no better way since SliceIndex is non-copy and thus cannot be used for indexing multiple Vecs
    pub fn slice_full(&self) -> RadSlice<'_> {
        (&self.r[..], &self.g[..], &self.b[..])
    }
    #[allow(unused)]
    pub fn slice_full_mut(&mut self) -> MutRadSlice<'_> {
        (&mut self.r[..], &mut self.g[..], &mut self.b[..])
    }
    #[allow(unused)]
    pub fn chunks_mut(&mut self, size: usize) -> impl Iterator<Item = MutRadSlice<'_>> {
        itertools::izip!(
            self.r.chunks_mut(size),
            self.g.chunks_mut(size),
            self.b.chunks_mut(size)
        )
    }

    pub fn chunks_mut2(
        &mut self,
        size: usize,
    ) -> (
        impl Iterator<Item = &mut [f32]>,
        impl Iterator<Item = &mut [f32]>,
        impl Iterator<Item = &mut [f32]>,
    ) {
        (
            self.r.chunks_mut(size),
            self.g.chunks_mut(size),
            self.b.chunks_mut(size),
        )
    }
}
