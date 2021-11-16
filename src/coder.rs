pub struct Decoder {
    decoder: *mut opuscelt_sys::OpusCustomDecoder,
    mode: *mut opuscelt_sys::OpusCustomMode,
}

impl Decoder {
    pub fn new() -> Self {
        unsafe {
            let mode = opuscelt_sys::opus_custom_mode_create(22050, 512, std::ptr::null_mut());
            if mode.is_null() {
                panic!("opus_custom_mode_create returns null");
            }
            let decoder = opuscelt_sys::opus_custom_decoder_create(mode, 1, std::ptr::null_mut());
            if decoder.is_null() {
                panic!("opus_custom_decoder_create returns null");
            }

            Self { decoder, mode }
        }
    }

    pub fn decode(&mut self, data: &[u8], output: &mut [i16]) -> Result<(), i32> {
        unsafe {
            let ret = opuscelt_sys::opus_custom_decode(
                self.decoder,
                data.as_ptr(),
                data.len() as _,
                output.as_mut_ptr(),
                output.len() as _,
            );
            if ret < 0 {
                return Err(ret);
            }

            Ok(())
        }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe {
            opuscelt_sys::opus_custom_decoder_destroy(self.decoder);
            opuscelt_sys::opus_custom_mode_destroy(self.mode);
        }
    }
}

unsafe impl Send for Decoder {}

pub struct Encoder {
    encoder: *mut opuscelt_sys::OpusCustomEncoder,
    mode: *mut opuscelt_sys::OpusCustomMode,
}

impl Encoder {
    pub fn new() -> Self {
        unsafe {
            let mode = opuscelt_sys::opus_custom_mode_create(22050, 512, std::ptr::null_mut());
            if mode.is_null() {
                panic!("opus_custom_mode_create returns null");
            }
            let encoder = opuscelt_sys::opus_custom_encoder_create(mode, 1, std::ptr::null_mut());
            if encoder.is_null() {
                panic!("opus_custom_encoder_create returns null");
            }

            Self { encoder, mode }
        }
    }

    pub fn encode(&mut self, pcm: &[i16], output: &mut [u8]) -> Result<(), i32> {
        unsafe {
            let ret = opuscelt_sys::opus_custom_encode(
                self.encoder,
                pcm.as_ptr(),
                pcm.len() as _,
                output.as_mut_ptr(),
                output.len() as _,
            );
            if ret < 0 {
                return Err(ret);
            }

            Ok(())
        }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            opuscelt_sys::opus_custom_encoder_destroy(self.encoder);
            opuscelt_sys::opus_custom_mode_destroy(self.mode);
        }
    }
}

unsafe impl Send for Encoder {}
