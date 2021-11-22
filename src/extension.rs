use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Mutex;

use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use tonic::transport::Server;
use tonic::{Request, Response, Status};

static mut RUNTIME: Option<Runtime> = None;
static mut RUNTIME_GUARD: Option<tokio::runtime::EnterGuard<'static>> = None;

const MAXPLAYERS: usize = 64;

mod coder;

type VoiceSenderVec = Vec<mpsc::Sender<Result<RecvVoiceResponse, Status>>>;

lazy_static::lazy_static! {
    static ref SENDVOICEPKTS: Mutex<VecDeque<(i32, Vec<u8>)>> = Mutex::new(VecDeque::new());
    static ref VOICESENDERS: Mutex<VoiceSenderVec> = Mutex::new(Vec::new());
    static ref DECODERS: Vec<Mutex<coder::Decoder>> = {
        let mut vec = Vec::new();
        for _ in 0..MAXPLAYERS {
            vec.push(Mutex::new(coder::Decoder::new()));
        }

        vec
    };
    static ref ENCODERS: Vec<Mutex<coder::Encoder>> = {
        let mut vec = Vec::new();
        for _ in 0..MAXPLAYERS {
            vec.push(Mutex::new(coder::Encoder::new()));
        }

        vec
    };
}

use voiceserver::voice_service_server::{VoiceService, VoiceServiceServer};
use voiceserver::{RecvVoiceRequest, RecvVoiceResponse, SendVoiceRequest, SendVoiceResponse};
pub mod voiceserver {
    tonic::include_proto!("voiceserver");
}

#[derive(Default)]
pub struct VoiceServiceImpl {}

#[tonic::async_trait]
impl VoiceService for VoiceServiceImpl {
    async fn send_voice_data(
        &self,
        request: Request<tonic::Streaming<SendVoiceRequest>>,
    ) -> Result<Response<SendVoiceResponse>, Status> {
        let mut stream = request.into_inner();
        let mut encoder = coder::Encoder::new();

        while let Some(req) = stream.next().await {
            let req = req?;
            if req.audio_data.is_empty() {
                continue;
            }

            let mut input = Vec::new();
            for chunk in req.audio_data.as_slice().chunks(2) {
                let mut v: [u8; 2] = Default::default();
                v.copy_from_slice(chunk);
                input.push(i16::from_le_bytes(v));
            }

            let mut frames = input.len() / 512;
            if input.len() % 512 != 0 {
                frames += 1;
                input.resize(frames * 512, 0);
            }

            let mut data = vec![0; frames * 64];
            let data_iter = data.as_mut_slice().chunks_mut(64);
            for (input, data) in input.as_slice().chunks(512).zip(data_iter) {
                match encoder.encode(input, data) {
                    Ok(_) => {}
                    Err(err) => {
                        ffi::log_error(&format!("encode error: {}", err));
                        continue;
                    }
                }
            }

            let mut pending = SENDVOICEPKTS.lock().unwrap();
            pending.push_back((req.client_index, data));
        }

        Ok(Response::new(SendVoiceResponse::default()))
    }

    type RecvVoiceDataStream = ReceiverStream<Result<RecvVoiceResponse, Status>>;

    async fn recv_voice_data(
        &self,
        _request: Request<RecvVoiceRequest>,
    ) -> Result<Response<Self::RecvVoiceDataStream>, Status> {
        let (tx, rx) = mpsc::channel(10);

        let mut senders = VOICESENDERS.lock().unwrap();
        senders.push(tx);

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

pub fn init(addr: &str) {
    std::panic::set_hook(Box::new(|panic| {
        let panic = format!("{}", panic);
        ffi::log_error(&panic);
    }));

    let addr = match addr.parse() {
        Ok(addr) => addr,
        Err(err) => {
            ffi::log_error(&format!("listen address parse error: {}", err));
            return;
        }
    };

    let rt = Builder::new_multi_thread()
        .enable_all()
        .thread_name("voiceserver-ext-pool")
        .build()
        .unwrap();
    unsafe {
        RUNTIME.replace(rt);
        RUNTIME_GUARD.replace(RUNTIME.as_ref().unwrap().enter());
    }

    tokio::spawn(main(addr));
}

pub async fn main(addr: SocketAddr) {
    let vsimpl = VoiceServiceImpl {};
    let svc = VoiceServiceServer::new(vsimpl);
    if let Err(err) = Server::builder().add_service(svc).serve(addr).await {
        ffi::log_error(&format!("{}", err));
    }
}

pub fn shutdown() {
    unsafe {
        drop(RUNTIME_GUARD.take());
        drop(RUNTIME.take());
    }
}

pub fn on_gameframe() {
    {
        let mut pending = SENDVOICEPKTS.lock().unwrap();
        while let Some((client_index, data)) = pending.pop_front() {
            ffi::send_client_voice(client_index, &data);
        }
    }

    {
        let mut senders = VOICESENDERS.lock().unwrap();
        let mut i = 0;
        while i < senders.len() {
            if senders[i].is_closed() {
                senders.remove(i);
                continue;
            }
            i += 1;
        }
    }
}

pub fn on_recv_voicedata(idx: usize, volume: f32, steamid: u64, audio_data: &[u8]) -> Vec<u8> {
    if audio_data.is_empty() {
        return audio_data.to_vec();
    }

    let frames = audio_data.len() / 64;

    let (data, mut input) = {
        if idx >= DECODERS.len() {
            return audio_data.to_vec();
        }
        let mut decoder = DECODERS[idx].lock().unwrap();

        let mut input = vec![0; 512 * frames];
        let input_iter = input.as_mut_slice().chunks_mut(512);

        for (data, input) in audio_data.chunks(64).zip(input_iter) {
            match decoder.decode(data, input) {
                Ok(_) => {}
                Err(err) => {
                    ffi::log_error(&format!("decode error: {}", err));
                    continue;
                }
            };
        }

        let mut data = vec![0; input.len() * 2];
        let input_iter = input.as_slice().iter();
        for (data, input) in data.as_mut_slice().chunks_mut(2).zip(input_iter) {
            data.copy_from_slice(&input.to_le_bytes());
        }

        (data, input)
    };

    for i in input.iter_mut() {
        *i = (*i as f64 * volume as f64) as i16
    }

    let ret = {
        if idx >= DECODERS.len() {
            return audio_data.to_vec();
        }
        let mut encoder = ENCODERS[idx].lock().unwrap();

        let mut ret = vec![0; frames * 64];
        let ret_iter = ret.as_mut_slice().chunks_mut(64);
        for (input, ret) in input.as_slice().chunks(512).zip(ret_iter) {
            match encoder.encode(input, ret) {
                Ok(_) => {}
                Err(err) => {
                    ffi::log_error(&format!("re-encode error: {}", err));
                    continue;
                }
            }
        }

        ret
    };

    let mut senders = VOICESENDERS.lock().unwrap();

    let mut i = 0;
    while i < senders.len() {
        if senders[i].is_closed() {
            senders.remove(i);
            continue;
        }

        let resp = RecvVoiceResponse {
            steamid,
            audio_data: data.clone(),
        };
        let _ = senders[i].try_send(Ok(resp));
        i += 1;
    }

    ret
}

#[cxx::bridge(namespace = "ext")]
mod ffi {
    extern "Rust" {
        fn init(addr: &str);
        fn shutdown();
        fn on_gameframe();
        fn on_recv_voicedata(idx: usize, volume: f32, steamid: u64, audio_data: &[u8]) -> Vec<u8>;
    }

    unsafe extern "C++" {
        include!("extension.h");

        fn send_client_voice(client_index: i32, audio_data: &[u8]);
        fn log_error(msg: &str);
    }
}

pub mod ffi_export {
    extern "C" {
        pub fn GetSMExtAPI_Internal() -> *const ();
    }

    #[cfg(feature = "metamod")]
    pub mod metamod {
        extern "C" {
            pub fn CreateInterface_Internal(
                name: *const i8,
                code: *mut std::os::raw::c_int,
            ) -> *const ();
        }
    }
}

pub mod sm {
    #[allow(clippy::missing_safety_doc)]
    #[no_mangle]
    pub unsafe extern "C" fn GetSMExtAPI() -> *const () {
        crate::ffi_export::GetSMExtAPI_Internal()
    }

    #[cfg(feature = "metamod")]
    pub mod metamod {
        #[allow(clippy::missing_safety_doc)]
        #[no_mangle]
        pub unsafe extern "C" fn CreateInterface(
            name: *const i8,
            code: *mut std::os::raw::c_int,
        ) -> *const () {
            crate::ffi_export::metamod::CreateInterface_Internal(name, code)
        }
    }
}
