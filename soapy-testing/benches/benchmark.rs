use std::{
    array::IntoIter,
    ops::{Add, ControlFlow, Deref, DerefMut, Mul},
    slice::Iter,
};

use criterion::{criterion_group, criterion_main, Criterion};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use soapy::{SliceRef, Soa, Soapy};

struct Rng(StdRng);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(StdRng::seed_from_u64(seed))
    }

    fn next_f32(&mut self) -> f32 {
        f32::from_ne_bytes(self.0.next_u32().to_ne_bytes())
    }
}

#[derive(Soapy, Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Vec4(
    #[align(64)] f32,
    #[align(64)] f32,
    #[align(64)] f32,
    #[align(64)] f32,
);

impl Vec4 {
    fn new_rng(rng: &mut Rng) -> Self {
        Self(
            rng.next_f32(),
            rng.next_f32(),
            rng.next_f32(),
            rng.next_f32(),
        )
    }
}

fn make_vec4_list<T>(rng: &mut Rng, count: usize) -> T
where
    T: FromIterator<Vec4>,
{
    std::iter::repeat_with(|| Vec4::new_rng(rng))
        .take(count)
        .collect()
}

fn sum_dots_vec(a: &[Vec4], b: &[Vec4]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(a, b)| a.0 * b.0 + a.1 * b.1 + a.2 * b.2 + a.3 * b.3)
        .sum()
}

fn sum_dots_soa(a: SliceRef<Vec4>, b: SliceRef<Vec4>) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(a, b)| a.0 * b.0 + a.1 * b.1 + a.2 * b.2 + a.3 * b.3)
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Vec4Arrays<const N: usize>([f32; N], [f32; N], [f32; N], [f32; N]);

impl<const N: usize> Vec4Arrays<N> {
    fn new_rng(rng: &mut Rng) -> Self {
        let mut out = Self([0.0; N], [0.0; N], [0.0; N], [0.0; N]);
        for i in 0..N {
            out.0[i] = rng.next_f32();
            out.1[i] = rng.next_f32();
            out.2[i] = rng.next_f32();
            out.3[i] = rng.next_f32();
        }
        out
    }

    fn iter(&self) -> impl Iterator<Item = (&f32, &f32, &f32, &f32)> {
        self.0
            .iter()
            .zip(self.1.iter())
            .zip(self.2.iter())
            .zip(self.3.iter())
            .map(|(((a0, a1), a2), a3)| (a0, a1, a2, a3))
    }
}

fn sum_dots_arrays<const N: usize>(a: &Vec4Arrays<N>, b: &Vec4Arrays<N>) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(a, b)| a.0 * b.0 + a.1 * b.1 + a.2 * b.2 + a.3 * b.3)
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(align(64))]
struct AlignedArray<const N: usize>([f32; N]);

impl<const N: usize> AlignedArray<N> {
    fn new_rng(rng: &mut Rng) -> Self {
        let mut out = [0.0; N];
        for el in out.iter_mut() {
            *el = rng.next_f32();
        }
        Self(out)
    }
}

impl<const N: usize> Deref for AlignedArray<N> {
    type Target = [f32; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for AlignedArray<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Vec4ArraysAligned<const N: usize>(
    AlignedArray<N>,
    AlignedArray<N>,
    AlignedArray<N>,
    AlignedArray<N>,
);

impl<const N: usize> Vec4ArraysAligned<N> {
    fn new_rng(rng: &mut Rng) -> Self {
        Self(
            AlignedArray::new_rng(rng),
            AlignedArray::new_rng(rng),
            AlignedArray::new_rng(rng),
            AlignedArray::new_rng(rng),
        )
    }

    fn iter(&self) -> impl Iterator<Item = (&f32, &f32, &f32, &f32)> {
        self.0
            .iter()
            .zip(self.1.iter())
            .zip(self.2.iter())
            .zip(self.3.iter())
            .map(|(((a, b), c), d)| (a, b, c, d))
    }
}

fn sum_dots_arrays_aligned<const N: usize>(
    a: &Vec4ArraysAligned<N>,
    b: &Vec4ArraysAligned<N>,
) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(a, b)| a.0 * b.0 + a.1 * b.1 + a.2 * b.2 + a.3 * b.3)
        .sum()
}

#[repr(align(32))]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct F32Group([f32; 8]);

impl Mul for F32Group {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self([
            self.0[0] * rhs.0[0],
            self.0[1] * rhs.0[1],
            self.0[2] * rhs.0[2],
            self.0[3] * rhs.0[3],
            self.0[4] * rhs.0[4],
            self.0[5] * rhs.0[5],
            self.0[6] * rhs.0[6],
            self.0[7] * rhs.0[7],
        ])
    }
}

impl Mul for &F32Group {
    type Output = F32Group;

    fn mul(self, rhs: Self) -> Self::Output {
        *self * *rhs
    }
}

impl Add for F32Group {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
            self.0[3] + rhs.0[3],
            self.0[4] + rhs.0[4],
            self.0[5] + rhs.0[5],
            self.0[6] + rhs.0[6],
            self.0[7] + rhs.0[7],
        ])
    }
}

impl Add for &F32Group {
    type Output = F32Group;

    fn add(self, rhs: Self) -> Self::Output {
        *self + *rhs
    }
}

impl IntoIterator for F32Group {
    type Item = f32;

    type IntoIter = IntoIter<f32, 8>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a F32Group {
    type Item = &'a f32;

    type IntoIter = Iter<'a, f32>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl F32Group {
    const ZERO: Self = Self([0.0; 8]);

    fn sum(self) -> f32 {
        self.0.into_iter().sum()
    }

    fn iter(&self) -> Iter<'_, f32> {
        self.0.iter()
    }

    fn new_rng(rng: &mut Rng) -> Self {
        Self([
            rng.next_f32(),
            rng.next_f32(),
            rng.next_f32(),
            rng.next_f32(),
            rng.next_f32(),
            rng.next_f32(),
            rng.next_f32(),
            rng.next_f32(),
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Vec4ArraysGrouped<const N: usize>(
    [F32Group; N],
    [F32Group; N],
    [F32Group; N],
    [F32Group; N],
);

impl<const N: usize> Vec4ArraysGrouped<N> {
    fn new_rng(rng: &mut Rng) -> Self {
        let mut out = Self(
            [F32Group::ZERO; N],
            [F32Group::ZERO; N],
            [F32Group::ZERO; N],
            [F32Group::ZERO; N],
        );
        for i in 0..N {
            out.0[i] = F32Group::new_rng(rng);
            out.1[i] = F32Group::new_rng(rng);
            out.2[i] = F32Group::new_rng(rng);
            out.3[i] = F32Group::new_rng(rng);
        }
        out
    }

    fn iter(&self) -> impl Iterator<Item = (&F32Group, &F32Group, &F32Group, &F32Group)> {
        self.0
            .iter()
            .zip(self.1.iter())
            .zip(self.2.iter())
            .zip(self.3.iter())
            .map(|(((a, b), c), d)| (a, b, c, d))
    }
}

fn sum_dots_grouped<const N: usize>(a: &Vec4ArraysGrouped<N>, b: &Vec4ArraysGrouped<N>) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(a, b)| (a.0 * b.0 + a.1 * b.1 + a.2 * b.2 + a.3 * b.3).sum())
        .sum()
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = Rng::new(42);

    let array1 = <Vec4ArraysGrouped<{ 1 << 13 }>>::new_rng(&mut rng);
    let array2 = <Vec4ArraysGrouped<{ 1 << 13 }>>::new_rng(&mut rng);
    c.bench_function("dots-grouped-array", |b| {
        b.iter(|| sum_dots_grouped(&array1, &array2))
    });

    let array1 = <Vec4Arrays<{ 1 << 16 }>>::new_rng(&mut rng);
    let array2 = <Vec4Arrays<{ 1 << 16 }>>::new_rng(&mut rng);
    c.bench_function("dots-array", |b| {
        b.iter(|| sum_dots_arrays(&array1, &array2))
    });

    let array1 = <Vec4ArraysAligned<{ 1 << 16 }>>::new_rng(&mut rng);
    let array2 = <Vec4ArraysAligned<{ 1 << 16 }>>::new_rng(&mut rng);
    c.bench_function("dots-aligned-array", |b| {
        b.iter(|| sum_dots_arrays_aligned(&array1, &array2))
    });

    let soa1: Soa<_> = make_vec4_list(&mut rng, 1 << 16);
    let soa2: Soa<_> = make_vec4_list(&mut rng, 1 << 16);
    c.bench_function("dots-soa", |b| {
        b.iter(|| sum_dots_soa(soa1.as_slice(), soa2.as_slice()))
    });

    c.bench_function("dots-fold-soa", |b| {
        b.iter(|| {
            soa1.try_fold_zip(&soa2, 0.0, |acc, a, b| {
                ControlFlow::Continue(acc + a.0 * b.0 + a.1 * b.1 + a.2 * b.2 + a.3 * b.3)
            })
        })
    });

    let vec1: Vec<_> = make_vec4_list(&mut rng, 1 << 16);
    let vec2: Vec<_> = make_vec4_list(&mut rng, 1 << 16);
    c.bench_function("dots-vec", |b| b.iter(|| sum_dots_vec(&vec1, &vec2)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
