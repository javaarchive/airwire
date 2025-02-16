use std::{collections::VecDeque, net::UdpSocket, sync::{Arc, Mutex}};

use crate::audio::Codec;
#[cfg(feature = "opus")]
use crate::opus::OpusCodec;

use clap::{Args, Parser, Subcommand};
use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, SupportedStreamConfig, SupportedStreamConfigRange};
use thread_priority::set_current_thread_priority;

pub mod audio;

#[cfg(feature = "opus")]
pub mod opus;

// https://rust-cli-recommendations.sunshowers.io/handling-arguments.html
#[derive(Debug, Parser)]
#[clap(name = "airwire", version, about = "audio over network utility")]
pub struct AirwireConfig {
    #[clap(flatten)]
    global_opts: AudioConfig,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Transmit(TransmitArgs),
    Recieve(RecieveArgs),
    Discover(DiscoverArgs),
    Enumerate(EnumerateArgs),
}

#[derive(Debug, Args, Clone)]
pub struct AudioConfig {
    #[clap(long, global = true, default_value_t = -1, env = "AIRWIRE_BUFFER", help = "buffer size in ms, if negative, use default suggested buffer size")]
    pub buffer: i32,
    #[clap(long, global = true, env = "AIRWIRE_ADDR", help = "ip:port to bind or connect to")]
    pub addr: Option<String>,
    #[clap(long, global = true, env = "AIRWIRE_DEFAULT_DEVICE_NAME", help = "name of the device to use, find names with the enumerate subcommand")]
    pub target_device_name: Option<String>,
    #[clap(long, global = true, default_value_t = 48000, env = "AIRWIRE_SAMPLE_RATE")]
    pub sample_rate: u32,
    #[clap(long, global = true, default_value_t = 480, help = "frame size as fraction of the sample rate")]
    pub frame_size: u32,
    #[clap(long, global = true, default_value_t = 2, env = "AIRWIRE_CHANNELS")]
    pub channels: u16,
    #[clap(long, global = true, default_value_t = Codec::None, env = "AIRWIRE_CODEC")]
    pub codec: Codec,
    #[clap(long, global = true, default_value_t = false, help = "try to set threads as high priority, cur only works with recieve and may require additional perms like on linux")]
    pub priority: bool,
    #[clap(long, global = true, default_value_t = false, help = "swap left and right channel, useful for some devices where order is not correct")]
    pub stereo_swap: bool,
    #[clap(short, long, global = true, default_value_t = 10, help = "quality of codec, defaults to 10 which is best for opus")]
    pub quality: u32,
    #[clap(short, long, global = true, default_value_t = { "audio".to_string() }, help = "profile/application preset to pass to codec, defaults to audio")]
    pub profile: String,
    #[clap(short, long, global = true, default_value_t = 128, help = "bitrate in kbps, defaults to 128kbps which is good for opus, negative or 0 value will omit")]
    pub bitrate: i32,
    #[clap(long, global = true, default_value_t = false, help = "enable forward error correction for opus codec")]
    pub fec: bool,
    #[clap(long, global = true, default_value_t = false, help = "enable variable bitrate for codecs that supported it")]
    pub vbr: bool,
    #[clap(long, global = true, default_value_t = false, help = "enable debug logging")]
    pub debug: bool,
}

impl AudioConfig {
    pub fn construct_encoder(&self) -> Box<dyn audio::Encoder> {
        let encoder: Box<dyn audio::Encoder> = match self.codec {
            Codec::None => Box::new(audio::PCMCodec::new(self)),
            Codec::Opus => {
                #[cfg(not(feature = "opus"))]
                panic!("Opus codec is not enabled, enable it with --features opus when compiling");
                #[cfg(feature = "opus")]
                Box::new(OpusCodec::new(self))
            },
        };
        encoder
    }

    pub fn construct_decoder(&self) -> Box<dyn audio::Decoder> {
        let decoder: Box<dyn audio::Decoder> = match self.codec {
            Codec::None => Box::new(audio::PCMCodec::new(self)),
            Codec::Opus => {
                #[cfg(not(feature = "opus"))]
                panic!("Opus codec is not enabled, enable it with --features opus when compiling");
                #[cfg(feature = "opus")]
                Box::new(OpusCodec::new(self))
            },
        };
        decoder
    }

    pub fn get_input_device(&self, host: &cpal::Host) -> Option<cpal::Device> {
        if let Some(ref device_name) = self.target_device_name {
            for device in host.input_devices().expect("Failed to get input devices") {
                if &device.name().unwrap_or_else(|_| "unknown device name".to_string()) == device_name {
                    return Some(device);
                }
            }
            return None;
        } else {
            return host.default_input_device();
        }
    }

    pub fn get_output_device(&self, host: &cpal::Host) -> Option<cpal::Device> {
        if let Some(ref device_name) = self.target_device_name {
            for device in host.output_devices().expect("Failed to get output devices") {
                if &device.name().unwrap_or_else(|_| "unknown device name".to_string()) == device_name {
                    return Some(device);
                }
            }
            return None;
        } else {
            host.default_output_device()
        }
    }

    pub fn get_stream_config(&self) -> cpal::StreamConfig {
        cpal::StreamConfig {
            channels: self.channels,
            sample_rate: cpal::SampleRate(self.sample_rate),
            buffer_size: match self.buffer <= 0 {
                true => cpal::BufferSize::Default,
                false => cpal::BufferSize::Fixed(self.buffer as u32),
            },
        }
    }
}

#[derive(Debug, Args)]
struct TransmitArgs {
}

#[derive(Debug, Args)]
struct RecieveArgs {
}

#[derive(Debug, Args)]
struct DiscoverArgs {
}

#[derive(Debug, Args)]
struct EnumerateArgs {
}

pub fn block_main_thread() {
    loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}

pub const SIGNATURE_SIZE: usize = 2;

pub fn add_signature(buffer: &mut Vec<u8>) {
    buffer.push(13);
    buffer.push(37);
}

fn describe_stream_config(stream_config: &SupportedStreamConfigRange) -> String {
    let sample_rate_max = stream_config.max_sample_rate();
    let sample_rate_max_number = sample_rate_max.0;
    let sample_rate_min = stream_config.min_sample_rate();
    let sample_rate_min_number = sample_rate_min.0;
    let channels = stream_config.channels();
    let buffer_size = stream_config.buffer_size();
    let mut buffer_size_str = "unknown".to_string();
    if let cpal::SupportedBufferSize::Range { min, max } = buffer_size {
        if *min == 0 && *max > 1000000 {
            // at this rate it's not informative
            buffer_size_str = "limitless".to_string();
        } else {
            buffer_size_str = format!("{}ms-{}ms", min, max);
        }
    }
    format!("{}-{}hz {} channels {}", sample_rate_min_number, sample_rate_max_number, channels, buffer_size_str)
}

fn main() {
    let airwire_config = AirwireConfig::parse();
    let calculate_max_buffer_frames = || ((airwire_config.global_opts.sample_rate as usize) * (airwire_config.global_opts.frame_size as usize)) / (1000 * airwire_config.global_opts.frame_size as usize); 
    let calculate_packet_size = || ((airwire_config.global_opts.frame_size as usize) * (airwire_config.global_opts.channels as usize) * 2);
    let calculate_real_frame_size = || ((airwire_config.global_opts.frame_size as usize) * (airwire_config.global_opts.channels as usize) * 2);
    let calculate_sample_frame_size = || ((airwire_config.global_opts.frame_size as usize) * (airwire_config.global_opts.channels as usize));

    let high_priority = airwire_config.global_opts.priority;

    // networking is hardcoded for now
    match airwire_config.command {
        Command::Transmit(args) => {
            let host = cpal::default_host();
            let mut encoder = airwire_config.global_opts.construct_encoder();
            let input_device = airwire_config.global_opts.get_input_device(&host).expect("No input device found");
            let max_buffer_frames = calculate_max_buffer_frames();
            let sample_frame_size = calculate_sample_frame_size();
            let packet_size = calculate_packet_size();
            let stereo_swap = airwire_config.global_opts.stereo_swap;

            if stereo_swap {
                println!("Stereo swap enabled on transmit side, performance may be only slightly reduced. ");
            }

            let cpal_config = airwire_config.global_opts.get_stream_config();

            let socket = UdpSocket::bind("0.0.0.0:0").expect("getting a udp socket failed");
            socket.connect(airwire_config.global_opts.addr.clone().expect("Give me an address to connect to")).expect("Connection failed to server");

            let socket_arc = Arc::new(socket);

            let mut input_buffer = vec![0.0f32; sample_frame_size as usize];
            let mut packet_buffer = Vec::with_capacity((packet_size + SIGNATURE_SIZE) as usize);
            let mut encoded_data_buffer = vec![0; (packet_size) as usize];
            let mut buffer_pos = 0;
            add_signature(&mut packet_buffer);

            let input_stream = input_device.build_input_stream(
                &cpal_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let incoming_len = data.len();
                    let will_encode = buffer_pos + incoming_len >= (sample_frame_size as usize);
                    for &sample in data.iter() {

                        if buffer_pos < sample_frame_size as usize {
                            // println!("sample {}", sample);
                            // stereo swap hack
                            let buffer_pos_internal = match stereo_swap {
                                false => buffer_pos,
                                true => match buffer_pos % 2 {
                                    0 => buffer_pos + 1, // 0 to 1
                                    _ => buffer_pos - 1, // 1 to 0
                                },
                            };
                            input_buffer[buffer_pos_internal] = sample;
                            buffer_pos += 1;
                        }
                        if buffer_pos >= sample_frame_size as usize {
                            encoded_data_buffer.resize(packet_size as usize, 0);
                            if let Err(err) = encoder.encode(&input_buffer, &mut encoded_data_buffer) {
                                println!("Error encoding data: {:?}", err);
                            } else {
                                // println!("send {} bytes (input {})", packet_buffer.len(),input_buffer.len());
                                packet_buffer.extend_from_slice(&encoded_data_buffer);
                                // println!("sent {} bytes", packet_buffer.len());
                                socket_arc.send(&packet_buffer).expect("Error sending data");
                                /*print!("sent a ");
                                for i in 450..500 {
                                    print!("{:02x} ", packet_buffer[i]);
                                }
                                println!("");*/
                                packet_buffer.clear();
                                add_signature(&mut packet_buffer);
                            }
                            buffer_pos = 0;
                        }
                    }
                },
                move |err | {
                    println!("input error: {:?}", err);
                },
                None
            ).expect("input stream creation failed");

            println!("starting input capture");
            input_stream.play().expect("Failed to play stream");

            block_main_thread();
        },
        Command::Recieve(args) => {
            let host = cpal::default_host();
            let output_device = airwire_config.global_opts.get_output_device(&host).expect("No output device found");
            let bind_str = airwire_config.global_opts.addr.clone().unwrap_or_else(|| "0.0.0.0:0".to_string());
            println!("Binding to {}", bind_str);
            let socket = UdpSocket::bind(bind_str).expect("Failed to bind socket");
            let max_buffer_frames = calculate_max_buffer_frames();
            let packet_size = calculate_packet_size();
            let real_frame_size = calculate_real_frame_size();
            let sample_frame_size = calculate_sample_frame_size();
            let should_configure_buffer = airwire_config.global_opts.buffer <= 0;
            let buffer_ms = airwire_config.global_opts.buffer as u32;
            let sample_rate = airwire_config.global_opts.sample_rate as u32;
            let channels = airwire_config.global_opts.channels as u16;
            let stereo_swap = airwire_config.global_opts.stereo_swap;

            if stereo_swap {
                println!("Stereo swap enabled on recv side, may reduce performance a lot.");
            }
            
            // struct idea from claude
            let audio_buffer: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::with_capacity(
                max_buffer_frames * (airwire_config.global_opts.frame_size as usize) * (airwire_config.global_opts.channels as usize)
            )));

            let socket_arc = Arc::new(socket);
            let audio_buffer_clone = audio_buffer.clone();

            let cpal_config = airwire_config.global_opts.get_stream_config();

            std::thread::Builder::new().name("networking".to_string()).spawn(move || {
                
                println!("begin recieve thread {}",packet_size + SIGNATURE_SIZE);
                let mut decoder = airwire_config.global_opts.construct_decoder();
                let mut receive_buffer = vec![0u8; packet_size + SIGNATURE_SIZE];
                let mut decode_buffer: Vec<f32> = vec![0.0; sample_frame_size];

                if high_priority {
                    match set_current_thread_priority(thread_priority::ThreadPriority::Max) {
                        Ok(_) => {
                            println!("Set thread priority to max");
                        },
                        Err(err) => {
                            println!("Failed to set thread priority {}", err);
                        },
                    }
                }

                loop {
                    match socket_arc.recv(&mut receive_buffer) {
                        Ok(recv_bytes) => {
                            // xd: in case some random network device sends random garbage at us we can detect it
                            if receive_buffer[0] == 13 && receive_buffer[1] == 37 {
                                // println!("recv {} bytes", recv_bytes);
                                match decoder.decode(&receive_buffer[SIGNATURE_SIZE..recv_bytes], &mut decode_buffer) {
                                    Ok(_) => {
                                        // thanks to rust being too safe we have a copy here
                                        {
                                            let mut audio_buffer = audio_buffer_clone.lock().unwrap();
                                            // println!("decode {} bytes {}", decode_buffer.len(), decode_buffer[70]);
                                            if stereo_swap {
                                                // TODO: optimize this
                                                for i in 0..decode_buffer.len() / 2 {
                                                    audio_buffer.push_back(decode_buffer[i * 2 + 1]);
                                                    audio_buffer.push_back(decode_buffer[i * 2]);
                                                }
                                            } else{
                                                audio_buffer.extend(decode_buffer.iter());
                                            }
                                        }
                                        // decode_buffer.clear();
                                    },
                                    Err(err) => {
                                        println!("Error decoding data so skipped: {:?}", err);
                                    }
                                }
                            } else {
                                println!("signature check failed? is something sending trash udp packets?");
                            }
                        },
                        Err(err) => {
                            println!("Error receiving data: {:?} {:?} ???", err, err.kind());
                        },
                    }
                }
            }).expect("recieve thread setup failed");

            let audio_buffer_clone_2 = audio_buffer.clone();
            let output_stream = output_device.build_output_stream(
                &cpal_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut audio_buffer = audio_buffer_clone_2.lock().unwrap();
                    let mut filled = 0;
                    for sample in data.iter_mut() {
                        if let Some(buffered_sample) = audio_buffer.pop_front() {
                            *sample = buffered_sample;
                            filled += 1;
                        } else {
                            *sample = 0.0; // silent
                        }
                    }
                    // println!("filled {}/{} {}", filled, data.len(), data[1]);
                    // claude suggested this logging thing
                    if data.len() > 0 && audio_buffer.len() % (sample_rate as usize) == 0 {
                        let buffer_ms = audio_buffer.len() * 1000 / (sample_rate as usize * channels as usize);
                        // println!("Buffer status: {}ms filled {}/{}", buffer_ms, filled, data.len());
                    }
                },
                move |err| {
                    println!("output error: {:?}", err);
                },
                None
            ).expect("output stream creation failed");
            println!("starting playback");
            output_stream.play().expect("Failed to play stream");

            block_main_thread();
        },
        Command::Discover(args) => {
            todo!("discover targets");
        },
        Command::Enumerate(args) => {
            let host = cpal::default_host();
            println!("Output devices:");
            for device in host.output_devices().expect("Failed to get output devices") {
                let mut output_configs_str = "<error>".to_string();
                if let Ok(supported_output_configs) = device.supported_output_configs() {
                    output_configs_str = "".to_string();
                    for config in supported_output_configs {
                        output_configs_str += &format!("{:?}, ", describe_stream_config(&config));
                    }
                }
                println!("{}: {}", device.name().unwrap_or_else(|_| "unknown device name".to_string()), output_configs_str);
            }
            println!("Input devices:");
            for device in host.input_devices().expect("Failed to get input devices") {
                let mut input_configs_str = "<error>".to_string();
                if let Ok(supported_input_configs) = device.supported_input_configs() {
                    input_configs_str = "".to_string();
                    for config in supported_input_configs {
                        input_configs_str += &format!("{:?}, ", describe_stream_config(&config));
                    }
                }
                println!("{}: {}", device.name().unwrap_or_else(|_| "unknown device name".to_string()), input_configs_str);
            }
        },
    }
}
