use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use tonic::transport::Server;
use tonic::{Request, Response, Status};

static mut RUNTIME: Option<Runtime> = None;
static mut RUNTIME_THREAD: Option<thread::JoinHandle<()>> = None;
static mut SHUTDOWN: Option<oneshot::Sender<()>> = None;

type VoiceSenderVec = Vec<mpsc::Sender<Result<RecvVoiceResponse, Status>>>;

lazy_static::lazy_static! {
    static ref SENDVOICEPKTS: Mutex<VecDeque<(i32, Vec<u8>)>> = Mutex::new(VecDeque::new());
    static ref VOICESENDERS: Mutex<VoiceSenderVec> = Mutex::new(Vec::new());
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

        while let Some(req) = stream.next().await {
            let req = req?;

            {
                let mut pending = SENDVOICEPKTS.lock().unwrap();
                pending.push_back((req.client_index, req.audio_data.clone()));
            }
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

    let rt = Builder::new_current_thread().enable_all().build().unwrap();
    unsafe {
        RUNTIME.replace(rt);
    }

    let addr = addr.parse().unwrap();

    let hndl = thread::spawn(move || unsafe {
        RUNTIME.as_ref().unwrap().block_on(async move {
            main(addr).await;
        });
    });
    unsafe {
        RUNTIME_THREAD.replace(hndl);
    }
}

pub async fn main(addr: SocketAddr) {
    let (tx, rx) = oneshot::channel();
    unsafe {
        SHUTDOWN.replace(tx);
    }

    let vsimpl = VoiceServiceImpl {};
    let svc = VoiceServiceServer::new(vsimpl);
    tokio::select! {
        _ = Server::builder().add_service(svc).serve(addr) => {},
        _ = rx => {}
    };
}

pub fn shutdown() {
    let tx = unsafe { SHUTDOWN.take().unwrap() };
    let _ = tx.send(());

    let hndl = unsafe { RUNTIME_THREAD.take().unwrap() };
    hndl.join().unwrap();

    let runtime = unsafe { RUNTIME.take().unwrap() };
    runtime.shutdown_timeout(Duration::from_millis(100));
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

pub fn on_recv_voicedata(steamid: u64, audio_data: &[u8]) {
    let audio_data = audio_data.to_vec();
    let mut senders = VOICESENDERS.lock().unwrap();

    let mut i = 0;
    while i < senders.len() {
        if senders[i].is_closed() {
            senders.remove(i);
            continue;
        }

        let resp = RecvVoiceResponse {
            steamid,
            audio_data: audio_data.clone(),
        };
        let _ = senders[i].try_send(Ok(resp));
        i += 1;
    }
}

#[cxx::bridge(namespace = "ext")]
mod ffi {
    extern "Rust" {
        fn init(addr: &str);
        fn shutdown();
        fn on_gameframe();
        fn on_recv_voicedata(steamid: u64, audio_data: &[u8]);
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
