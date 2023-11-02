use crossbeam_queue::{ArrayQueue, SegQueue};
use dasp::interpolate::sinc::Sinc;
use dasp::signal::interpolate::Converter;
use dasp::{ring_buffer, Signal};
use live_sampler::LiveSampler;
use nih_plug::nih_export_standalone;
use std::sync::Arc;
use std::time::Instant;
use std::vec::IntoIter;

const ZERO: [f32; 2] = [0.0f32, 0.0f32];

fn main() {
    let rb = dasp::ring_buffer::Fixed::from([ZERO; 10]);
    let sinc = Sinc::new(rb);
    let f = dasp::interpolate::floor::Floor::new([0.0, 0.0]);
    let lin = dasp::interpolate::linear::Linear::new(ZERO, ZERO);
    let q = Arc::new(SegQueue::new());
    fn gen_vec(n: usize) -> Vec<[f32; 2]> {
        let mut v = vec![ZERO; n];

        for i in 0..n {
            let l = (i as f32) / n as f32;
            let r = ((n - i - 1) as f32) / n as f32;
            v.push([l, r]);
        }
        v
    }
    fn gen_buf(n: usize) -> dasp::ring_buffer::Bounded<Vec<[f32; 2]>> {
        dasp::ring_buffer::Bounded::from(gen_vec(n))
    }

    #[derive(Default)]
    struct SegQueueIntoIter {
        q: Arc<SegQueue<dasp::ring_buffer::Bounded<Vec<[f32; 2]>>>>,
    }

    struct SegQueueIter {
        q: Arc<SegQueue<dasp::ring_buffer::Bounded<Vec<[f32; 2]>>>>,
        buf: dasp::ring_buffer::Bounded<Vec<[f32; 2]>>,
    }
    impl IntoIterator for SegQueueIntoIter {
        type IntoIter = SegQueueIter;
        type Item = [f32; 2];
        fn into_iter(self) -> Self::IntoIter {
            SegQueueIter {
                q: self.q,
                buf: ring_buffer::Bounded::from(vec![ZERO; 1]),
            }
        }
    }
    impl Iterator for SegQueueIter {
        type Item = [f32; 2];
        fn next(&mut self) -> Option<Self::Item> {
            if let Some(x) = self.buf.pop() {
                Some(x)
            } else if let Some(new_buf) = self.q.pop() {
                self.buf = new_buf;
                self.buf.pop()
            } else {
                None
            }
        }
    }
    q.push(gen_buf(1024));
    let it = SegQueueIntoIter { q: q.clone() };
    let sig = dasp::signal::from_iter(it);
    //let sig = dasp::signal::gen_mut(|| {
    //    if buf.is_empty() {
    //        if let Some(new_buf) = q.pop() {
    //            buf = new_buf;
    //        }
    //    }
    //    buf.pop().unwrap_or(ZERO)
    //});

    let tmp = gen_vec(1023 * 1024);
    let tmp = dasp::signal::from_iter(tmp);
    let converted = Converter::from_hz_to_hz(tmp, lin, 44100.0, 22050.0);

    let mut c = 0;
    let start = Instant::now();
    let mut iter = converted.until_exhausted();
    let mut done = false;
    while !done && start.elapsed().as_secs() < 5 {
        for _ in 0..2000 {
            if iter.next().is_none() {
                done = true;
                break;
            }
            c += 1;
            //if q.len() < 1 {
            //    q.push(gen_buf());
            //}
        }
    }

    eprintln!("{}", c);
    eprintln!("{:?}", start.elapsed().as_millis());
    //converted.is_exhausted()
    ////nih_export_standalone::<LiveSampler>();
}
