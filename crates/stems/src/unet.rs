//! Spleeter's U-Net (one per stem), run on the CPU.
//!
//! The same network as Deezer's `apply_unet` with ELU activations: six 5×5
//! stride-2 convolutions (16…512 filters) with batch norm, six 5×5 stride-2
//! transposed convolutions back up, joined to the encoder's raw convolution
//! outputs, then a 4×4 dilated convolution giving a sigmoid mask that's
//! multiplied with the input spectrogram. Tensors are height × width ×
//! channels (time × frequency × channels), like TensorFlow's NHWC.
//!
//! Convolutions are im2col plus one matrix multiplication, split across
//! threads by output rows.

use crate::model::Weights;

/// Keras' batch-norm epsilon.
const BN_EPS: f32 = 1e-3;

#[derive(Clone)]
pub struct Tensor {
    pub h: usize,
    pub w: usize,
    pub c: usize,
    pub d: Vec<f32>,
}

impl Tensor {
    pub fn zeros(h: usize, w: usize, c: usize) -> Self {
        Self {
            h,
            w,
            c,
            d: vec![0.0; h * w * c],
        }
    }
}

struct Conv {
    k: usize,
    cin: usize,
    cout: usize,
    stride: usize,
    dilation: usize,
    /// [kh][kw][cin][cout], TensorFlow's layout, which is exactly im2col's.
    w: Vec<f32>,
    b: Vec<f32>,
}

struct ConvT {
    k: usize,
    cin: usize,
    cout: usize,
    /// [cin][kh][kw][cout]: reordered from TensorFlow's [kh][kw][cout][cin].
    w: Vec<f32>,
    b: Vec<f32>,
}

/// Batch norm folded into a scale and shift per channel.
struct Bn {
    scale: Vec<f32>,
    shift: Vec<f32>,
}

pub struct UNet {
    enc: Vec<Conv>,
    enc_bn: Vec<Bn>,
    dec: Vec<ConvT>,
    dec_bn: Vec<Bn>,
    last: Conv,
}

fn suffix(n: usize) -> String {
    if n == 0 {
        String::new()
    } else {
        format!("_{n}")
    }
}

impl UNet {
    /// The network for stem `index` (0 vocals, 1 drums, 2 bass, 3 other):
    /// layers are named in the order Deezer built them, one network after another.
    pub fn load(weights: &Weights, index: usize) -> Result<Self, String> {
        let conv = |n: usize, stride: usize, dilation: usize| -> Result<Conv, String> {
            let (shape, w) = weights.get(&format!("conv2d{}/kernel", suffix(n)))?;
            let (_, b) = weights.get(&format!("conv2d{}/bias", suffix(n)))?;
            Ok(Conv {
                k: shape[0],
                cin: shape[2],
                cout: shape[3],
                stride,
                dilation,
                w: w.clone(),
                b: b.clone(),
            })
        };
        let conv_t = |n: usize| -> Result<ConvT, String> {
            let (shape, w) = weights.get(&format!("conv2d_transpose{}/kernel", suffix(n)))?;
            let (_, b) = weights.get(&format!("conv2d_transpose{}/bias", suffix(n)))?;
            let (k, cout, cin) = (shape[0], shape[2], shape[3]);
            let mut r = vec![0.0; w.len()];
            for ky in 0..k {
                for kx in 0..k {
                    for co in 0..cout {
                        for ci in 0..cin {
                            r[((ci * k + ky) * k + kx) * cout + co] =
                                w[((ky * k + kx) * cout + co) * cin + ci];
                        }
                    }
                }
            }
            Ok(ConvT {
                k,
                cin,
                cout,
                w: r,
                b: b.clone(),
            })
        };
        let bn = |n: usize| -> Result<Bn, String> {
            let g = |p: &str| {
                weights
                    .get(&format!("batch_normalization{}/{p}", suffix(n)))
                    .map(|t| t.1.clone())
            };
            let (gamma, beta, mean, var) = (
                g("gamma")?,
                g("beta")?,
                g("moving_mean")?,
                g("moving_variance")?,
            );
            let scale: Vec<f32> = gamma
                .iter()
                .zip(&var)
                .map(|(g, v)| g / (v + BN_EPS).sqrt())
                .collect();
            let shift = beta
                .iter()
                .zip(&mean)
                .zip(&scale)
                .map(|((b, m), s)| b - m * s)
                .collect();
            Ok(Bn { scale, shift })
        };
        let (c0, t0, b0) = (7 * index, 6 * index, 12 * index);
        Ok(Self {
            enc: (0..6)
                .map(|i| conv(c0 + i, 2, 1))
                .collect::<Result<_, _>>()?,
            enc_bn: (0..5).map(|i| bn(b0 + i)).collect::<Result<_, _>>()?,
            dec: (0..6).map(|i| conv_t(t0 + i)).collect::<Result<_, _>>()?,
            dec_bn: (6..12).map(|i| bn(b0 + i)).collect::<Result<_, _>>()?,
            last: conv(c0 + 6, 1, 2)?,
        })
    }

    /// The stem's estimated spectrogram for a mix spectrogram (time × 1024 × 2).
    pub fn forward(&self, input: &Tensor, threads: usize) -> Tensor {
        // Encoder: keep each raw convolution output for the skip connections.
        let mut raw: Vec<Tensor> = Vec::with_capacity(6);
        let mut x = input.clone();
        for (i, layer) in self.enc.iter().enumerate() {
            let c = conv(&x, layer, threads);
            if i < 5 {
                x = c.clone();
                batch_norm(&mut x, &self.enc_bn[i]);
                elu(&mut x);
            }
            raw.push(c);
        }
        // Decoder: transposed convolution, ELU, batch norm, then join the encoder's output.
        let mut y = raw[5].clone();
        for (i, layer) in self.dec.iter().enumerate() {
            let mut up = conv_t(&y, layer, threads);
            elu(&mut up);
            batch_norm(&mut up, &self.dec_bn[i]);
            y = if i < 5 { concat(&raw[4 - i], &up) } else { up };
        }
        let mut mask = conv(&y, &self.last, threads);
        for v in &mut mask.d {
            *v = 1.0 / (1.0 + (-*v).exp());
        }
        for (m, x) in mask.d.iter_mut().zip(&input.d) {
            *m *= x;
        }
        mask
    }
}

fn elu(t: &mut Tensor) {
    for v in &mut t.d {
        if *v < 0.0 {
            *v = v.exp() - 1.0;
        }
    }
}

fn batch_norm(t: &mut Tensor, bn: &Bn) {
    for px in t.d.chunks_exact_mut(t.c) {
        for ((v, s), b) in px.iter_mut().zip(&bn.scale).zip(&bn.shift) {
            *v = *v * s + b;
        }
    }
}

fn concat(a: &Tensor, b: &Tensor) -> Tensor {
    let c = a.c + b.c;
    let mut out = Tensor::zeros(a.h, a.w, c);
    for ((o, pa), pb) in out
        .d
        .chunks_exact_mut(c)
        .zip(a.d.chunks_exact(a.c))
        .zip(b.d.chunks_exact(b.c))
    {
        o[..a.c].copy_from_slice(pa);
        o[a.c..].copy_from_slice(pb);
    }
    out
}

/// TensorFlow's "same" padding before the first row/column.
fn pad_before(input: usize, output: usize, stride: usize, effective_k: usize) -> usize {
    ((output - 1) * stride + effective_k).saturating_sub(input) / 2
}

/// C (m×n) = A (m×k) · B (k×n), all row-major.
fn gemm(m: usize, k: usize, n: usize, a: &[f32], b: &[f32], c: &mut [f32]) {
    // SAFETY: the slices hold at least m×k, k×n and m×n values with these strides.
    unsafe {
        matrixmultiply::sgemm(
            m,
            k,
            n,
            1.0,
            a.as_ptr(),
            k as isize,
            1,
            b.as_ptr(),
            n as isize,
            1,
            0.0,
            c.as_mut_ptr(),
            n as isize,
            1,
        );
    }
}

fn conv(x: &Tensor, l: &Conv, threads: usize) -> Tensor {
    let ke = (l.k - 1) * l.dilation + 1;
    let (oh, ow) = (x.h.div_ceil(l.stride), x.w.div_ceil(l.stride));
    let (pt, pl) = (
        pad_before(x.h, oh, l.stride, ke),
        pad_before(x.w, ow, l.stride, ke),
    );
    let kk = l.k * l.k * l.cin;
    let mut out = Tensor::zeros(oh, ow, l.cout);
    let rows_per = oh.div_ceil(threads.max(1)).max(1);
    std::thread::scope(|s| {
        for (chunk_i, chunk) in out.d.chunks_mut(rows_per * ow * l.cout).enumerate() {
            s.spawn(move || {
                let first_row = chunk_i * rows_per;
                let rows = chunk.len() / (ow * l.cout);
                // A few output rows at a time keeps the im2col buffer small.
                let step = (8192 / ow).max(1);
                let mut a = vec![0.0f32; step * ow * kk];
                let mut r = 0;
                while r < rows {
                    let n_rows = step.min(rows - r);
                    let px = n_rows * ow;
                    let a = &mut a[..px * kk];
                    a.fill(0.0);
                    for dr in 0..n_rows {
                        let oy = first_row + r + dr;
                        for ox in 0..ow {
                            let row = &mut a[(dr * ow + ox) * kk..(dr * ow + ox + 1) * kk];
                            for ky in 0..l.k {
                                let iy = (oy * l.stride + ky * l.dilation) as isize - pt as isize;
                                if iy < 0 || iy >= x.h as isize {
                                    continue;
                                }
                                for kx in 0..l.k {
                                    let ix =
                                        (ox * l.stride + kx * l.dilation) as isize - pl as isize;
                                    if ix < 0 || ix >= x.w as isize {
                                        continue;
                                    }
                                    let src = (iy as usize * x.w + ix as usize) * x.c;
                                    let dst = (ky * l.k + kx) * l.cin;
                                    row[dst..dst + l.cin].copy_from_slice(&x.d[src..src + l.cin]);
                                }
                            }
                        }
                    }
                    let out = &mut chunk[r * ow * l.cout..(r + n_rows) * ow * l.cout];
                    gemm(px, kk, l.cout, a, &l.w, out);
                    for p in out.chunks_exact_mut(l.cout) {
                        for (v, b) in p.iter_mut().zip(&l.b) {
                            *v += b;
                        }
                    }
                    r += n_rows;
                }
            });
        }
    });
    out
}

fn conv_t(x: &Tensor, l: &ConvT, threads: usize) -> Tensor {
    // A stride-2 "same" transposed convolution doubles the size.
    let (oh, ow) = (x.h * 2, x.w * 2);
    let pt = pad_before(oh, x.h, 2, l.k);
    let pl = pad_before(ow, x.w, 2, l.k);
    let n = l.k * l.k * l.cout;
    let npx = x.h * x.w;
    // Z = X · W: each input pixel's contribution to every kernel position.
    let mut z = vec![0.0f32; npx * n];
    let per = npx.div_ceil(threads.max(1)).max(1);
    std::thread::scope(|s| {
        for (i, zc) in z.chunks_mut(per * n).enumerate() {
            let rows = zc.len() / n;
            let a = &x.d[i * per * l.cin..(i * per + rows) * l.cin];
            s.spawn(move || gemm(rows, l.cin, n, a, &l.w, zc));
        }
    });
    // Scatter-add into the output.
    let mut out = Tensor::zeros(oh, ow, l.cout);
    for iy in 0..x.h {
        for ix in 0..x.w {
            let zp = &z[(iy * x.w + ix) * n..(iy * x.w + ix + 1) * n];
            for ky in 0..l.k {
                let y = (iy * 2 + ky) as isize - pt as isize;
                if y < 0 || y >= oh as isize {
                    continue;
                }
                for kx in 0..l.k {
                    let xx = (ix * 2 + kx) as isize - pl as isize;
                    if xx < 0 || xx >= ow as isize {
                        continue;
                    }
                    let o = (y as usize * ow + xx as usize) * l.cout;
                    let src = &zp[(ky * l.k + kx) * l.cout..(ky * l.k + kx + 1) * l.cout];
                    for (v, s) in out.d[o..o + l.cout].iter_mut().zip(src) {
                        *v += s;
                    }
                }
            }
        }
    }
    for p in out.d.chunks_exact_mut(l.cout) {
        for (v, b) in p.iter_mut().zip(&l.b) {
            *v += b;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A direct (slow) convolution, to check the im2col one.
    fn naive(x: &Tensor, l: &Conv) -> Tensor {
        let ke = (l.k - 1) * l.dilation + 1;
        let (oh, ow) = (x.h.div_ceil(l.stride), x.w.div_ceil(l.stride));
        let (pt, pl) = (
            pad_before(x.h, oh, l.stride, ke),
            pad_before(x.w, ow, l.stride, ke),
        );
        let mut out = Tensor::zeros(oh, ow, l.cout);
        for oy in 0..oh {
            for ox in 0..ow {
                for co in 0..l.cout {
                    let mut s = l.b[co];
                    for ky in 0..l.k {
                        for kx in 0..l.k {
                            let iy = (oy * l.stride + ky * l.dilation) as isize - pt as isize;
                            let ix = (ox * l.stride + kx * l.dilation) as isize - pl as isize;
                            if iy < 0 || ix < 0 || iy >= x.h as isize || ix >= x.w as isize {
                                continue;
                            }
                            for ci in 0..l.cin {
                                s += x.d[(iy as usize * x.w + ix as usize) * x.c + ci]
                                    * l.w[((ky * l.k + kx) * l.cin + ci) * l.cout + co];
                            }
                        }
                    }
                    out.d[(oy * ow + ox) * l.cout + co] = s;
                }
            }
        }
        out
    }

    fn pseudo(n: usize, seed: u32) -> Vec<f32> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s.wrapping_mul(1_103_515_245).wrapping_add(12345);
                ((s >> 16) as f32 / 65536.0) - 0.5
            })
            .collect()
    }

    #[test]
    fn im2col_convolution_matches_the_direct_one() {
        let x = Tensor {
            h: 9,
            w: 12,
            c: 3,
            d: pseudo(9 * 12 * 3, 1),
        };
        for (stride, dilation, k) in [(2, 1, 5), (1, 2, 4)] {
            let l = Conv {
                k,
                cin: 3,
                cout: 4,
                stride,
                dilation,
                w: pseudo(k * k * 3 * 4, 2),
                b: pseudo(4, 3),
            };
            let (a, b) = (conv(&x, &l, 3), naive(&x, &l));
            assert_eq!((a.h, a.w, a.c), (b.h, b.w, b.c));
            for (p, q) in a.d.iter().zip(&b.d) {
                assert!((p - q).abs() < 1e-4, "{p} vs {q}");
            }
        }
    }

    #[test]
    fn transposed_convolution_doubles_and_matches_the_definition() {
        let x = Tensor {
            h: 3,
            w: 4,
            c: 2,
            d: pseudo(24, 5),
        };
        let k = 5;
        // TensorFlow layout [kh][kw][cout][cin], reordered as `load` does.
        let tf = pseudo(k * k * 3 * 2, 6);
        let mut w = vec![0.0; tf.len()];
        for ky in 0..k {
            for kx in 0..k {
                for co in 0..3 {
                    for ci in 0..2 {
                        w[((ci * k + ky) * k + kx) * 3 + co] =
                            tf[((ky * k + kx) * 3 + co) * 2 + ci];
                    }
                }
            }
        }
        let l = ConvT {
            k,
            cin: 2,
            cout: 3,
            w,
            b: vec![0.0; 3],
        };
        let out = conv_t(&x, &l, 2);
        assert_eq!((out.h, out.w, out.c), (6, 8, 3));
        // Definition: out[2i + ky - 1][2j + kx - 1][co] += x[i][j][ci] * W[ky][kx][co][ci].
        let mut want = vec![0.0f32; 6 * 8 * 3];
        for i in 0..3 {
            for j in 0..4 {
                for ky in 0..k {
                    for kx in 0..k {
                        let (y, xx) = (
                            2 * i as isize + ky as isize - 1,
                            2 * j as isize + kx as isize - 1,
                        );
                        if y < 0 || xx < 0 || y >= 6 || xx >= 8 {
                            continue;
                        }
                        for co in 0..3 {
                            for ci in 0..2 {
                                want[(y as usize * 8 + xx as usize) * 3 + co] += x.d
                                    [(i * 4 + j) * 2 + ci]
                                    * tf[((ky * k + kx) * 3 + co) * 2 + ci];
                            }
                        }
                    }
                }
            }
        }
        for (p, q) in out.d.iter().zip(&want) {
            assert!((p - q).abs() < 1e-4);
        }
    }
}
