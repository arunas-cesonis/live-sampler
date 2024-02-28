use crate::common_types::{InitParams, Params};
use crate::sampler::Sampler;
use core::slice;

#[no_mangle]
pub extern "C" fn sampler_new(channel_count: usize, params: &InitParams) -> *mut Sampler {
    let b = Box::new(Sampler::new(channel_count, params));
    let b_ptr = Box::into_raw(b);
    b_ptr
}

#[no_mangle]
pub unsafe extern "C" fn sampler_free(sampler: *mut Sampler) {
    let _ = Box::from_raw(sampler);
}

#[no_mangle]
pub unsafe extern "C" fn sampler_reset(sampler: &mut Sampler) {
    sampler.reset()
}

#[no_mangle]
pub unsafe extern "C" fn sampler_process_frame<'a>(
    sampler: &mut Sampler,
    inputs: *mut *const f32,
    outputs: *mut *mut f32,
    frames: usize,
    params: &Params,
) {
    let inputs: &[*const f32] = slice::from_raw_parts(inputs, sampler.channels.len());
    let inputs: Vec<&[f32]> = inputs
        .into_iter()
        .map(|v| slice::from_raw_parts(*v, frames))
        .collect();
    eprintln!("{:?}", inputs);
    //    sampler.process_frame(frame, params)
}
