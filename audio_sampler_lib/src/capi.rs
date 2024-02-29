mod sampler1 {
    use crate::common_types::{InitParams, Params};
    use crate::sampler::Sampler;
    use core::slice;

    #[no_mangle]
    pub extern "C" fn sampler_params_default() -> Params {
        Params::default()
    }

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
        let out: &[*mut f32] = slice::from_raw_parts(outputs, sampler.channels.len());
        let mut out: Vec<&mut [f32]> = out
            .into_iter()
            .map(|v| slice::from_raw_parts_mut(*v, frames))
            .collect();
        let inputs: &[*const f32] = slice::from_raw_parts(inputs, sampler.channels.len());
        let inputs: Vec<&[f32]> = inputs
            .into_iter()
            .map(|v| slice::from_raw_parts(*v, frames))
            .collect();
        for channel in 0..inputs.len() {
            for i in 0..frames {
                out[channel][i] = sampler.process_sample(channel, inputs[channel][i], params);
            }
        }
    }
}
