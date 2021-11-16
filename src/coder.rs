pub struct Decoder {
    decoder: *mut celt_rs::CELTDecoder,
    mode: *mut celt_rs::CELTMode,
}

impl Decoder {
    pub fn new() -> Self {
        unsafe {
            let mode = celt_rs::celt_mode_create(22050, 512, std::ptr::null_mut());
            if mode.is_null() {
                panic!("celt_mode_create returns null");
            }
            let decoder = celt_rs::celt_decoder_create_custom(mode, 1, std::ptr::null_mut());
            if decoder.is_null() {
                panic!("celt_decoder_create_custom returns null");
            }

            Self { decoder, mode }
        }
    }

    pub fn decode(&mut self, data: &[u8], output: &mut [i16]) -> Result<(), i32> {
        unsafe {
            let ret = celt_rs::celt_decode(
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
            celt_rs::celt_decoder_destroy(self.decoder);
            celt_rs::celt_mode_destroy(self.mode);
        }
    }
}

unsafe impl Send for Decoder {}

pub struct Encoder {
    encoder: *mut celt_rs::CELTEncoder,
    mode: *mut celt_rs::CELTMode,
}

impl Encoder {
    pub fn new() -> Self {
        unsafe {
            let mode = celt_rs::celt_mode_create(22050, 512, std::ptr::null_mut());
            if mode.is_null() {
                panic!("celt_mode_create returns null");
            }
            let encoder = celt_rs::celt_encoder_create_custom(mode, 1, std::ptr::null_mut());
            if encoder.is_null() {
                panic!("celt_encoder_create_custom returns null");
            }

            Self { encoder, mode }
        }
    }

    pub fn encode(&mut self, pcm: &[i16], output: &mut [u8]) -> Result<(), i32> {
        unsafe {
            let ret = celt_rs::celt_encode(
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
            celt_rs::celt_encoder_destroy(self.encoder);
            celt_rs::celt_mode_destroy(self.mode);
        }
    }
}

unsafe impl Send for Encoder {}
