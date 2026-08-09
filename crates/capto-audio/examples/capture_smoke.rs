use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

fn main() {
    let devices = capto_audio::list_devices().expect("list WASAPI devices");
    let endpoint = devices
        .iter()
        .find(|device| device.kind == capto_audio::AudioDeviceKind::Loopback && device.is_default)
        .or_else(|| {
            devices
                .iter()
                .find(|device| device.kind == capto_audio::AudioDeviceKind::Input)
        })
        .expect("a loopback or microphone endpoint");
    println!("capturing: {} ({:?})", endpoint.name, endpoint.kind);

    let (mic, loopback) = match endpoint.kind {
        capto_audio::AudioDeviceKind::Input => (Some(endpoint.id.as_str()), None),
        capto_audio::AudioDeviceKind::Loopback => (None, Some(endpoint.id.as_str())),
        capto_audio::AudioDeviceKind::Output => unreachable!(),
    };
    let mut session = capto_audio::NativeAudioSession::prepare(mic, loopback)
        .expect("prepare native audio")
        .expect("one audio source");
    let url = session.inputs()[0]
        .url
        .strip_prefix("tcp://")
        .expect("tcp URL")
        .to_string();
    session.start().expect("start WASAPI capture");

    let mut stream = TcpStream::connect(url).expect("connect PCM consumer");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut pcm = vec![0u8; 48_000 * 2 * 4];
    stream.read_exact(&mut pcm).expect("one second of PCM");
    session.stop();

    let nonzero = pcm.iter().filter(|byte| **byte != 0).count();
    println!(
        "received {} bytes, {} non-zero ({:.1}%)",
        pcm.len(),
        nonzero,
        nonzero as f64 * 100.0 / pcm.len() as f64
    );
}
